pub trait IPointer<K, V> {
    type Keys: IntoIterator<Item = K>;
    fn pointer_keys(&self) -> Self::Keys;
    fn pointer_value(&self, key: &K) -> Option<V>;
}
