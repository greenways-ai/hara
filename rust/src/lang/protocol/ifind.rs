pub trait IFind<K> {
    type Output;

    fn find(&self, key: &K) -> Option<Self::Output>;

    fn has(&self, key: &K) -> bool {
        self.find(key).is_some()
    }
}
