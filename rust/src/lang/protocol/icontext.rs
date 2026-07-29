pub trait IContext<A> {
    type Output;

    fn call(&self, arguments: A) -> Self::Output;
}
