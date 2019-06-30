use std::fmt::Display;
use std::hash::Hash;
use std::iter::once;
use std::ops::Range;

use hashbrown::{HashMap, HashSet};

pub fn pad<T>(s: &T, max_size: usize, right: bool) -> String
where
    T: Display,
{
    if right {
        format!("{:<width$}", s, width = max_size)
    } else {
        format!("{:>width$}", s, width = max_size)
    }
}

pub fn pad_with<T: Clone>(v: &mut Vec<T>, el: T, max_size: usize, right: bool) {
    if let Some(additional) = max_size.checked_sub(v.len()) {
        if additional == 0 {
            return;
        }

        let plus = std::iter::repeat(el).take(additional);

        if right {
            v.extend(plus);
        } else {
            let _ = v.splice(..0, plus);
        }
    }
}

pub fn transpose<T: Clone>(input: &[Vec<T>]) -> Result<Vec<Vec<T>>, String> {
    if input.is_empty() {
        return Ok(vec![]);
    }

    let sizes: Vec<_> = input.iter().map(Vec::len).collect();
    let min_size = sizes.iter().min().unwrap_or(&0);
    let max_size = sizes.iter().max().unwrap_or(&0);

    if min_size != max_size {
        return Err(format!("Jagged matrix: {} vs {}", min_size, max_size));
    }

    Ok((0..input[0].len())
        .map(|j| input.iter().map(|row| row[j].clone()).collect())
        .collect())
}

pub fn replace<T>(vec: &mut Vec<T>, what: &T, with_what: &T)
where
    T: PartialEq + Clone,
{
    if what == with_what {
        return;
    }

    for x in vec {
        if *x == *what {
            *x = with_what.clone();
        }
    }
}

pub fn two_powers(num: u32) -> impl Iterator<Item = u32> {
    (0..num.count_ones()).scan(num, |num, _i| {
        let prev = *num;
        *num = prev & (prev - 1);
        Some(prev - *num)
    })
}

pub fn from_two_powers(numbers: &[u32]) -> u32 {
    numbers.iter().fold(0, |acc, &x| acc | x)
}

pub fn dedup<T>(vec: &[T]) -> Vec<T>
where
    T: Eq + Hash + Clone,
{
    let set: HashSet<_> = vec.iter().cloned().collect();
    set.into_iter().collect()
}

pub mod iter {
    #[cfg(feature = "try_trait")]
    use std::ops::Try;

    pub trait FindOk: Iterator {
        #[cfg(feature = "try_trait")]
        /// Generalization of `find_map` for any `Try` types
        /// If the iterator is exhausted, return `on_empty_error`.
        fn first_ok_with_error<B, E, F, R>(&mut self, on_empty_error: E, mut f: F) -> R
        where
            Self: Sized,
            F: FnMut(Self::Item) -> R,
            R: Try<Ok = B, Error = E>,
        {
            let mut return_err = on_empty_error;

            for x in self {
                match f(x).into_result() {
                    Ok(res) => return Try::from_ok(res),
                    Err(e) => return_err = e,
                }
            }

            Try::from_error(return_err)
        }

        #[cfg(not(feature = "try_trait"))]
        /// Generalization of `find_map` for `Result` type.
        /// If the iterator is exhausted, return `on_empty_error`.
        fn first_ok_with_error<B, E, F>(&mut self, on_empty_error: E, mut f: F) -> Result<B, E>
        where
            Self: Sized,
            F: FnMut(Self::Item) -> Result<B, E>,
        {
            let mut return_err = on_empty_error;

            for x in self {
                match f(x) {
                    Ok(res) => return Ok(res),
                    Err(e) => return_err = e,
                }
            }

            Err(return_err)
        }

        /// Generalization of `find_map` for `Result` type.
        /// If the iterator is exhausted, return default error for provided `E` type.
        fn first_ok<B, E, F>(&mut self, f: F) -> Result<B, E>
        where
            Self: Sized,
            E: Default,
            F: FnMut(Self::Item) -> Result<B, E>,
        {
            self.first_ok_with_error(E::default(), f)
        }
    }

    impl<I: Iterator> FindOk for I {}

    pub trait PartialEntry {
        type Output: Copy;

        fn unwrap_or_insert_with<F>(&mut self, index: usize, default: F) -> Self::Output
        where
            F: FnOnce() -> Self::Output;

        fn with_none(capacity: usize) -> Self;
    }

    impl<T> PartialEntry for Vec<Option<T>>
    where
        T: Copy,
    {
        type Output = T;

        fn unwrap_or_insert_with<F: FnOnce() -> T>(&mut self, index: usize, default: F) -> T {
            if let Some(elem) = self.get(index) {
                if let Some(y) = elem {
                    return *y;
                }
            }

            let new = default();
            self[index] = Some(new);
            new
        }

        fn with_none(capacity: usize) -> Self {
            vec![None; capacity]
        }
    }
}

pub fn product<T, U>(s1: &[T], s2: &[U]) -> Vec<(T, U)>
where
    T: Clone,
    U: Clone,
{
    s1.iter()
        .flat_map(|x| s2.iter().map(move |y| (x.clone(), y.clone())))
        .collect()
}

fn idx_to_ranges<T>(indexes: &[T]) -> Option<Vec<Range<T>>>
where
    T: Ord + Clone,
{
    if indexes.len() < 2 {
        return None;
    }

    let mut indexes = indexes.to_owned();
    indexes.sort_unstable();
    let shifted: Vec<_> = indexes[1..].to_vec();

    Some(
        indexes
            .into_iter()
            .zip(shifted)
            .map(|(x, y)| Range { start: x, end: y })
            .collect(),
    )
}

pub fn split_sections<'a, 'b>(
    text: &'a str,
    section_names: &'b [&'b str],
    include_header: bool,
    first_section: Option<&'b str>,
) -> Result<HashMap<&'b str, Vec<&'a str>>, String> {
    let lines: Vec<_> = text.lines().map(str::trim).collect();

    let first_section_name = first_section.unwrap_or("");

    let section_indexes: Result<HashMap<_, _>, String> = section_names
        .iter()
        .map(|&section| {
            Ok((
                section,
                lines
                    .iter()
                    .position(|&r| r == section)
                    .ok_or_else(|| format!("{:?} section not found", section))?,
            ))
        })
        .chain(once(Ok((first_section_name, 0))))
        .collect();

    let section_indexes = section_indexes?;

    let eof = lines.len();
    let indexes_with_eof: Vec<_> = section_indexes.values().cloned().chain(once(eof)).collect();

    let mut ranges: HashMap<_, _> = idx_to_ranges(&indexes_with_eof)
        .ok_or_else(|| "Should be enough indexes".to_string())?
        .into_iter()
        .map(|range| (range.start, range))
        .collect();

    let res = section_indexes
        .iter()
        .map(|(&section, start)| {
            let mut range = ranges.remove(start).expect("Start of section not found");
            if !include_header {
                range.start += 1;
            }
            (section, lines[range].to_vec())
        })
        .collect();

    Ok(res)
}

pub mod time {
    use std::time::Instant;

    #[cfg(feature = "std_time")]
    pub fn now() -> Option<Instant> {
        Some(Instant::now())
    }

    #[cfg(not(feature = "std_time"))]
    pub fn now() -> Option<Instant> {
        None
    }
}

pub mod rc {
    #[cfg(feature = "threaded")]
    use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};
    #[cfg(not(feature = "threaded"))]
    use std::{
        cell::{Ref, RefCell, RefMut},
        rc::Rc,
    };

    #[derive(Debug)]
    pub struct MutRc<T>(ReadRc<InteriorMutableRef<T>>);

    impl<T> MutRc<T> {
        pub fn new(data: T) -> Self {
            Self(ReadRc::new(InteriorMutableRef::new(data)))
        }

        pub fn read(&self) -> ReadRef<T> {
            read_ref(&self.0)
        }

        pub fn write(&self) -> MutRef<T> {
            mutate_ref(&self.0)
        }
    }

    impl<T> Clone for MutRc<T> {
        fn clone(&self) -> Self {
            Self(ReadRc::clone(&self.0))
        }
    }

    #[cfg(feature = "threaded")]
    pub type ReadRc<T> = Arc<T>;
    #[cfg(feature = "threaded")]
    pub type InteriorMutableRef<T> = RwLock<T>;
    #[cfg(feature = "threaded")]
    pub type ReadRef<'a, T> = RwLockReadGuard<'a, T>;
    #[cfg(feature = "threaded")]
    pub type MutRef<'a, T> = RwLockWriteGuard<'a, T>;

    #[cfg(feature = "threaded")]
    pub fn read_ref<T>(cell_value: &InteriorMutableRef<T>) -> ReadRef<T> {
        cell_value
            .read()
            .expect("Cannot read the value under mutable reference: already locked for writing.")
    }

    #[cfg(feature = "threaded")]
    pub fn mutate_ref<T>(cell_value: &InteriorMutableRef<T>) -> MutRef<T> {
        cell_value
            .write()
            .expect("Cannot write the value under mutable reference: already locked.")
    }

    #[cfg(not(feature = "threaded"))]
    pub type ReadRc<T> = Rc<T>;
    #[cfg(not(feature = "threaded"))]
    pub type InteriorMutableRef<T> = RefCell<T>;
    #[cfg(not(feature = "threaded"))]
    pub type ReadRef<'a, T> = Ref<'a, T>;
    #[cfg(not(feature = "threaded"))]
    pub type MutRef<'a, T> = RefMut<'a, T>;

    #[cfg(not(feature = "threaded"))]
    pub fn read_ref<T>(cell_value: &InteriorMutableRef<T>) -> ReadRef<T> {
        cell_value.borrow()
    }

    #[cfg(not(feature = "threaded"))]
    pub fn mutate_ref<T>(cell_value: &InteriorMutableRef<T>) -> MutRef<T> {
        cell_value.borrow_mut()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pad_vector_left() {
        let mut v = vec![1, 2];
        pad_with(&mut v, 0, 4, false);
        assert_eq!(v, [0, 0, 1, 2])
    }

    #[test]
    fn pad_vector_right() {
        let mut v = vec![1, 2];
        pad_with(&mut v, 0, 4, true);
        assert_eq!(v, [1, 2, 0, 0])
    }

    #[test]
    fn do_not_pad_vector_right() {
        let mut v = vec![1, 2, 3];
        pad_with(&mut v, 0, 2, true);
        assert_eq!(v, [1, 2, 3])
    }

    #[test]
    fn do_not_pad_vector_left() {
        let mut v = vec![1, 2, 3];
        pad_with(&mut v, 0, 2, false);
        assert_eq!(v, [1, 2, 3])
    }

    #[test]
    fn pad_string_left() {
        let s = "hello";
        let s2 = pad(&s.to_string(), 7, false);
        assert_eq!(s2, "  hello")
    }

    #[test]
    fn pad_string_right() {
        let s = "world";
        let s2 = pad(&s.to_string(), 7, true);
        assert_eq!(s2, "world  ")
    }

    #[test]
    fn do_not_pad_string_left() {
        let s = "hello";
        let s2 = pad(&s.to_string(), 4, false);
        assert_eq!(s2, "hello")
    }

    #[test]
    fn do_not_pad_string_right() {
        let s = "world";
        let s2 = pad(&s.to_string(), 5, true);
        assert_eq!(s2, "world")
    }

    #[test]
    fn pad_non_ascii_right() {
        let s = "Привет";
        let s2 = pad(&s.to_string(), 8, true);
        assert_eq!(s2, "Привет  ")
    }

    #[test]
    fn transpose_empty() {
        let m: Vec<Vec<u8>> = vec![];
        assert_eq!(transpose(&m).unwrap(), Vec::<Vec<u8>>::new())
    }

    #[test]
    fn transpose_empty_rows() {
        let m: Vec<Vec<u8>> = vec![vec![], vec![], vec![]];
        assert_eq!(transpose(&m).unwrap(), Vec::<Vec<u8>>::new())
    }

    #[test]
    fn transpose_square() {
        let m = vec![vec![1, 2, 3], vec![4, 5, 6], vec![7, 8, 9]];
        assert_eq!(
            transpose(&m).unwrap(),
            vec![vec![1, 4, 7], vec![2, 5, 8], vec![3, 6, 9]]
        )
    }

    #[test]
    fn transpose_jagged() {
        let m = vec![vec![1, 2, 3], vec![4, 5], vec![7, 8, 9]];
        assert_eq!(transpose(&m).unwrap_err(), "Jagged matrix: 2 vs 3")
    }

    #[test]
    fn replace_ints() {
        let mut v = vec![1, 2, 3, 2];
        replace(&mut v, &2, &5);

        assert_eq!(v, vec![1, 5, 3, 5]);
    }

    #[test]
    fn no_replacement() {
        let mut v = vec![1, 2, 3, 2];
        replace(&mut v, &5, &4);

        assert_eq!(v, vec![1, 2, 3, 2]);
    }

    #[test]
    fn product_2_by_3() {
        let a = ['a', 'b'];
        let b: Vec<_> = (0..3).collect();

        assert_eq!(
            product(&a, &b),
            vec![('a', 0), ('a', 1), ('a', 2), ('b', 0), ('b', 1), ('b', 2)]
        );
    }

    #[test]
    fn two_powers_factorization() {
        let numbers = [0, 1, 2, 5, 42, 1000, 23700723];
        let results = [
            vec![],
            vec![1],
            vec![2],
            vec![1, 4],
            vec![2, 8, 32],
            vec![8, 32, 64, 128, 256, 512],
            vec![
                1,
                2,
                16,
                32,
                64,
                128,
                1 << 10,
                1 << 13,
                1 << 15,
                1 << 16,
                1 << 19,
                1 << 21,
                1 << 22,
                1 << 24,
            ],
        ];
        for (x, res) in numbers.into_iter().zip(&results) {
            assert_eq!(&two_powers(*x).collect::<Vec<_>>(), res);
        }
    }

    #[test]
    fn to_ranges_empty() {
        let vec: Vec<u8> = vec![];

        assert_eq!(idx_to_ranges(&vec), None);
    }

    #[test]
    fn to_ranges_single() {
        let vec = vec![5];
        assert_eq!(idx_to_ranges(&vec), None);
    }

    #[test]
    fn to_ranges_unsorted() {
        let vec = vec![9, 5];
        assert_eq!(idx_to_ranges(&vec), Some(vec![5..9]));
    }

    #[test]
    fn to_ranges_multiple() {
        let vec = vec![5, 9, 42, 111, 84, 7, 0];
        assert_eq!(
            idx_to_ranges(&vec),
            Some(vec![0..5, 5..7, 7..9, 9..42, 42..84, 84..111])
        );
    }
}
