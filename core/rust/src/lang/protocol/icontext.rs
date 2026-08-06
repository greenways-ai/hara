pub trait IContext<A> {
    type Output;

    fn call(&mut self, arguments: A) -> Self::Output;
}
