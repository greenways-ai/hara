pub trait IConj<E> {
    type Output;
    fn conj(&self, value: E) -> Self::Output;
}
