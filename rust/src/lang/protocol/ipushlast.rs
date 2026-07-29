pub trait IPushLast<E> {
    type Output;
    fn push_last(&self, value: E) -> Self::Output;
}
