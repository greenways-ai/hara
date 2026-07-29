pub trait IRealize<V> {
    fn is_realized(&self) -> bool;
    fn realize(&self) -> V;
}
