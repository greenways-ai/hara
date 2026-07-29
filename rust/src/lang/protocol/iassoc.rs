pub trait IAssoc<K, V> {
    type Output;
    fn assoc(&self, key: K, value: V) -> Self::Output;
}
