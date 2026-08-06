use std::collections::{HashMap, HashSet, VecDeque};
use std::hash::Hash;

#[derive(Debug, Clone, Default)]
pub struct AsList<E>(VecDeque<E>);
impl<E> AsList<E> {
    pub fn new(values: impl IntoIterator<Item = E>) -> Self {
        Self(values.into_iter().collect())
    }
    pub fn len(&self) -> usize {
        self.0.len()
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    pub fn empty(&mut self) -> &mut Self {
        self.0.clear();
        self
    }
    pub fn nth(&self, index: usize) -> Option<&E> {
        self.0.get(index)
    }
    pub fn peek_first(&self) -> Option<&E> {
        self.0.front()
    }
    pub fn peek_last(&self) -> Option<&E> {
        self.0.back()
    }
    pub fn pop_first(&mut self) -> Option<E> {
        self.0.pop_front()
    }
    pub fn pop_last(&mut self) -> Option<E> {
        self.0.pop_back()
    }
    pub fn push_first(&mut self, value: E) -> &mut Self {
        self.0.push_front(value);
        self
    }
    pub fn push_last(&mut self, value: E) -> &mut Self {
        self.0.push_back(value);
        self
    }
    pub fn iter(&self) -> impl Iterator<Item = &E> {
        self.0.iter()
    }
}

#[derive(Debug, Clone, Default)]
pub struct AsMap<K, V>(HashMap<K, V>);
impl<K: Eq + Hash, V> AsMap<K, V> {
    pub fn new(values: impl IntoIterator<Item = (K, V)>) -> Self {
        Self(values.into_iter().collect())
    }
    pub fn len(&self) -> usize {
        self.0.len()
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    pub fn assoc(&mut self, key: K, value: V) -> &mut Self {
        self.0.insert(key, value);
        self
    }
    pub fn dissoc(&mut self, key: &K) -> &mut Self {
        self.0.remove(key);
        self
    }
    pub fn empty(&mut self) -> &mut Self {
        self.0.clear();
        self
    }
    pub fn find(&self, key: &K) -> Option<(&K, &V)> {
        self.0.get_key_value(key)
    }
    pub fn get(&self, key: &K) -> Option<&V> {
        self.0.get(key)
    }
    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.0.iter()
    }
}

#[derive(Debug, Clone, Default)]
pub struct AsSet<E>(HashSet<E>);
impl<E: Eq + Hash> AsSet<E> {
    pub fn new(values: impl IntoIterator<Item = E>) -> Self {
        Self(values.into_iter().collect())
    }
    pub fn len(&self) -> usize {
        self.0.len()
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    pub fn conj(&mut self, value: E) -> &mut Self {
        self.0.insert(value);
        self
    }
    pub fn dissoc(&mut self, value: &E) -> &mut Self {
        self.0.remove(value);
        self
    }
    pub fn empty(&mut self) -> &mut Self {
        self.0.clear();
        self
    }
    pub fn find(&self, value: &E) -> Option<&E> {
        self.0.get(value)
    }
    pub fn iter(&self) -> impl Iterator<Item = &E> {
        self.0.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::{AsList, AsMap, AsSet};
    #[test]
    fn expose_mutable_host_collections() {
        let mut list = AsList::new([2]);
        list.push_first(1).push_last(3);
        assert_eq!(list.iter().copied().collect::<Vec<_>>(), vec![1, 2, 3]);
        let mut map = AsMap::new([]);
        map.assoc("a", 1);
        assert_eq!(map.get(&"a"), Some(&1));
        let mut set = AsSet::new([1, 1]);
        set.conj(2);
        assert_eq!(set.len(), 2);
    }
}
