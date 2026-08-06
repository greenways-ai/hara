pub trait INth<E> {
    fn nth(&self, index: usize) -> Option<&E>;
}
