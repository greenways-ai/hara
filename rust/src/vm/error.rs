//! Error types for the experimental bytecode VM.
//!
//! Compile and runtime errors carry source positions and render like the
//! parser's errors: `message [line L, column C]`.

use crate::kernel::{ParseError, Position};

/// What went wrong while compiling a form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompileErrorKind {
    /// The source could not be read at all.
    Parse,
    /// A form outside the supported synchronous subset.
    UnsupportedForm,
    /// A symbol that is not a lexical local.
    UnboundSymbol,
    /// Wrong argument or binding counts (`if`, `let`, `loop`, primitive
    /// arity above the `u8` limit).
    Arity,
    /// `recur` outside a loop, in a non-tail position, or with mismatched
    /// arity.
    Recur,
    /// A program limit (constants, code size, locals, stack, arguments).
    Limit,
    /// The compiler produced a program the validator rejected; indicates a
    /// compiler bug rather than bad source.
    Internal,
}

/// A compile-time failure with source context.
#[derive(Debug, Clone)]
pub struct CompileError {
    kind: CompileErrorKind,
    message: String,
    position: Option<Position>,
}

impl CompileError {
    pub(crate) fn new(
        kind: CompileErrorKind,
        message: impl Into<String>,
        position: Option<Position>,
    ) -> CompileError {
        CompileError {
            kind,
            message: message.into(),
            position,
        }
    }

    pub fn kind(&self) -> CompileErrorKind {
        self.kind
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn position(&self) -> Option<Position> {
        self.position
    }
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.position {
            Some(position) => write!(
                formatter,
                "{} [line {}, column {}]",
                self.message, position.line, position.column
            ),
            None => formatter.write_str(&self.message),
        }
    }
}

impl std::error::Error for CompileError {}

impl From<ParseError> for CompileError {
    fn from(error: ParseError) -> CompileError {
        CompileError::new(
            CompileErrorKind::Parse,
            error.message,
            Some(error.position),
        )
    }
}

/// A program rejected by the validator before execution.
#[derive(Debug, Clone)]
pub struct ValidationError {
    pub message: String,
    /// The instruction index the failure concerns, when applicable.
    pub instruction: Option<u32>,
}

impl ValidationError {
    pub(crate) fn new(message: impl Into<String>, instruction: Option<u32>) -> ValidationError {
        ValidationError {
            message: message.into(),
            instruction,
        }
    }
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.instruction {
            Some(instruction) => {
                write!(formatter, "validation failed at {instruction:04}: {}", self.message)
            }
            None => write!(formatter, "validation failed: {}", self.message),
        }
    }
}

impl std::error::Error for ValidationError {}

/// A runtime failure with the failing instruction and its source position.
#[derive(Debug, Clone)]
pub struct VmError {
    pub message: String,
    pub instruction: u32,
    pub position: Option<Position>,
}

impl VmError {
    pub(crate) fn new(
        message: impl Into<String>,
        instruction: u32,
        position: Option<Position>,
    ) -> VmError {
        VmError {
            message: message.into(),
            instruction,
            position,
        }
    }
}

impl std::fmt::Display for VmError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.position {
            Some(position) => write!(
                formatter,
                "{} [line {}, column {}] (instruction {:04})",
                self.message, position.line, position.column, self.instruction
            ),
            None => write!(formatter, "{} (instruction {:04})", self.message, self.instruction),
        }
    }
}

impl std::error::Error for VmError {}
