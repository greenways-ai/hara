#[path = "kernel/form.rs"]
pub mod form;
#[path = "kernel/generated.rs"]
pub mod generated;
#[path = "kernel/halc.rs"]
pub mod halc;
#[path = "kernel/namespace.rs"]
pub mod namespace;
#[path = "kernel/parser.rs"]
pub mod parser;
#[path = "kernel/reader.rs"]
pub mod reader;
#[cfg(not(target_arch = "wasm32"))]
#[path = "kernel/secret.rs"]
pub mod secret;
#[cfg(not(target_arch = "wasm32"))]
#[path = "kernel/session_snapshot.rs"]
pub mod session_snapshot;
#[path = "kernel/var.rs"]
pub mod var;

pub use form::Form;
pub use generated::GeneratedNamespaceConfig;
pub use namespace::{Namespace, NamespaceLoadState, NamespaceRegistry};
pub use parser::{parse, parse_forms, read_forms, ParseError, Parser, Span, SpannedForm};
pub use reader::{Position, Reader};
#[cfg(not(target_arch = "wasm32"))]
pub use secret::{ResolvedSecret, ResolvedSecrets, SecretCatalog, SecretDescriptor};
#[cfg(not(target_arch = "wasm32"))]
pub use session_snapshot::{
    FrozenSession, SessionMode, SharedStateCell, SnapshotKernel, SnapshotRegistry,
    SnapshotSessionDefinition,
};
pub use var::{Var, VarMetadata, VarOrigin};
