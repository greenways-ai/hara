pub trait IReset<V> {
    type Error;
    fn reset(&self, value: V) -> Result<V, Self::Error>;
}
