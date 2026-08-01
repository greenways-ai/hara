use crate::core::Primitive;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraceValue {
    I64(i64),
    Bool(bool),
    Nil,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TraceOp {
    GuardLocalI64 { local: u16 },
    LoadLocal { local: u16 },
    ConstantI64(i64),
    BinaryI64(Primitive),
    StoreLocal { local: u16 },
    GuardTruthy { expected: bool },
    Pop,
    LoopBackedge,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Trace {
    pub function: u16,
    pub header: u32,
    pub resume_ip: u32,
    pub operations: Vec<TraceOp>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExitReason {
    WrongTag,
    BranchChanged,
    Overflow,
    DivisionByZero,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExitSnapshot {
    pub function: u16,
    pub instruction: u32,
    pub locals: Vec<TraceValue>,
    pub stack: Vec<TraceValue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TraceOutcome {
    Completed { iterations: u32 },
    SideExit { reason: ExitReason, snapshot: ExitSnapshot },
}
