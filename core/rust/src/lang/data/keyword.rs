use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::{Rc, Weak};

use crate::lang::protocol::{
    HashType, IDisplay, IHash, IHashCached, ILookup, IMetadata, INamespaced, IObjType, MetaType,
    ObjType,
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
pub struct Keyword(Rc<Data>);

impl Keyword {
    pub fn create(namespace: Option<&str>, name: &str) -> Result<Self, String> {
        let full = namespace
            .map(|ns| format!("{ns}/{name}"))
            .unwrap_or_else(|| name.into());
        Ok(Self::intern(namespace, name, &full))
    }

    pub fn parse(full: &str) -> Result<Self, String> {
        validate(full)?;
        let slash = full.find(char::from(47));
        Ok(Self::intern(
            slash.map(|i| &full[..i]),
            slash.map(|i| &full[i + 1..]).unwrap_or(full),
            full,
        ))
    }

    fn intern(namespace: Option<&str>, name: &str, full: &str) -> Self {
        INTERNED.with(|cache| {
            if let Some(value) = cache.borrow().get(full).and_then(Weak::upgrade) {
                return Self(value);
            }
            let data = Rc::new(Data {
                namespace: namespace.map(str::to_owned),
                name: name.into(),
                full: full.into(),
                hash: Cell::new(0),
            });
            cache.borrow_mut().insert(full.into(), Rc::downgrade(&data));
            let keyword = Self(data);
            // Java parity: Keyword.create precomputes the hash at intern time
            // (k.hashGet() inside the RefCache factory), so first use is free.
            keyword.hash_put(keyword.hash());
            keyword
        })
    }

    pub fn as_str(&self) -> &str {
        &self.0.full
    }
    pub fn same_identity(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }

    pub fn lookup<V: Clone, L: ILookup<Self, V>>(&self, target: &L) -> Option<V> {
        target.lookup(self)
    }

    pub fn lookup_or<V: Clone, L: ILookup<Self, V>>(&self, target: &L, fallback: V) -> V {
        target.lookup_or(self, fallback)
    }
}

fn validate(full: &str) -> Result<(), String> {
    if full.is_empty() {
        return Err("Keyword name cannot be empty.".into());
    }
    if full == "/" {
        return Err("Keyword name cannot be a single slash.".into());
    }
    if full.bytes().filter(|byte| *byte == b'/').count() > 1 {
        return Err("Keyword name can only contain one slash.".into());
    }
    if full.starts_with('/') {
        return Err("Keyword name cannot start with a slash.".into());
    }
    if full.ends_with('/') {
        return Err("Keyword name cannot end with a slash.".into());
    }
    Ok(())
}

impl INamespaced for Keyword {
    fn get_name(&self) -> &str {
        &self.0.name
    }
    fn get_namespace(&self) -> Option<&str> {
        self.0.namespace.as_deref()
    }
}
impl IMetadata for Keyword {
    type Metadata = Rc<crate::lang::data::Metadata>;

    fn meta(&self) -> Option<&Self::Metadata> {
        None
    }

    fn with_meta(&self, _metadata: Option<Self::Metadata>) -> Self {
        self.clone()
    }

    fn metatype(&self) -> MetaType {
        MetaType::String
    }
}
impl IDisplay for Keyword {
    fn display(&self) -> String {
        format!(":{}", self.0.full)
    }
}
impl IObjType for Keyword {
    fn obj_type(&self) -> ObjType {
        ObjType::Keyword
    }
}
impl IHash for Keyword {
    fn hash_calc(&self, hash_type: HashType) -> u64 {
        // DEVIATION from Java: IStringType.hashCalc uses toString(), and
        // Java's Keyword does not override toString(), so the Java hash is
        // built on Object identity garbage ("::KEYWORD|hara.lang.data.Keyword@…")
        // and is non-deterministic across JVM runs. This port standardises on
        // the display form "::KEYWORD|:ns/name" (see lang::hash module docs).
        crate::lang::hash::hash_string_type(
            hash_type,
            &format!("{}|{}", self.hash_seed(), self.display()),
        ) as u64
    }
    fn hash_get(&self) -> u64 {
        self.hash_cached()
    }
    fn hash_get_as(&self, hash_type: HashType) -> u64 {
        self.hash_cached_as(hash_type)
    }
}
impl IHashCached for Keyword {
    fn hash_current(&self) -> u64 {
        self.0.hash.get()
    }
    fn hash_put(&self, hash: u64) {
        self.0.hash.set(hash);
    }
}
impl crate::lang::hash::JavaHash for Keyword {
    fn java_hash(&self, hash_type: HashType) -> i64 {
        self.hash_calc(hash_type) as i64
    }
}
impl PartialEq for Keyword {
    fn eq(&self, other: &Self) -> bool {
        self.0.full == other.0.full
    }
}
impl Eq for Keyword {}
impl PartialOrd for Keyword {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Keyword {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.full.cmp(&other.0.full)
    }
}
impl std::hash::Hash for Keyword {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.full.hash(state);
    }
}

impl From<&str> for Keyword {
    fn from(value: &str) -> Self {
        Self::parse(value).expect("valid keyword")
    }
}
impl From<String> for Keyword {
    fn from(value: String) -> Self {
        Self::from(value.as_str())
    }
}
impl std::fmt::Display for Keyword {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::Keyword;
    use crate::lang::data::Map;
    use crate::lang::protocol::IAssoc;
    use crate::lang::protocol::{
        HashType, IDisplay, IHash, IMetadata, INamespaced, IObjType, MetaType, ObjType,
    };

    #[test]
    fn matches_java_validation_namespace_and_interning() {
        let first = Keyword::parse("hara/name").unwrap();
        let second = Keyword::create(Some("hara"), "name").unwrap();
        assert!(first.same_identity(&second));
        assert_eq!(first.get_namespace(), Some("hara"));
        assert_eq!(first.get_name(), "name");
        assert_eq!(first.display(), ":hara/name");
        assert_eq!(first.obj_type(), ObjType::Keyword);
        assert_eq!(first.hash_seed(), "::KEYWORD");
        assert_eq!(first.metatype(), MetaType::String);
        assert_eq!(first.hash_get(), first.hash());
        assert_eq!(
            first.hash_get_as(HashType::Murmur3),
            first.hash_calc(HashType::Murmur3)
        );
        for invalid in ["", "/", "/name", "name/", "a/b/c"] {
            assert!(Keyword::parse(invalid).is_err());
        }

        let values = Map::new().assoc(first.clone(), 42);
        assert_eq!(first.lookup(&values), Some(42));
        assert_eq!(Keyword::from("missing").lookup_or(&values, 7), 7);

        let multipart = Keyword::create(Some("constructor/namespace"), "name").unwrap();
        assert_eq!(multipart.as_str(), "constructor/namespace/name");
        assert_eq!(multipart.get_namespace(), Some("constructor/namespace"));
        assert_eq!(multipart.get_name(), "name");
        assert!(Keyword::parse("constructor/namespace/name").is_err());

        let documented = first.with_meta(Some(crate::lang::data::Metadata::document("ignored")));
        assert!(documented.meta().is_none());
        assert!(documented.same_identity(&first));
    }

    #[test]
    fn intern_precomputes_the_hash() {
        use crate::lang::protocol::IHashCached;
        // Java Keyword.create calls k.hashGet() inside the cache factory, so
        // the cached hash is populated at intern time and first use is free.
        let keyword = Keyword::parse("precomputed/hash").unwrap();
        let current = keyword.hash_current();
        assert_ne!(current, 0);
        assert_eq!(current, keyword.hash_calc(HashType::Rapid));
        assert_eq!(keyword.hash_get(), current);
        assert_eq!(keyword.hash_get_as(HashType::Rapid), current);
        assert_eq!(
            keyword.hash_get_as(HashType::Murmur3),
            keyword.hash_calc(HashType::Murmur3)
        );
        // re-interning the same name returns the same precomputed data
        let again = Keyword::create(Some("precomputed"), "hash").unwrap();
        assert_eq!(again.hash_current(), current);
        assert!(keyword.same_identity(&again));
    }
}
