//! Dependency-free values and identities shared across Hara ABI boundaries.

use std::collections::BTreeMap;

pub const HTA_V1: &str = "hta.v1";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
    String(String),
    Integer(i64),
    Bytes(Vec<u8>),
    Record(RecordValue),
}

pub type RecordValue = BTreeMap<String, Value>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Error {
    pub code: String,
    pub detail: String,
}

impl Error {
    pub fn new(code: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            detail: detail.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct NativeIdentity {
    pub package: String,
    pub export: String,
    pub crate_name: String,
    pub abi: String,
}

impl NativeIdentity {
    pub fn new(
        package: impl Into<String>,
        export: impl Into<String>,
        crate_name: impl Into<String>,
        abi: impl Into<String>,
    ) -> Result<Self, Error> {
        let identity = Self {
            package: package.into(),
            export: export.into(),
            crate_name: crate_name.into(),
            abi: abi.into(),
        };
        for (label, value) in [
            ("package", identity.package.as_str()),
            ("export", identity.export.as_str()),
            ("crate", identity.crate_name.as_str()),
            ("abi", identity.abi.as_str()),
        ] {
            if value.is_empty() || value.chars().any(char::is_whitespace) {
                return Err(Error::new(
                    "native-identity-invalid",
                    format!("{label} must be a non-empty identifier"),
                ));
            }
        }
        Ok(identity)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_identities_are_exact_and_portable() {
        let identity = NativeIdentity::new(
            "gh:greenways-ai:hoplite-store-sqlite",
            "hoplite/store",
            "hoplite-store-sqlite",
            "hoplite-auth-store/1",
        )
        .unwrap();
        assert_eq!(identity.crate_name, "hoplite-store-sqlite");
        assert_eq!(
            NativeIdentity::new("", "hoplite/store", "crate", "abi")
                .unwrap_err()
                .code,
            "native-identity-invalid"
        );
    }
}
