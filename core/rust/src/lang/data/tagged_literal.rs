use crate::lang::data::Symbol;
use crate::lang::protocol::IDisplay;
use std::fmt;
use std::hash::{Hash, Hasher};
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TaggedLiteral<T> {
    tag: Symbol,
    form: T,
}
impl<T> TaggedLiteral<T> {
    pub fn new(tag: Symbol, form: T) -> Self {
        Self { tag, form }
    }
    pub fn tag(&self) -> &Symbol {
        &self.tag
    }
    pub fn form(&self) -> &T {
        &self.form
    }
    pub fn into_form(self) -> T {
        self.form
    }
}
impl<T: fmt::Display> IDisplay for TaggedLiteral<T> {
    fn display(&self) -> String {
        format!("#{}{}", self.tag.as_str(), self.form)
    }
}
impl<T: fmt::Display> fmt::Display for TaggedLiteral<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.display())
    }
}
impl<T: Hash> Hash for TaggedLiteral<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.tag.hash(state);
        self.form.hash(state)
    }
}
