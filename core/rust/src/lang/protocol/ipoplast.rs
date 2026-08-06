pub trait IPopLast {
    type Output;
    fn pop_last(&self) -> Self::Output;
}
