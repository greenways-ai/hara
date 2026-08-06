pub trait IIter {
    type Item;
    type Iter: Iterator<Item = Self::Item>;

    fn iter(&self) -> Self::Iter;
}
