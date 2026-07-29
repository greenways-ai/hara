use super::IIter;

pub trait IIterator: IIter + Iterator<Item = <Self as IIter>::Item> {
    fn iter_next(&mut self) -> Option<<Self as IIter>::Item> {
        self.next()
    }

    fn iter_next_available(&mut self) -> bool;
}
