pub trait IPopFirst {
    type Output;
    fn pop_first(&self) -> Self::Output;
}
