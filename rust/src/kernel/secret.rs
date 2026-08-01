//! Opaque secret requirement resolution for restored sessions.
//!
//! The kernel sees descriptors only. Secret bytes remain owned by the host's
//! provider and are consumed through provider operations, never Hara values.

use crate::snapshot::SecretRequirement;
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SecretDescriptor {
    pub id: String,
    pub provider: String,
    pub version: Option<String>,
}

pub trait SecretCatalog {
    fn describe(&self, id: &str) -> Result<Option<SecretDescriptor>, String>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedSecret {
    pub id: String,
    pub provider: String,
    pub version: Option<String>,
    pub purpose: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ResolvedSecrets {
    entries: BTreeMap<String, ResolvedSecret>,
}

impl ResolvedSecrets {
    pub fn resolve(
        requirements: &[SecretRequirement],
        catalog: &dyn SecretCatalog,
    ) -> Result<Self, String> {
        let mut entries = BTreeMap::new();
        for requirement in requirements {
            let Some(descriptor) = catalog.describe(&requirement.id)? else {
                if requirement.required {
                    return Err(format!("secret/required-unavailable: {}", requirement.id));
                }
                continue;
            };
            if let Some(expected) = requirement.version.as_deref() {
                if descriptor.version.as_deref() != Some(expected) {
                    return Err(format!(
                        "secret/provider-version-mismatch: {} expected {expected}, received {}",
                        requirement.id,
                        descriptor.version.as_deref().unwrap_or("unspecified")
                    ));
                }
            }
            entries.insert(
                requirement.id.clone(),
                ResolvedSecret {
                    id: requirement.id.clone(),
                    provider: descriptor.provider,
                    version: descriptor.version,
                    purpose: requirement.purpose.clone(),
                },
            );
        }
        Ok(Self { entries })
    }

    pub fn get(&self, id: &str) -> Option<&ResolvedSecret> {
        self.entries.get(id)
    }

    pub fn ids(&self) -> impl Iterator<Item = &str> {
        self.entries.keys().map(String::as_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Catalog(BTreeMap<String, SecretDescriptor>);

    impl SecretCatalog for Catalog {
        fn describe(&self, id: &str) -> Result<Option<SecretDescriptor>, String> {
            Ok(self.0.get(id).cloned())
        }
    }

    fn requirement(required: bool, version: Option<&str>) -> SecretRequirement {
        SecretRequirement {
            id: "payments".into(),
            purpose: "sign payment requests".into(),
            required,
            version: version.map(str::to_owned),
        }
    }

    #[test]
    fn resolves_descriptors_without_receiving_secret_material() {
        let catalog = Catalog(BTreeMap::from([(
            "payments".into(),
            SecretDescriptor {
                id: "payments".into(),
                provider: "kms".into(),
                version: Some("7".into()),
            },
        )]));
        let resolved = ResolvedSecrets::resolve(&[requirement(true, Some("7"))], &catalog).unwrap();
        assert_eq!(resolved.get("payments").unwrap().provider, "kms");
        assert_eq!(resolved.ids().collect::<Vec<_>>(), ["payments"]);
    }

    #[test]
    fn required_missing_or_wrong_version_prevents_publication() {
        let empty = Catalog(BTreeMap::new());
        assert!(ResolvedSecrets::resolve(&[requirement(true, None)], &empty)
            .unwrap_err()
            .contains("required-unavailable"));
        assert!(
            ResolvedSecrets::resolve(&[requirement(false, None)], &empty)
                .unwrap()
                .ids()
                .next()
                .is_none()
        );

        let catalog = Catalog(BTreeMap::from([(
            "payments".into(),
            SecretDescriptor {
                id: "payments".into(),
                provider: "kms".into(),
                version: Some("8".into()),
            },
        )]));
        assert!(
            ResolvedSecrets::resolve(&[requirement(true, Some("7"))], &catalog)
                .unwrap_err()
                .contains("version-mismatch")
        );
    }
}
