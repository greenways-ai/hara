#[path = "kernel/form.rs"]
pub mod form;
#[path = "kernel/generated.rs"]
pub mod generated;
#[path = "kernel/hir.rs"]
pub mod hir;
#[path = "kernel/namespace.rs"]
pub mod namespace;
#[path = "kernel/parser.rs"]
pub mod parser;
#[path = "kernel/reader.rs"]
pub mod reader;
#[path = "kernel/var.rs"]
pub mod var;

pub use form::Form;
pub use generated::GeneratedNamespaceConfig;
pub use namespace::{Namespace, NamespaceLoadState, NamespaceRegistry};
pub use parser::{parse, parse_forms, read_forms, ParseError, Parser, Span, SpannedForm};
pub use reader::{Position, Reader};
pub use var::{Var, VarMetadata, VarOrigin};
