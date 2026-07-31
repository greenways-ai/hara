use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::{Rc, Weak};

use crate::lang::protocol::{
    HashType, IDisplay, IHash, IHashCached, IMetadata, INamespaced, IObjType, MetaType, ObjType,
};

thread_local! {
    static INTERNED: RefCell<HashMap<String, Weak<Data>>> = RefCell::new(HashMap::new());
}

#[derive(Debug)]
struct Data {
    namespace: Option<String>,
    name: String,
    full: String,
    hash: Cell<u64>,
}

#[derive(Debug, Clone)]
pub struct Symbol {
    data: Rc<Data>,
    metadata: Option<Rc<crate::lang::data::Metadata>>,
}

impl Symbol {
    pub fn create(namespace: Option<&str>, name: &str) -> Self {
        let full = namespace
            .map(|ns| format!("{ns}/{name}"))
            .unwrap_or_else(|| name.into());
        Self::intern(namespace, name, &full)
    }

    pub fn parse(full: &str) -> Self {
        let slash = if full == "/" {
            None
        } else {
            full.find(char::from(47))
        };
        Self::intern(
            slash.map(|i| &full[..i]),
            slash.map(|i| &full[i + 1..]).unwrap_or(full),
            full,
        )
    }

    fn intern(namespace: Option<&str>, name: &str, full: &str) -> Self {
        INTERNED.with(|cache| {
            if let Some(value) = cache.borrow().get(full).and_then(Weak::upgrade) {
                return Self {
                    data: value,
                    metadata: None,
                };
            }
            let data = Rc::new(Data {
                namespace: namespace.map(str::to_owned),
                name: name.into(),
                full: full.into(),
                hash: Cell::new(0),
            });
            cache.borrow_mut().insert(full.into(), Rc::downgrade(&data));
            let symbol = Self {
                data,
                metadata: None,
            };
            // Java parity: ObjPersistent.hashGet() is populated at creation,
            // so the first hash lookup on an interned symbol is free.
            symbol.hash_put(symbol.hash());
            symbol
        })
    }

    pub fn as_str(&self) -> &str {
        &self.data.full
    }
    pub fn same_identity(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.data, &other.data)
    }
}

impl INamespaced for Symbol {
    fn get_name(&self) -> &str {
        &self.data.name
    }
    fn get_namespace(&self) -> Option<&str> {
        self.data.namespace.as_deref()
    }
}
impl IMetadata for Symbol {
    type Metadata = Rc<crate::lang::data::Metadata>;

    fn meta(&self) -> Option<&Self::Metadata> {
        self.metadata.as_ref()
    }

    fn with_meta(&self, metadata: Option<Self::Metadata>) -> Self {
        Self {
            data: self.data.clone(),
            metadata,
        }
    }

    fn metatype(&self) -> MetaType {
        MetaType::String
    }
}
impl IDisplay for Symbol {
    fn display(&self) -> String {
        self.data.full.clone()
    }
}
impl IObjType for Symbol {
    fn obj_type(&self) -> ObjType {
        ObjType::Symbol
    }
}
impl IHash for Symbol {
    fn hash_calc(&self, hash_type: HashType) -> u64 {
        // Mirrors Java exactly: Symbol inherits ObjPersistent.toString() =
        // class-name + "<" + display() + ">" (deterministic, class-qualified),
        // so Java hashes "::SYMBOL|hara.lang.data.Symbol<ns/name>".
        crate::lang::hash::hash_string_type(
            hash_type,
            &format!("{}|hara.lang.data.Symbol<{}>", self.hash_seed(), self.display()),
        ) as u64
    }
    fn hash_get(&self) -> u64 {
        self.hash_cached()
    }
    fn hash_get_as(&self, hash_type: HashType) -> u64 {
        self.hash_cached_as(hash_type)
    }
}
impl IHashCached for Symbol {
    fn hash_current(&self) -> u64 {
        self.data.hash.get()
    }
    fn hash_put(&self, hash: u64) {
        self.data.hash.set(hash);
    }
}
impl crate::lang::hash::JavaHash for Symbol {
    fn java_hash(&self, hash_type: HashType) -> i64 {
        self.hash_calc(hash_type) as i64
    }
}
impl PartialEq for Symbol {
    fn eq(&self, other: &Self) -> bool {
        self.data.full == other.data.full
    }
}
impl Eq for Symbol {}
impl std::hash::Hash for Symbol {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.data.full.hash(state);
    }
}

impl From<&str> for Symbol {
    fn from(value: &str) -> Self {
        Self::parse(value)
    }
}
impl From<String> for Symbol {
    fn from(value: String) -> Self {
        Self::parse(&value)
    }
}
impl std::fmt::Display for Symbol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::Symbol;
    use crate::lang::protocol::{
        HashType, IDisplay, IHash, IMetadata, INamespaced, IObjType, MetaType, ObjType,
    };

    #[test]
    fn matches_java_namespace_and_interning() {
        let first = Symbol::parse("hara/name");
        let second = Symbol::create(Some("hara"), "name");
        assert!(first.same_identity(&second));
        assert_eq!(first.get_namespace(), Some("hara"));
        assert_eq!(first.get_name(), "name");
        assert_eq!(first.display(), "hara/name");
        assert_eq!(first.obj_type(), ObjType::Symbol);
        assert_eq!(first.hash_seed(), "::SYMBOL");
        assert_eq!(first.metatype(), MetaType::String);
        assert_eq!(first.hash_get(), first.hash());
        assert_eq!(first.hash_type(), HashType::Rapid);
        assert_eq!(Symbol::parse("/").get_namespace(), None);
        let nested = Symbol::parse("a/b/c");
        assert_eq!(nested.get_namespace(), Some("a"));
        assert_eq!(nested.get_name(), "b/c");
        assert_eq!(nested.as_str(), "a/b/c");

        let multipart = Symbol::create(Some("constructor/namespace"), "name");
        assert_eq!(multipart.as_str(), "constructor/namespace/name");
        assert_eq!(multipart.get_namespace(), Some("constructor/namespace"));
        assert_eq!(multipart.get_name(), "name");
        let interned = Symbol::parse("constructor/namespace/name");
        assert!(multipart.same_identity(&interned));
        assert_eq!(interned.get_namespace(), Some("constructor/namespace"));

        let documented = first.with_meta(Some(crate::lang::data::Metadata::document("doc")));
        assert_eq!(documented.meta().and_then(|value| value.doc()), Some("doc"));
        assert_eq!(documented, first);
        assert!(documented.same_identity(&first));
    }

    #[test]
    fn intern_precomputes_the_hash() {
        use crate::lang::protocol::IHashCached;
        // Java precomputes the hash at creation (ObjPersistent.hashGet), so
        // the cached hash is populated at intern time and first use is free.
        let symbol = Symbol::parse("precomputed/hash");
        let current = symbol.hash_current();
        assert_ne!(current, 0);
        assert_eq!(current, symbol.hash_calc(HashType::Rapid));
        assert_eq!(symbol.hash_get(), current);
        assert_eq!(symbol.hash_get_as(HashType::Rapid), current);
        assert_eq!(
            symbol.hash_get_as(HashType::Murmur3),
            symbol.hash_calc(HashType::Murmur3)
        );
        // metadata variants share the interned precomputed hash
        let documented = symbol.with_meta(Some(crate::lang::data::Metadata::document("doc")));
        assert_eq!(documented.hash_current(), current);
        assert_eq!(documented.hash_get(), current);
    }
}
