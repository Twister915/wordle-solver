use std::marker::PhantomData;
use std::iter::FusedIterator;

///
/// Iterator over Option<E>. Used to iterate over guesses. I used to use a filter_map and fuse to
/// implement this, but technically that implementation was incorrect since [None, Some(x), ..]
/// should emit no items, but instead emitted the item at [1].
///
/// This struct simply iterates over any &[Option<E>] emitting all Some(E) until encountering the
/// first None... which causes the iterator to Fuse immediately.
///

pub struct OptionIter<I, E> {
    upstream: Option<I>,
    max_size: Option<usize>,
    _e: PhantomData<E>,
}

impl<I, E> OptionIter<I, E>
where
    I: Iterator<Item=Option<E>>,
{
    pub fn new(upstream: I) -> Self {
        let (_, max_size) = upstream.size_hint();
        Self {
            upstream: Some(upstream),
            max_size,
            _e: PhantomData,
        }
    }
}

impl<I, E> Iterator for OptionIter<I, E>
where
    I: Iterator<Item=Option<E>>,
{
    type Item = E;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(next) = self.upstream.as_mut().and_then(|u| u.next()).flatten() {
            Some(next)
        } else {
            self.upstream = None;
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, self.upstream
            .as_ref()
            .map(|u| u.size_hint())
            .map(|(_, max_size)| max_size)
            .unwrap_or(self.max_size))
    }
}

impl<I, E> FusedIterator for OptionIter<I, E>
where
    I: Iterator<Item=Option<E>>,
{}