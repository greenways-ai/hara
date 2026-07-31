pub trait IInvokeIn<C, A> {
    type Output;

    fn invoke_in(&self, context: &mut C, arguments: A) -> Self::Output;
}
