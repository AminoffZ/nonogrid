use super::super::block::{Block, Description};
use super::super::board::{Board, Point};
use super::line::LineSolver;

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

use log::Level;

pub struct Solver<B>
where
    B: Block,
{
    board: Rc<RefCell<Board<B>>>,
    point: Option<Point>,
}

type Job = (bool, usize);

impl<B> Solver<B>
where
    B: Block,
{
    pub fn new(board: Rc<RefCell<Board<B>>>) -> Self {
        Self { board, point: None }
    }

    pub fn with_point(board: Rc<RefCell<Board<B>>>, point: Point) -> Self {
        Self {
            board,
            point: Some(point),
        }
    }

    pub fn run<S>(&self) -> Result<Vec<Point>, String>
    where
        S: LineSolver<BlockType = B>,
    {
        let (rows, columns) = if let Some(point) = self.point {
            (vec![point.y()], vec![point.x()])
        } else {
            let board = self.board.borrow();
            let rows: Vec<_> = (0..board.height()).collect();
            let cols: Vec<_> = (0..board.width()).collect();

            // `is_solved_full` is expensive, so minimize calls to it.
            // Do not call if only a handful of lines has to be solved
            if board.is_solved_full() {
                //return 0, ()
            }
            (rows, cols)
        };

        let start = Instant::now();
        let mut lines_solved = 0_u32;

        // every job is a tuple (is_column, index)
        //
        // Why `is_column`, not `is_row`?
        // To assign more priority to the rows:
        // when adding row, `is_column = False = 0`
        // when adding column, `is_column = True = 1`
        // heap always pops the lowest item, so the rows will go first

        debug!(
            "Solving {:?} rows and {:?} columns with {} method",
            &rows, &columns, "standard"
        );

        let mut line_jobs = Vec::with_capacity(rows.len() + columns.len());

        line_jobs.extend(columns.into_iter().rev().map(|column_index| {
            // the more this line solved
            // priority = 1 - board.column_solution_rate(column_index)

            // the closer to edge
            // priority = 1 - abs(2.0 * column_index / board.width - 1)

            // the more 'dense' this line
            // priority = 1 - board.densities[True][column_index]

            //let new_job = (true, column_index);

            // if has_blots:
            //   // the more attempts the less priority
            //   priority = board.attempts_to_try(*new_job)

            (true, column_index)
        }));

        line_jobs.extend(rows.into_iter().rev().map(|row_index| {
            // the more this line solved
            // priority = 1 - board.row_solution_rate(row_index)

            // the closer to edge
            // priority = 1 - abs(2.0 * row_index / board.height - 1)

            // the more 'dense' this line
            // priority = 1 - board.densities[False][row_index]

            //let new_job = (false, row_index);

            // if has_blots:
            //    // the more attempts the less priority
            //    priority = board.attempts_to_try(*new_job)

            (false, row_index)
        }));

        let mut solved_cells = vec![];

        while let Some((is_column, index)) = Self::get_top_job(&mut line_jobs) {
            let new_jobs = self.update_line::<S>(index, is_column)?;

            let new_states = new_jobs.iter().map(|(another_index, _color)| {
                let (x, y) = if is_column {
                    (&index, another_index)
                } else {
                    (another_index, &index)
                };
                Point::new(*x, *y)
            });

            solved_cells.extend(new_states);

            line_jobs.extend(new_jobs.into_iter().rev().map(|(new_index, _color)| {
                // if board.has_blots:
                //    // the more attempts the less priority
                //    new_priority = board.attempts_to_try(*new_job)

                //let new_job = (!is_column, new_index);
                // higher priority = more priority
                //add_job(new_job, new_priority);

                (!is_column, new_index)
            }));

            lines_solved += 1;
        }

        // all the following actions applied only to verified solving
        //if !self.contradiction_mode
        {
            //let board = board.borrow();
            //board.solution_round_completed()
            //let rate = board.solution_rate();
            //if rate != 1 {
            //    warn!("The nonogram is not solved full: {:.4}", rate)
            //}

            if log_enabled!(Level::Info) {
                let total_time = start.elapsed();
                info!(
                    "Full solution: {}.{:06} sec",
                    total_time.as_secs(),
                    total_time.subsec_micros()
                );
                info!("Lines solved: {}", lines_solved);
            }
        }

        Ok(solved_cells)
    }

    fn get_top_job(pq: &mut Vec<Job>) -> Option<Job> {
        let top_job = pq.pop()?;
        let before_retain_size = pq.len();
        // remove all the previous occurrences of the new job
        pq.retain(|&x| x != top_job);

        if log_enabled!(Level::Debug) {
            debug!(
                "Removed {} instances of job {:?}",
                before_retain_size - pq.len(),
                top_job
            );

            let (is_column, index) = top_job;
            let line_description = if is_column { "column" } else { "row" };
            debug!("Solving {} {}", index, line_description);
        }
        Some(top_job)
    }

    /// Solve a line with the solver S and update the board.
    /// If the line gets partially solved, put the crossed lines into queue.
    ///
    /// Return the list of indexes which was updated during this solution.
    pub fn update_line<S>(
        &self,
        index: usize,
        is_column: bool,
    ) -> Result<Vec<(usize, B::Color)>, String>
    where
        S: LineSolver<BlockType = B>,
    {
        let start = Instant::now();

        let (line_desc, line) = {
            let board = self.board.borrow();
            if is_column {
                (
                    Rc::clone(&board.descriptions(false)[index]),
                    board.get_column(index),
                )
            } else {
                (
                    Rc::clone(&board.descriptions(true)[index]),
                    board.get_row(index),
                )
            }
        };

        //let pre_solution_rate = Board::<B>::line_solution_rate(&line);
        //if pre_solution_rate == 1 {
        //    // do not check solved lines in trusted mode
        //    if ! contradiction_mode {
        //        return vec![];
        //     }
        //}

        if log_enabled!(Level::Debug) {
            let name = if is_column { "column" } else { "row" };
            debug!(
                "Solving {} {}: {:?}. Partial: {:?}",
                index, name, line_desc, line
            );
        }

        let line = Rc::new(line);
        let solution = self.solve::<S>(index, is_column, line_desc, Rc::clone(&line))?;
        let indexes = self.update_solved(index, is_column, &line, &solution);

        if log_enabled!(Level::Debug) {
            let name = if is_column { "column" } else { "row" };
            let total_time = start.elapsed();
            debug!(
                "{}s solution: {}.{:06} sec",
                name,
                total_time.as_secs(),
                total_time.subsec_micros()
            );
            if !indexes.is_empty() {
                debug!("New info on {} {}: {:?}", name, index, indexes);
            }
        }

        Ok(indexes)
    }

    fn update_solved(
        &self,
        index: usize,
        is_column: bool,
        old: &[B::Color],
        new: &[B::Color],
    ) -> Vec<(usize, B::Color)> {
        // let new_solution_rate = Board::<B>::line_solution_rate(&updated);
        // if new_solution_rate > pre_solution_rate

        if old == new {
            return vec![];
        }
        let mut board = self.board.borrow_mut();

        if is_column {
            board.set_column(index, new);
        } else {
            board.set_row(index, new);
        }

        debug!("Original: {:?}", old);
        debug!("Updated: {:?}", new);

        old.iter()
            .zip(new)
            .enumerate()
            .filter_map(|(i, (pre, post))| {
                if pre != post {
                    debug!(
                        "Diff on index={}: original={:?}, updated={:?}",
                        i, pre, &post
                    );
                    Some((i, *post))
                } else {
                    None
                }
            })
            .collect()
    }

    fn solve<S>(
        &self,
        index: usize,
        is_column: bool,
        line_desc: Rc<Description<B>>,
        line: Rc<Vec<B::Color>>,
    ) -> Result<Rc<Vec<B::Color>>, String>
    where
        S: LineSolver<BlockType = B>,
    {
        let cache_index = if is_column {
            self.board.borrow().column_cache_index(index)
        } else {
            self.board.borrow().row_cache_index(index)
        };
        let key = (cache_index, Rc::clone(&line));

        let cached = self.board.borrow_mut().cached_solution(is_column, &key);

        if let Some(cached) = cached {
            return cached;
        }

        let mut line_solver = S::new(line_desc, line);
        let value = line_solver.solve();

        let rc_value = value.map(Rc::new);
        self.board
            .borrow_mut()
            .set_cached_solution(is_column, key, rc_value.clone());
        rc_value
    }
}
