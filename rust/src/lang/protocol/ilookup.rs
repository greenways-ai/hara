use super::IFind;

pub trait ILookup<K, V>: IFind<K, Output = (K, V)>
where
    K: Clone,
    V: Clone,
{
    type Keys: Iterator<Item = K>;
    type Values: Iterator<Item = V>;
    fn keys(&self) -> Self::Keys;
    fn vals(&self) -> Self::Values;
    fn lookup(&self, key: &K) -> Option<V> {
        self.find(key).map(|(_, value)| value)
    }
    fn lookup_or(&self, key: &K, not_found: V) -> V {
        self.lookup(key).unwrap_or(not_found)
    }
}
