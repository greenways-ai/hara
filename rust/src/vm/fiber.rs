use std::rc::Rc;

use crate::core::{Promise, PromiseState, Value};

use super::error::VmError;
use super::machine::{Machine, VmOutcome};
use super::program::Program;

#[derive(Debug, Clone)]
pub enum VmFiberState {
    Running,
    Suspended,
    Completed(Value),
    Failed(VmError),
    Cancelled,
}

/// Host-independent lifetime for resumable bytecode execution. The machine
/// owns every frame, local, operand and handler needed to continue.
pub struct VmFiber {
    machine: Machine,
    state: VmFiberState,
    pending: Option<Promise>,
}

impl VmFiber {
    pub fn start(program: Rc<Program>) -> Self {
        let mut fiber = Self {
            machine: Machine::entry(program),
            state: VmFiberState::Running,
            pending: None,
        };
        let outcome = fiber.machine.run();
        fiber.apply(outcome);
        fiber
    }

    pub fn state(&self) -> VmFiberState {
        self.state.clone()
    }

    pub fn pending(&self) -> Option<Promise> {
        self.pending.clone()
    }

    pub fn resume(&mut self, state: PromiseState) -> VmFiberState {
        if !matches!(self.state, VmFiberState::Suspended) {
            return self.state();
        }
        self.state = VmFiberState::Running;
        self.pending = None;
        let outcome = self.machine.resume(state);
        self.apply(outcome);
        self.state()
    }

    /// Drains queued child resumptions and advances this fiber when its
    /// awaited promise became settled. Hosts call this from their event loop.
    pub fn poll(&mut self) -> VmFiberState {
        self.machine.poll_async();
        if matches!(self.state, VmFiberState::Suspended) {
            let state = self.pending.as_ref().expect("suspended promise").state();
            if !matches!(state, PromiseState::Pending) {
                return self.resume(state);
            }
        }
        self.state()
    }

    pub fn cancel(&mut self) -> bool {
        if matches!(
            self.state,
            VmFiberState::Completed(_) | VmFiberState::Failed(_) | VmFiberState::Cancelled
        ) {
            return false;
        }
        if let Some(promise) = self.pending.take() {
            promise.notify_cancel();
        }
        self.state = VmFiberState::Cancelled;
        true
    }

    pub fn drive_sync(&mut self) -> Result<Value, VmError> {
        loop {
            match self.state() {
                VmFiberState::Completed(value) => return Ok(value),
                VmFiberState::Failed(error) => return Err(error),
                VmFiberState::Cancelled => {
                    return Err(VmError::new("cancelled", 0, None));
                }
                VmFiberState::Suspended => {
                    let state = self.pending.as_ref().expect("suspended promise").state();
                    if matches!(state, PromiseState::Pending) {
                        return Err(VmError::new(
                            "VM fiber suspended on an unresolved promise",
                            0,
                            None,
                        ));
                    }
                    self.resume(state);
                }
                VmFiberState::Running => {
                    let outcome = self.machine.run();
                    self.apply(outcome);
                }
            }
        }
    }

    fn apply(&mut self, outcome: VmOutcome) {
        match outcome {
            VmOutcome::Returned(value) => self.state = VmFiberState::Completed(value),
            VmOutcome::Failed(error) => self.state = VmFiberState::Failed(error),
            VmOutcome::Suspended(promise) => {
                self.pending = Some(promise);
                self.state = VmFiberState::Suspended;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vm::opcode::Instruction;
    use crate::vm::program::FunctionPrototype;
    use crate::vm::source_map::SourceMap;

    fn program(promise: Promise) -> Rc<Program> {
        let code = vec![
            Instruction::Constant(0),
            Instruction::Await,
            Instruction::Return,
        ];
        let mut source_map = SourceMap::default();
        for _ in &code {
            source_map.record(None);
        }
        Rc::new(Program {
            constants: vec![Value::Promise(promise)],
            var_metadata: vec![],
            functions: vec![FunctionPrototype {
                name: None,
                async_function: false,
                arity: 0,
                variadic: false,
                capture_count: 0,
                local_count: 0,
                max_stack: 1,
                code,
                source_map,
                handlers: vec![],
            }],
            entry: 0,
        })
    }

    #[test]
    fn pending_await_preserves_machine_and_resumes() {
        let promise = Promise::new();
        let mut fiber = VmFiber::start(program(promise.clone()));
        assert!(matches!(fiber.state(), VmFiberState::Suspended));
        promise.resolve(Value::Number(42));
        assert!(matches!(
            fiber.resume(promise.state()),
            VmFiberState::Completed(Value::Number(42))
        ));
    }

    #[test]
    fn settled_await_stays_on_the_synchronous_path() {
        let promise = Promise::new();
        promise.resolve(Value::Number(7));
        let fiber = VmFiber::start(program(promise));
        assert!(matches!(
            fiber.state(),
            VmFiberState::Completed(Value::Number(7))
        ));
    }
}
