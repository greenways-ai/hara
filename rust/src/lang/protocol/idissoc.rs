pub trait IDissoc<K> {
    type Output;
    fn dissoc(&self, key: &K) -> Self::Output;
}
