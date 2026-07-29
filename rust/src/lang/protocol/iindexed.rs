pub trait IIndexed<V> {
    type Index;
    fn index_of(&self, value: &V) -> Option<Self::Index>;
}
