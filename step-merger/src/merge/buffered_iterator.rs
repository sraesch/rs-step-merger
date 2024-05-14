/// An iterator that buffers the items it produces when activated to revert the iterator.
pub struct BufferedIterator<Item: Clone + Sized, I: Iterator<Item = Item>> {
    buffer: Vec<Item>,
    iterator: I,
    mode: Mode,
}

/// An iterator that buffers the items it produces when activated to revert the iterator.
pub struct BufferedIteratorIter<'a, Item: Clone + Sized, I: Iterator<Item = Item>> {
    buffer: &'a mut BufferedIterator<Item, I>,
}

/// The mode of the iterator.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Mode {
    /// The iterator is in the normal state, producing items from the underlying iterator.
    Normal,
    /// The iterator is in the buffering state, each produced item is also stored in the buffer.
    FillBuffer,
    /// The iterator consumes the buffer, producing items from it.
    ReadBuffer(usize),
}

impl<Item: Clone + Sized, I: Iterator<Item = Item>> BufferedIterator<Item, I> {
    pub fn new(iterator: I) -> Self {
        Self {
            buffer: Vec::new(),
            iterator,
            mode: Mode::Normal,
        }
    }

    /// Activates the buffering mode, causing the iterator to produce items from the buffer.
    pub fn set_buffering_mode(&mut self) {
        self.buffer.clear();
        self.mode = Mode::FillBuffer;
    }

    /// Deactivates the buffering mode and starts to produce items from the underlying buffer.
    pub fn reset(&mut self) {
        if self.mode == Mode::FillBuffer {
            self.mode = Mode::ReadBuffer(0);
        }
    }

    /// Returns an iterator over the items produced by the iterator.
    pub fn iter(&mut self) -> BufferedIteratorIter<Item, I> {
        BufferedIteratorIter { buffer: self }
    }

    fn next(&mut self) -> Option<Item> {
        match self.mode {
            Mode::Normal => self.iterator.next(),
            Mode::FillBuffer => {
                let item = self.iterator.next();
                if let Some(item) = item {
                    self.buffer.push(item.clone());
                    Some(item)
                } else {
                    None
                }
            }
            Mode::ReadBuffer(index) => {
                if index < self.buffer.len() {
                    let item = self.buffer[index].clone();
                    self.mode = Mode::ReadBuffer(index + 1);
                    Some(item)
                } else {
                    self.mode = Mode::Normal;
                    self.next()
                }
            }
        }
    }
}

impl<'a, Item: Clone + Sized, I: Iterator<Item = Item>> Iterator
    for BufferedIteratorIter<'a, Item, I>
{
    type Item = Item;

    fn next(&mut self) -> Option<Self::Item> {
        self.buffer.next()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_buffered_iterator1() {
        let data = [1, 2, 3, 4, 5];
        let mut iter = BufferedIterator::new(data.iter().cloned());
        assert_eq!(iter.next(), Some(1));
        assert_eq!(iter.next(), Some(2));
        iter.set_buffering_mode();
        assert_eq!(iter.next(), Some(3));
        assert_eq!(iter.next(), Some(4));
        assert_eq!(iter.next(), Some(5));
        iter.reset();
        assert_eq!(iter.next(), Some(3));
        assert_eq!(iter.next(), Some(4));
        assert_eq!(iter.next(), Some(5));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_buffered_iterator2() {
        let data = [1, 2, 3, 4, 5];
        let mut iter = BufferedIterator::new(data.iter().cloned());
        iter.set_buffering_mode();
        assert_eq!(iter.next(), Some(1));
        assert_eq!(iter.next(), Some(2));
        assert_eq!(iter.next(), Some(3));
        iter.reset();
        assert_eq!(iter.next(), Some(1));
        assert_eq!(iter.next(), Some(2));
        assert_eq!(iter.next(), Some(3));
        assert_eq!(iter.next(), Some(4));
        assert_eq!(iter.next(), Some(5));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_buffered_iterator3() {
        let data = [1, 2, 3, 4, 5];
        let mut iter = BufferedIterator::new(data.iter().cloned());
        iter.set_buffering_mode();
        assert_eq!(iter.next(), Some(1));
        assert_eq!(iter.next(), Some(2));
        assert_eq!(iter.next(), Some(3));
        assert_eq!(iter.next(), Some(4));
        assert_eq!(iter.next(), Some(5));
        assert_eq!(iter.next(), None);
    }
}
