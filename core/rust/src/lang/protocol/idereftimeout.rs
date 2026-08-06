pub trait IDerefTimeout<V> {
    fn deref_timeout(&self, milliseconds: u64, timeout_value: V) -> V;
}
