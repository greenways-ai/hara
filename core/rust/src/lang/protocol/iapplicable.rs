pub trait IApplicable<R, A> {
    type Output;

    fn apply_in(&self, runtime: &mut R, arguments: A) -> Self::Output;
    fn apply_default(&mut self) -> &mut R;
    fn transform_in(&self, runtime: &R, arguments: A) -> A;
    fn transform_out(&self, runtime: &R, arguments: A, value: Self::Output) -> Self::Output;
}
