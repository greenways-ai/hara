//! Generic persistent source-analyzer host for `.hal` modules.
//!
//! The runtime owns reusable JSONL, spanned-reader, whole-Wasm, source-range,
//! structural-summary, and protocol materialization concerns. Analyzer policy
//! remains in the supplied `.hal` module through typed `describe` and `analyze`
//! functions.

use crate::core::Value;
use crate::kernel::{normalize_schema, read_forms, Form, SchemaType, Span, SpannedForm};
use crate::lang::data::{OrderedMap, Vector};
use crate::lang::protocol::IDisplay;
use crate::vm::FunctionId;
use crate::whole_wasm::NativeModule;
use sha2::{Digest, Sha256};
use std::collections::{BTreeSet, HashMap};
use std::env;
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::Path;
use std::time::Instant;

include!("source_analyzer/runtime.rs");
include!("source_analyzer/tree.rs");
include!("source_analyzer/shape.rs");
include!("source_analyzer/protocol.rs");

#[cfg(test)]
include!("source_analyzer/tests.rs");
