use crate::core::Value;

/// Opaque ABI token for a Hara value owned by one prepared native call.
/// The high half is the call generation and the low half is a one-based
/// arena index, so a token retained by guest code cannot alias a later call.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct Handle(u64);

impl Handle {
    pub fn from_abi(value: i64) -> Self {
        Self(value as u64)
    }

    pub fn to_abi(self) -> i64 {
        self.0 as i64
    }

    fn new(generation: u32, index: u32) -> Self {
        Self((u64::from(generation) << 32) | u64::from(index))
    }

    fn generation(self) -> u32 {
        (self.0 >> 32) as u32
    }

    fn index(self) -> u32 {
        self.0 as u32
    }
}

/// Scoped native-value arena used by imported collection operations.
/// Reading returns a cheap `Value` clone; persistent collections retain their
/// shared immutable roots and therefore do not copy their contents on read.
#[derive(Debug, Default)]
pub(crate) struct HandleScope {
    generation: u32,
    values: Vec<Value>,
}

impl HandleScope {
    pub fn begin_call(&mut self) {
        self.values.clear();
        self.generation = self.generation.wrapping_add(1);
        if self.generation == 0 {
            self.generation = 1;
        }
    }

    pub fn insert(&mut self, value: Value) -> Result<Handle, String> {
        let index = u32::try_from(self.values.len() + 1)
            .map_err(|_| "whole-Wasm handle scope exhausted")?;
        self.values.push(value);
        Ok(Handle::new(self.generation, index))
    }

    pub fn get(&self, handle: Handle) -> Result<Value, String> {
        if handle.generation() != self.generation || handle.index() == 0 {
            return Err("stale whole-Wasm runtime handle".into());
        }
        self.values
            .get(handle.index() as usize - 1)
            .cloned()
            .ok_or_else(|| "invalid whole-Wasm runtime handle".into())
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }
}

#[cfg(test)]
mod tests {
    use super::{Handle, HandleScope};
    use crate::core::Value;

    #[test]
    fn handles_are_call_scoped_and_generation_checked() {
        let mut scope = HandleScope::default();
        scope.begin_call();
        let first = scope.insert(Value::Number(19)).unwrap();
        assert_eq!(scope.get(first), Ok(Value::Number(19)));
        assert_eq!(Handle::from_abi(first.to_abi()), first);

        scope.begin_call();
        let second = scope.insert(Value::Number(23)).unwrap();
        assert_ne!(first, second);
        assert_eq!(
            scope.get(first),
            Err("stale whole-Wasm runtime handle".into())
        );
        assert_eq!(scope.get(second), Ok(Value::Number(23)));
        assert_eq!(scope.len(), 1);
    }

    #[test]
    fn persistent_values_are_shared_across_handle_reads() {
        let value = crate::vm::eval_source("{:nested [1 2 3]}").unwrap();
        let mut scope = HandleScope::default();
        scope.begin_call();
        let handle = scope.insert(value.clone()).unwrap();
        assert_eq!(scope.get(handle), Ok(value));
    }
}
