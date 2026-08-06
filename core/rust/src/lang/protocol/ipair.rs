pub trait IPair<K, V> {
    fn key(&self) -> &K;
    fn value(&self) -> &V;
}
