pub trait IDeref {
    type Output;

    fn deref(&self) -> Self::Output;
}
