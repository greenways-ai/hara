pub trait ICons<E> {
    type Output;
    fn cons(&self, value: E) -> Self::Output;
}
