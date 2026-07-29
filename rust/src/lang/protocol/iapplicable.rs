pub trait IApplicable<R, A, V> {
    fn apply_in(&self, runtime: &R, arguments: A) -> V;
    fn apply_default(&self) -> V;
    fn transform_in(&self, runtime: &R, arguments: A) -> V;
    fn transform_out(&self, runtime: &R, arguments: A, value: V) -> V;
}
