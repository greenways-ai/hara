pub trait ICas<V> {
    type Error;

    fn cas(&self, old_value: &V, new_value: V) -> Result<bool, Self::Error>;
}
