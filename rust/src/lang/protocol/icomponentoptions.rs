pub trait IComponentOptions {
    type Options;

    fn options(&self) -> Self::Options;
}
