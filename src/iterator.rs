//! BoxedIterator just wraps around a box of an iterator, it is an owned trait object.
//! This allows it to be used inside other data-structures, such as a `Result`.
//! That means that you can `.collect()` on an `I where I: Iterator<Result<V, E>>` and get out a
//! `Result<BoxedIterator<V>, E>`. And then you can `try!` it. At least, that was my use-case.

use std::iter::FromIterator;
use std::iter::IntoIterator;

pub struct BoxedIterator<T> {
    iter: Box<Iterator<Item = T>>,
}

impl<T> BoxedIterator<T> {
    pub fn new<I>(iter: I) -> Self
        where I: Iterator<Item = T> + 'static
    {
        BoxedIterator { iter: Box::new(iter) }
    }
}

impl<T> Iterator for BoxedIterator<T> {
    type Item = T;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }
}

impl<T> FromIterator<T> for BoxedIterator<T> {
    fn from_iter<I>(iter: I) -> Self
        where I: IntoIterator<Item = T>,
              I::IntoIter: 'static
    {
        BoxedIterator { iter: Box::new(iter.into_iter()) }
    }
}
