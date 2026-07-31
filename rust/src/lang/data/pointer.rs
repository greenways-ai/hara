use crate::lang::data::Symbol;
use crate::lang::protocol::{HashType, IDisplay, IHash, IMetadata, INamespaced, IObjType, ObjType};
use std::fmt;
use std::hash::{Hash, Hasher};
use std::rc::Rc;
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Pointer(Symbol);
impl Pointer {
    pub fn create(namespace: Option<&str>, name: &str) -> Self {
        Self(Symbol::create(namespace, name))
    }
    pub fn parse(path: &str) -> Self {
        Self(Symbol::parse(path))
    }
    pub fn path(&self) -> &str {
        self.0.as_str()
    }
}
impl INamespaced for Pointer {
    fn get_name(&self) -> &str {
        self.0.get_name()
    }
    fn get_namespace(&self) -> Option<&str> {
        self.0.get_namespace()
    }
}
impl IMetadata for Pointer {
    type Metadata = Rc<crate::lang::data::Metadata>;
    fn meta(&self) -> Option<&Self::Metadata> {
        self.0.meta()
    }
    fn with_meta(&self, metadata: Option<Self::Metadata>) -> Self {
        Self(self.0.with_meta(metadata))
    }
}
impl IDisplay for Pointer {
    fn display(&self) -> String {
        format!("#'{}", self.path())
    }
}
impl IObjType for Pointer {
    fn obj_type(&self) -> ObjType {
        ObjType::Pointer
    }
}
impl IHash for Pointer {
    fn hash_calc(&self, hash_type: HashType) -> u64 {
        // DEVIATION from Java: Pointer.hashCalc is System.identityHashCode
        // (non-deterministic). This port uses the deterministic string-type
        // hash of "::POINTER|" + path (see lang::hash module docs).
        crate::lang::hash::hash_string_type(
            hash_type,
            &format!("{}|{}", self.hash_seed(), self.path()),
        ) as u64
    }
}
impl crate::lang::hash::JavaHash for Pointer {
    fn java_hash(&self, hash_type: HashType) -> i64 {
        self.hash_calc(hash_type) as i64
    }
}
impl Hash for Pointer {
    fn hash<H: Hasher>(&self, state: &mut H) {
        Hash::hash(&self.0, state)
    }
}
impl fmt::Display for Pointer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.display())
    }
}
