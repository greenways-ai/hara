pub trait IEmpty {
    type Output;
    fn empty(&self) -> Self::Output;
}
