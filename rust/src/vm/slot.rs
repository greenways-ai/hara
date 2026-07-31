//! Compact VM-owned values that must not cross the public runtime boundary.

use std::rc::Rc;

use crate::core::Value;

#[derive(Clone, Debug)]
pub(crate) enum VmSlot {
    Number(i64),
    Bool(bool),
    Nil,
    Value(Box<Value>),
    Closure(Rc<VmClosure>),
}

#[derive(Clone, Debug)]
pub(crate) struct VmClosure {
    pub prototype: u16,
    pub captures: Vec<VmSlot>,
}

impl VmSlot {
    pub fn runtime_value(&self) -> Option<Value> {
        match self {
            VmSlot::Number(value) => Some(Value::Number(*value)),
            VmSlot::Bool(value) => Some(Value::Bool(*value)),
            VmSlot::Nil => Some(Value::Nil),
            VmSlot::Value(value) => Some((**value).clone()),
            VmSlot::Closure(_) => None,
        }
    }

    pub fn truthy(&self) -> bool {
        match self {
            VmSlot::Bool(false) | VmSlot::Nil => false,
            VmSlot::Number(_) | VmSlot::Bool(true) | VmSlot::Value(_) => true,
            VmSlot::Closure(_) => true,
        }
    }
}

impl From<Value> for VmSlot {
    fn from(value: Value) -> Self {
        match value {
            Value::Number(value) => VmSlot::Number(value),
            Value::Bool(value) => VmSlot::Bool(value),
            Value::Nil => VmSlot::Nil,
            value => VmSlot::Value(Box::new(value)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::VmSlot;

    #[test]
    fn hot_vm_slots_stay_two_machine_words_or_smaller() {
        assert!(std::mem::size_of::<VmSlot>() <= 16);
    }
}
