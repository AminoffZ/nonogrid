//! The algorithm based on the ideas
//! from the [article](https://habr.com/ru/post/433330/)
use std::{
    collections::HashMap,
    iter::{from_fn, once},
    marker::PhantomData,
    ops::Range,
};

use varisat::{solver::Solver, CnfFormula, ExtendFormula, Lit, Var};

use crate::{
    block::{base::color::ColorId, Block, Color, Description},
    board::Point,
    solver::probing::Impact,
    utils::{is_overlapping, is_touching, pair_combinations, product, rc::ReadRc},
};

trait BlockPosition {
    fn block_starts(&self) -> Vec<usize>;
    fn min_space(&self) -> usize;
    fn positions_number(&self, line_length: usize) -> usize {
        let min_space = self.min_space();
        assert!(line_length >= min_space);
        line_length - min_space + 1
    }
}

impl<B> BlockPosition for Description<B>
where
    B: Block,
{
    fn block_starts(&self) -> Vec<usize> {
        self.vec
            .iter()
            .zip(Block::partial_sums(&self.vec))
            .map(|(block, end)| end - block.size())
            .collect()
    }

    fn min_space(&self) -> usize {
        if self.vec.is_empty() {
            return 0;
        }
        *Block::partial_sums(&self.vec)
            .last()
            .expect("Partial sums should be non-empty")
    }
}

#[derive(Debug, Clone)]
struct Position {
    var: Var,
    range: Range<usize>,
}

#[derive(Debug, Clone)]
struct BlockPositions {
    index: usize,
    color: ColorId,
    vec: Vec<Position>,
}

#[derive(Debug)]
struct LinePositions(Vec<BlockPositions>);

#[derive(Debug)]
pub struct ClauseGenerator<B>
where
    B: Block,
{
    columns_vars: Vec<LinePositions>,
    rows_vars: Vec<LinePositions>,
    cell_vars: Vec<Vec<HashMap<ColorId, Var>>>,
    cells: Vec<B::Color>,
    width: usize,
    height: usize,
    formula: CnfFormula,
    _phantom: PhantomData<B>,
}

impl<B> ClauseGenerator<B>
where
    B: Block,
{
    const BLACK_COLOR: u32 = 0;

    pub fn with_clues(
        columns: &[ReadRc<Description<B>>],
        rows: &[ReadRc<Description<B>>],
        cells: Vec<B::Color>,
    ) -> Self {
        let mut block_colors: Vec<_> = rows.iter().flat_map(|row| row.colors()).collect();
        if block_colors.is_empty() {
            block_colors = vec![Self::BLACK_COLOR];
        }

        let width = columns.len();
        let height = rows.len();

        let mut formula = CnfFormula::new();
        let columns_vars = Self::clues_vars(columns, height, &mut formula);
        let rows_vars = Self::clues_vars(rows, width, &mut formula);
        let clues_vars = formula.var_count();

        let cell_vars = cells
            .chunks(width)
            .map(|row| {
                row.iter()
                    .map(|_cell| {
                        block_colors
                            .iter()
                            .map(|&color_id| (color_id, formula.new_var()))
                            .collect()
                    })
                    .collect()
            })
            .collect();

        let vars_total = formula.var_count();
        warn!(
            "Vars: {} (clues: {}, cells: {})",
            vars_total,
            clues_vars,
            vars_total - clues_vars
        );
        Self {
            columns_vars,
            rows_vars,
            cell_vars,
            cells,
            width,
            height,
            formula,
            _phantom: PhantomData,
        }
    }

    fn clues_vars_count(&self) -> usize {
        let col_vars: usize = self
            .columns_vars
            .iter()
            .flat_map(|col| col.0.iter().map(|block| block.vec.len()))
            .sum();

        let row_vars: usize = self
            .rows_vars
            .iter()
            .flat_map(|col| col.0.iter().map(|block| block.vec.len()))
            .sum();

        col_vars + row_vars
    }

    fn clues_vars(
        clues: &[ReadRc<Description<B>>],
        line_length: usize,
        formula: &mut CnfFormula,
    ) -> Vec<LinePositions> {
        clues
            .iter()
            .map(|clue| {
                let positions = clue.positions_number(line_length);
                LinePositions(
                    clue.block_starts()
                        .iter()
                        .zip(&clue.vec)
                        .enumerate()
                        .map(|(index, (&start, block))| BlockPositions {
                            index,
                            color: Self::get_id(block.color()).expect("Block without color!"),
                            vec: formula
                                .new_var_iter(positions)
                                .zip(start..)
                                .map(|(var, start)| Position {
                                    var,
                                    range: start..start + block.size(),
                                })
                                .collect(),
                        })
                        .collect(),
                )
            })
            .collect()
    }

    fn block_positions_clause(positions: &BlockPositions) -> Vec<Lit> {
        // 1. Каждый блок, объявленный в строке или столбце обязан появиться хотя-бы в одной позиции.
        // Этому соответствует клоз вида (X1 V X2 V… XN),
        // где X1, X2… XN — все возможные позиции данного блока в строке или столбце.
        positions.vec.iter().map(|pos| pos.var.positive()).collect()
    }

    fn block_once_clauses(positions: &BlockPositions) -> impl Iterator<Item = Vec<Lit>> {
        // 2. Каждый блок в строке или столбце должен появиться не более одного раза.
        // Этому соответствует множество клозов вида (not Xi) V (not Xj),
        // где Xi, Xj (i != j) — все возможные позиции данного блока в строке или столбце.
        let pairs = pair_combinations(&positions.vec);
        pairs
            .into_iter()
            .map(|(f, s)| vec![f.var.negative(), s.var.negative()])
    }

    fn non_overlap_clauses(positions: &LinePositions) -> impl Iterator<Item = Vec<Lit>> {
        // 3. Правильный порядок блоков. Поскольку необходимо поддерживать правильный порядок расположения блоков,
        // а также исключить их пересечение, необходимо добавить клозы, вида (not Xi) V (not Xj),
        // где Xi, Xj — переменные, соответствующие позициям разных блоков, которые имеют неправильный порядок или пересекаются.
        let pairs = pair_combinations(&positions.0);
        pairs.into_iter().flat_map(|(block1, block2)| {
            let pairs = product(&block1.vec, &block2.vec);
            pairs.into_iter().filter_map(
                move |(
                    Position {
                        range: range1,
                        var: var1,
                    },
                    Position {
                        range: range2,
                        var: var2,
                    },
                )| {
                    let conflict = if block1.color == block2.color {
                        is_touching(range1.clone(), range2.clone())
                    } else {
                        is_overlapping(range1.clone(), range2.clone())
                    };

                    if conflict || ((block1.index < block2.index) && (range1.start > range2.start))
                    {
                        Some(vec![var1.negative(), var2.negative()])
                    } else {
                        None
                    }
                },
            )
        })
    }

    fn covering_positions(&self, cell_point: Point, color_id: ColorId) -> (Vec<Var>, Vec<Var>) {
        let (x, y) = (cell_point.x(), cell_point.y());
        let column = &self.columns_vars[x];
        let row = &self.rows_vars[y];

        let column_vars = column
            .0
            .iter()
            .filter(|block_pos| block_pos.color == color_id)
            .flat_map(move |block_pos| {
                block_pos.vec.iter().filter_map(move |pos| {
                    if pos.range.contains(&y) {
                        Some(pos.var)
                    } else {
                        None
                    }
                })
            })
            .collect();
        let row_vars = row
            .0
            .iter()
            .filter(|block_pos| block_pos.color == color_id)
            .flat_map(move |block_pos| {
                block_pos.vec.iter().filter_map(move |pos| {
                    if pos.range.contains(&x) {
                        Some(pos.var)
                    } else {
                        None
                    }
                })
            })
            .collect();

        (column_vars, row_vars)
    }

    fn cell_color_clauses(&self, cell_point: Point) -> Vec<Vec<Lit>> {
        // 4. Окрашенная клетка должна содержаться внутри хотя бы одного блока, позиция которого включает данную клетку.
        // Этому соответствует клоз вида ((not Yk) V X1 V X2… XN), где Yk — переменная, соответствующая клетке,
        // а X1, X2… XN — переменные, соответствующие позициям блоков, содержащих данную клетку.
        let point_vars = self.get_vars(cell_point);

        point_vars
            .iter()
            .flat_map(|(&color_id, point_var)| {
                let (column_vars, row_vars) = self.covering_positions(cell_point, color_id);

                let column_clause = column_vars
                    .into_iter()
                    .map(|var| var.positive())
                    .chain(once(point_var.negative()))
                    .collect();

                let row_clause = row_vars
                    .into_iter()
                    .map(|var| var.positive())
                    .chain(once(point_var.negative()))
                    .collect();

                once(column_clause).chain(once(row_clause))
            })
            .collect()
    }

    fn cell_space_clauses(&self, cell_point: Point) -> Vec<Vec<Lit>> {
        // 5. Каждая пустая клетка не должна содержаться ни в одной возможной позиции ни одного блока.
        // Этому соответствует множество клозов вида Yi V (not Xj), где Yi — переменная, соответствующая клетке,
        // а Xj — переменная, соответствующая одной позиции какого-либо блока, содержащая данную клетку.
        let point_vars = self.get_vars(cell_point);
        point_vars
            .iter()
            .flat_map(|(&color_id, point_var)| {
                let (column_vars, row_vars) = self.covering_positions(cell_point, color_id);

                column_vars
                    .into_iter()
                    .chain(row_vars)
                    .map(move |var| vec![var.negative(), point_var.positive()])
            })
            .collect()
    }

    fn get_id(color: B::Color) -> Option<ColorId> {
        if color == Color::blank() {
            return None;
        }
        color.as_color_id().or(Some(Self::BLACK_COLOR))
    }

    fn point_once_clauses(&self, cell_point: Point) -> Vec<Vec<Lit>> {
        let point_vars = self.get_vars(cell_point);

        let values: Vec<_> = point_vars.values().collect();
        let pairs = pair_combinations(&values);
        pairs
            .into_iter()
            .map(|(f, s)| vec![f.negative(), s.negative()])
            .collect()
    }

    fn precomputed_cells_clauses(&self) -> Vec<Lit> {
        if self.cells.is_empty() {
            return Vec::new();
        }

        self.cell_vars
            .concat()
            .iter()
            .zip(&self.cells)
            .flat_map(|(vars, &cell)| {
                if cell.is_solved() {
                    let color_id = Self::get_id(cell);
                    if let Some(color_id) = color_id {
                        let var = vars.get(&color_id).expect("Solved color should be present");
                        vec![var.positive()]
                    } else {
                        // blank cell
                        vars.values().map(|var| var.negative()).collect()
                    }
                } else {
                    let colors: Vec<_> = cell
                        .variants()
                        .into_iter()
                        .filter_map(Self::get_id)
                        .collect();
                    vars.iter()
                        .filter_map(|(color, var)| {
                            if !colors.contains(color) {
                                Some(var.negative())
                            } else {
                                None
                            }
                        })
                        .collect()
                }
            })
            .collect()
    }

    fn clauses(&self) -> impl Iterator<Item = Vec<Lit>> + '_ {
        let columns_positions = self
            .columns_vars
            .iter()
            .flat_map(|line_positions| line_positions.0.iter().map(Self::block_positions_clause));
        let rows_positions = self
            .rows_vars
            .iter()
            .flat_map(|line_positions| line_positions.0.iter().map(Self::block_positions_clause));

        let columns_once_positions = self
            .columns_vars
            .iter()
            .flat_map(|line_positions| line_positions.0.iter().flat_map(Self::block_once_clauses));
        let rows_once_positions = self
            .rows_vars
            .iter()
            .flat_map(|line_positions| line_positions.0.iter().flat_map(Self::block_once_clauses));

        let non_overlap_columns = self.columns_vars.iter().flat_map(Self::non_overlap_clauses);
        let non_overlap_rows = self.rows_vars.iter().flat_map(Self::non_overlap_clauses);

        let all_points = product(
            &(0..self.width).collect::<Vec<_>>(),
            &(0..self.height).collect::<Vec<_>>(),
        );
        let box_clauses = all_points
            .clone()
            .into_iter()
            .flat_map(move |(x, y)| self.cell_color_clauses(Point::new(x, y)));
        let space_clauses = all_points
            .clone()
            .into_iter()
            .flat_map(move |(x, y)| self.cell_space_clauses(Point::new(x, y)));

        let point_once_clauses = all_points
            .into_iter()
            .flat_map(move |(x, y)| self.point_once_clauses(Point::new(x, y)));

        let fixed_points = self
            .precomputed_cells_clauses()
            .into_iter()
            .map(|lit| vec![lit]);

        columns_positions
            .chain(rows_positions)
            .chain(columns_once_positions)
            .chain(rows_once_positions)
            .chain(non_overlap_columns)
            .chain(non_overlap_rows)
            .chain(box_clauses)
            .chain(space_clauses)
            .chain(point_once_clauses)
            .chain(fixed_points)
    }

    fn get_formula(&self) -> CnfFormula {
        debug!("{:#?}", self);

        let mut formula = CnfFormula::new();
        for clause in self.clauses() {
            formula.add_clause(&clause);
        }
        formula
    }

    fn get_vars(&self, point: Point) -> &HashMap<ColorId, Var> {
        self.cell_vars
            .get(point.y())
            .and_then(|row| row.get(point.x()))
            .expect("Cannot get vars for point")
    }

    //fn generate_implications(
    //    &self,
    //    probe: Point,
    //    probe_color: B::Color,
    //    impact: Vec<(Point, B::Color)>,
    //) -> impl Iterator<Item = (Vec<Lit>)> + '_ {
    //    // (X -> Y) == (-X \/ Y)
    //    let probe_vars = self.get_colors(probe);
    //
    //    let probe_id = Self::get_id(probe_color);
    //    impact.into_iter().map(move |(point, color)| {
    //        let var = self.get_colors(point);
    //        let impact_id = Self::get_id(color);
    //        vec![probe_vars.lit(is_white_probe), var.lit(!is_white_impact)]
    //    })
    //}

    #[allow(clippy::suspicious_map)]
    pub fn run(
        &self,
        probing_impact: Impact<B>,
        solutions_number: Option<usize>,
    ) -> impl Iterator<Item = Vec<B::Color>> {
        let formula = self.get_formula();
        //let total_impact_clauses: usize = probing_impact
        //    .into_iter()
        //    .map(|impact| {
        //        let (probe, probe_color, impact, _) = impact.into_tuple();
        //        self.generate_implications(probe, probe_color, impact)
        //            .map(|clause| {
        //                formula.add_clause(&clause);
        //            })
        //            .count()
        //    })
        //    .sum();
        //
        //warn!("Add {} impact clauses", total_impact_clauses);
        warn!("Total clauses: {}", formula.len());
        for (i, clause) in formula.iter().enumerate() {
            info!("{}. {:?}", i, clause);
        }

        let block_vars = self.clues_vars_count();
        let cell_vars = self.cell_vars.clone();

        let mut solver = Solver::new();
        solver.add_formula(&formula);

        let mut found = 0;
        from_fn(move || {
            if let Some(solutions_number) = solutions_number {
                if found >= solutions_number {
                    return None;
                }
            }

            let _ = solver.solve().unwrap();
            //assert_eq!(solution, true); // satisfiable
            solver.model().map(|model| {
                found += 1;

                let cells = &model[block_vars..];

                let colored_cells: Vec<_> = cells
                    .iter()
                    .filter_map(|cell| {
                        if cell.is_positive() {
                            Some(cell.var())
                        } else {
                            None
                        }
                    })
                    .collect();

                let fixed_clause: Vec<_> =
                    colored_cells.iter().map(|&var| var.negative()).collect();
                solver.add_clause(&fixed_clause);

                colored_cells
            })
        })
        .map(move |colored_vars| {
            cell_vars
                .iter()
                .flat_map(|row| row.iter())
                .map(|cell_map| {
                    let color_id = cell_map
                        .iter()
                        .filter_map(|(&color_id, var)| {
                            if colored_vars.contains(&var) {
                                Some(color_id)
                            } else {
                                None
                            }
                        })
                        .next();

                    match color_id {
                        None => B::Color::blank(),
                        Some(Self::BLACK_COLOR) => B::Color::from_color_ids(&[]),
                        Some(color_id) => B::Color::from_color_ids(&[color_id]),
                    }
                })
                .collect()
        })
    }
}
