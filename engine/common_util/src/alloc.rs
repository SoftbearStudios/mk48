use std::iter::FromIterator;
use std::sync::Arc;

pub fn arc_default_n<T: Default>(n: usize) -> Arc<[T]> {
    Arc::from_iter((0..n).map(|_| T::default()))
}

pub fn box_default_n<T: Default>(n: usize) -> Box<[T]> {
    Box::from_iter((0..n).map(|_| T::default()))
}
