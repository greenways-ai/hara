use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

use crate::lang::data::{Atom, Metadata, Symbol};
use crate::lang::protocol::{IDeref, IDisplay, INamespaced, IReset};

thread_local! {
    static DYNAMIC_BINDINGS: RefCell<HashMap<usize, Vec<Box<dyn Any>>>> = RefCell::new(HashMap::new());
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum VarOrigin {
    #[default]
    Source,
    HalFallback,
    RustLibrary,
    RuntimePrimitive,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct VarMetadata {
    pub hara: Option<Rc<Metadata>>,
    pub control: bool,
    pub dynamic: bool,
    pub macro_form: bool,
    pub doc: Option<String>,
    pub arglists: Vec<String>,
    pub origin: VarOrigin,
    pub extra: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct Var<V> {
    identity: Rc<()>,
    symbol: Symbol,
    value: Atom<V>,
    metadata: Rc<RefCell<VarMetadata>>,
}
impl<V> Var<V> {
    pub fn new(path: impl AsRef<str>, value: V) -> Self {
        Self {
            identity: Rc::new(()),
            symbol: Symbol::parse(path.as_ref()),
            value: Atom::new(value),
            metadata: Rc::new(RefCell::new(VarMetadata::default())),
        }
    }
    pub fn with_metadata(path: impl AsRef<str>, value: V, metadata: VarMetadata) -> Self {
        Self {
            identity: Rc::new(()),
            symbol: Symbol::parse(path.as_ref()),
            value: Atom::new(value),
            metadata: Rc::new(RefCell::new(metadata)),
        }
    }
    pub fn symbol(&self) -> &Symbol {
        &self.symbol
    }
    pub fn metadata(&self) -> VarMetadata {
        self.metadata.borrow().clone()
    }
    pub fn hara_metadata(&self) -> Option<Rc<Metadata>> {
        self.metadata.borrow().hara.clone()
    }
    pub fn set_hara_metadata(&self, metadata: Option<Rc<Metadata>>) {
        self.metadata.borrow_mut().hara = metadata;
    }
    pub fn set_metadata(&self, metadata: VarMetadata) -> VarMetadata {
        std::mem::replace(&mut *self.metadata.borrow_mut(), metadata)
    }
    pub fn update_metadata(&self, update: impl FnOnce(&mut VarMetadata)) {
        update(&mut self.metadata.borrow_mut());
    }
    pub fn origin(&self) -> VarOrigin {
        self.metadata.borrow().origin
    }
    pub fn set_origin(&self, origin: VarOrigin) {
        self.metadata.borrow_mut().origin = origin;
    }
    pub fn identity_address(&self) -> usize {
        self.identity_key()
    }
    pub fn same_identity(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.identity, &other.identity)
    }
    fn identity_key(&self) -> usize {
        Rc::as_ptr(&self.identity) as usize
    }
    pub fn is_control(&self) -> bool {
        self.metadata.borrow().control
            || self
                .metadata
                .borrow()
                .hara
                .as_ref()
                .is_some_and(|meta| meta.flag("control"))
    }
    pub fn is_dynamic(&self) -> bool {
        self.metadata.borrow().dynamic
            || self
                .metadata
                .borrow()
                .hara
                .as_ref()
                .is_some_and(|meta| meta.flag("dynamic"))
    }
    pub fn is_macro(&self) -> bool {
        self.metadata.borrow().macro_form
            || self
                .metadata
                .borrow()
                .hara
                .as_ref()
                .is_some_and(|meta| meta.flag("macro"))
    }
}
impl<V: Clone + 'static> Var<V> {
    pub fn requalify(&self, path: impl AsRef<str>) -> Self {
        Self {
            identity: self.identity.clone(),
            symbol: Symbol::parse(path.as_ref()),
            value: self.value.clone(),
            metadata: self.metadata.clone(),
        }
    }
    pub fn deref_value(&self) -> V {
        let key = self.identity_key();
        DYNAMIC_BINDINGS.with(|bindings| {
            bindings
                .borrow()
                .get(&key)
                .and_then(|stack| stack.last())
                .and_then(|value| value.downcast_ref::<V>())
                .cloned()
                .unwrap_or_else(|| self.value.deref_value())
        })
    }
    pub fn bind(&self, value: V) {
        let key = self.identity_key();
        DYNAMIC_BINDINGS.with(|bindings| {
            bindings
                .borrow_mut()
                .entry(key)
                .or_default()
                .push(Box::new(value));
        });
    }
    pub fn unbind(&self) -> Result<V, String> {
        let key = self.identity_key();
        DYNAMIC_BINDINGS.with(|bindings| {
            let mut bindings = bindings.borrow_mut();
            let stack = bindings
                .get_mut(&key)
                .ok_or_else(|| "Var has no dynamic binding".to_string())?;
            let value = stack
                .pop()
                .and_then(|value| value.downcast::<V>().ok())
                .map(|value| *value)
                .ok_or_else(|| "Var dynamic binding type mismatch".to_string())?;
            if stack.is_empty() {
                bindings.remove(&key);
            }
            Ok(value)
        })
    }
    pub fn reset_value(&self, value: V) -> V {
        self.value.reset(value).expect("unvalidated var")
    }
}
impl<V: Clone + 'static> IDeref for Var<V> {
    type Output = V;
    fn deref(&self) -> V {
        self.deref_value()
    }
}
impl<V: Clone + 'static> IReset<V> for Var<V> {
    type Error = String;
    fn reset(&self, value: V) -> Result<V, String> {
        Ok(self.reset_value(value))
    }
}
impl<V> INamespaced for Var<V> {
    fn get_name(&self) -> &str {
        self.symbol.get_name()
    }
    fn get_namespace(&self) -> Option<&str> {
        self.symbol.get_namespace()
    }
}
impl<V> IDisplay for Var<V> {
    fn display(&self) -> String {
        format!("#'{}", self.symbol.as_str())
    }
}
impl<V> fmt::Display for Var<V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.display())
    }
}

#[cfg(test)]
mod tests {
    use super::{Var, VarMetadata};
    use crate::lang::protocol::{IDeref, IDisplay, INamespaced};
    #[test]
    fn is_namespaced_resettable_and_metadata_driven() {
        let var = Var::with_metadata(
            "hello/value",
            1,
            VarMetadata {
                dynamic: true,
                ..Default::default()
            },
        );
        assert_eq!(var.get_namespace(), Some("hello"));
        assert_eq!(var.display(), "#'hello/value");
        assert_eq!(var.reset_value(2), 2);
        assert_eq!(var.deref(), 2);
        assert!(var.is_dynamic());
    }
    #[test]
    fn dynamic_bindings_are_nested_thread_local_and_share_identity() {
        let var = Var::new("hello/value", 1);
        let alias = var.clone();
        assert!(var.same_identity(&alias));
        var.bind(2);
        assert_eq!(alias.deref(), 2);
        alias.bind(3);
        assert_eq!(var.deref(), 3);
        assert_eq!(var.unbind().unwrap(), 3);
        assert_eq!(alias.deref(), 2);
        assert_eq!(alias.unbind().unwrap(), 2);
        assert_eq!(var.deref(), 1);
        assert!(var.unbind().is_err());
    }
    #[test]
    fn cloned_vars_share_metadata_updates() {
        let var = Var::new("hello/value", 1);
        let alias = var.clone();
        var.update_metadata(|meta| {
            meta.doc = Some("A value".into());
            meta.arglists.push("[x]".into());
        });
        assert_eq!(alias.metadata().doc.as_deref(), Some("A value"));
        assert_eq!(alias.metadata().arglists, vec!["[x]"]);
        let old = alias.set_metadata(VarMetadata {
            dynamic: true,
            ..Default::default()
        });
        assert_eq!(old.doc.as_deref(), Some("A value"));
        assert!(var.is_dynamic());
    }
    #[test]
    fn requalification_preserves_identity_root_metadata_and_bindings() {
        let var = Var::new("value", 1);
        var.update_metadata(|meta| meta.dynamic = true);
        let qualified = var.requalify("hello/value");
        assert!(var.same_identity(&qualified));
        assert_eq!(qualified.symbol().as_str(), "hello/value");
        qualified.reset_value(2);
        assert_eq!(var.deref(), 2);
        var.bind(3);
        assert_eq!(qualified.deref(), 3);
        qualified.unbind().unwrap();
        assert!(qualified.is_dynamic());
    }
}
