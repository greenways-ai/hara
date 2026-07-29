pub trait IHasRuntime {
    type Runtime;

    fn runtime(&self) -> Self::Runtime;
}
