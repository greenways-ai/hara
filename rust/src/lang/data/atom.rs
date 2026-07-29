use std::fmt;
use std::sync::{Arc, Mutex};

use crate::lang::protocol::{IDeref, IDisplay, IReset, IWatch};

type Validator<V> = Arc<dyn Fn(&V) -> bool + Send + Sync>;
type Watch<V> = Arc<dyn Fn(&WatchEntry<V>) + Send + Sync>;

#[derive(Clone)]
pub struct WatchEntry<V> {
    pub key: String,
    pub old_value: V,
    pub new_value: V,
}

#[derive(Clone)]
pub struct Atom<V> {
    state: Arc<Mutex<V>>,
    validator: Option<Validator<V>>,
    watches: Arc<Mutex<Vec<(String, Watch<V>)>>>,
}
impl<V> Atom<V> {
    pub fn new(value: V) -> Self {
        Self {
            state: Arc::new(Mutex::new(value)),
            validator: None,
            watches: Arc::new(Mutex::new(Vec::new())),
        }
    }
    pub fn same_identity(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.state, &other.state)
    }
    pub fn identity_address(&self) -> usize {
        Arc::as_ptr(&self.state) as usize
    }
    pub fn with_validator(
        value: V,
        validator: impl Fn(&V) -> bool + Send + Sync + 'static,
    ) -> Self {
        Self {
            validator: Some(Arc::new(validator)),
            ..Self::new(value)
        }
    }
    pub fn add_watch(
        &self,
        key: impl Into<String>,
        watch: impl Fn(&WatchEntry<V>) + Send + Sync + 'static,
    ) {
        let key = key.into();
        let mut watches = self.watches.lock().expect("atom watches");
        watches.retain(|(candidate, _)| candidate != &key);
        watches.push((key, Arc::new(watch)));
    }
    pub fn remove_watch(&self, key: &str) {
        self.watches
            .lock()
            .expect("atom watches")
            .retain(|(candidate, _)| candidate != key);
    }
    fn accepts(&self, value: &V) -> bool {
        self.validator
            .as_ref()
            .is_none_or(|validator| validator(value))
    }
}
impl<V: Clone> Atom<V> {
    pub fn deref_value(&self) -> V {
        self.state.lock().expect("atom state").clone()
    }
    pub fn reset(&self, new_value: V) -> Result<V, String> {
        if !self.accepts(&new_value) {
            return Err("atom validator rejected value".into());
        }
        let old_value = {
            let mut state = self.state.lock().expect("atom state");
            std::mem::replace(&mut *state, new_value.clone())
        };
        self.notify(old_value, new_value.clone());
        Ok(new_value)
    }
    pub fn swap(&self, f: impl FnOnce(&V) -> V) -> Result<V, String> {
        let (old_value, new_value) = {
            let mut state = self.state.lock().expect("atom state");
            let old = state.clone();
            let new = f(&old);
            if !self.accepts(&new) {
                return Err("atom validator rejected value".into());
            }
            *state = new.clone();
            (old, new)
        };
        self.notify(old_value, new_value.clone());
        Ok(new_value)
    }
    fn notify(&self, old_value: V, new_value: V) {
        for (key, watch) in self.watches.lock().expect("atom watches").iter() {
            watch(&WatchEntry {
                key: key.clone(),
                old_value: old_value.clone(),
                new_value: new_value.clone(),
            });
        }
    }
}
impl<V: Clone + PartialEq> Atom<V> {
    pub fn compare_and_set(&self, old: &V, new: V) -> Result<bool, String> {
        if !self.accepts(&new) {
            return Ok(false);
        }
        let prior = {
            let mut state = self.state.lock().expect("atom state");
            if &*state != old {
                return Ok(false);
            }
            let prior = state.clone();
            *state = new.clone();
            prior
        };
        self.notify(prior, new);
        Ok(true)
    }
}
impl<V: Clone> IDeref for Atom<V> {
    type Output = V;
    fn deref(&self) -> V {
        self.deref_value()
    }
}
impl<V: Clone> IReset<V> for Atom<V> {
    type Error = String;

    fn reset(&self, value: V) -> Result<V, Self::Error> {
        Atom::reset(self, value)
    }
}
impl<V: Clone> IWatch<V> for Atom<V> {
    type Key = String;
    type WatchEntry = WatchEntry<V>;

    fn add_watch(&self, key: Self::Key, watch: impl Fn(&Self::WatchEntry) + Send + Sync + 'static) {
        Atom::add_watch(self, key, watch);
    }

    fn remove_watch(&self, key: &Self::Key) {
        Atom::remove_watch(self, key);
    }

    fn notify_watches(&self, old_value: V, new_value: V) {
        self.notify(old_value, new_value);
    }
}
impl<V: fmt::Display + Clone> IDisplay for Atom<V> {
    fn display(&self) -> String {
        format!("#atom <{}>", self.deref_value())
    }
}
impl<V: fmt::Display + Clone> fmt::Display for Atom<V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.display())
    }
}
impl<V> fmt::Debug for Atom<V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Atom").finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::Atom;
    use crate::lang::protocol::{IReset, IWatch};
    use std::sync::{Arc, Mutex};
    #[test]
    fn validates_swaps_and_notifies() {
        let atom = Atom::with_validator(1, |v| *v >= 0);
        let seen = Arc::new(Mutex::new(Vec::new()));
        let output = seen.clone();
        atom.add_watch("test", move |event| {
            output
                .lock()
                .unwrap()
                .push((event.old_value, event.new_value))
        });
        assert_eq!(atom.swap(|v| v + 2).unwrap(), 3);
        assert!(atom.reset(-1).is_err());
        assert_eq!(&*seen.lock().unwrap(), &[(1, 3)]);
        assert_eq!(IReset::reset(&atom, 4).unwrap(), 4);
        IWatch::remove_watch(&atom, &"test".to_string());
        IWatch::notify_watches(&atom, 4, 5);
        assert_eq!(seen.lock().unwrap().len(), 2);
    }
}
