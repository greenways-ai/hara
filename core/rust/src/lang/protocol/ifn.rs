pub trait IFn<A> {
    type Output;

    fn invoke(&self, arguments: A) -> Self::Output;
}
