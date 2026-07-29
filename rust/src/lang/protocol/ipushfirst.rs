pub trait IPushFirst<E> {
    type Output;
    fn push_first(&self, value: E) -> Self::Output;
}
