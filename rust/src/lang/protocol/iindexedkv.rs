pub trait IIndexedKV<K, V> {
    fn index_of_key(&self, key: &K) -> Option<usize>;
    fn index_of_val(&self, value: &V) -> Option<usize>;
}
