pub trait IWatch<V: Clone> {
    type Key: Clone + Eq;
    type WatchEntry;

    fn add_watch(&self, key: Self::Key, watch: impl Fn(&Self::WatchEntry) + Send + Sync + 'static);
    fn remove_watch(&self, key: &Self::Key);
    fn notify_watches(&self, old_value: V, new_value: V);
}
