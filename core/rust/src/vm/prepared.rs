//! Prepare-once callable dispatch through a stable namespace Var.

use std::rc::Rc;

use crate::core::Value;
use crate::kernel::{NamespaceRegistry, Var};
use crate::lang::data::Symbol;

use super::fiber::VmFiber;
use super::opcode::Instruction;
use super::program::{FunctionPrototype, Program, MAX_PRIMITIVE_ARGUMENTS};
use super::source_map::SourceMap;
use super::validate::validate;

/// A stable global function cell plus a validated argument-call stub.
///
/// The Var is dereferenced for every call, so redefining its root takes effect
/// without rebuilding the handle. Arguments enter lexical slots directly;
/// no request binding is interned into a namespace.
#[derive(Debug, Clone)]
pub struct PreparedCall {
    symbol: String,
    arity: u16,
    var: Var<Value>,
    program: Rc<Program>,
}

impl PreparedCall {
    pub fn symbol(&self) -> &str {
        &self.symbol
    }

    pub fn arity(&self) -> u16 {
        self.arity
    }

    /// Starts one resumable invocation with the current root of the Var.
    pub fn start(&self, arguments: Vec<Value>) -> Result<VmFiber, String> {
        if arguments.len() != usize::from(self.arity) {
            return Err(format!("{} expects {} arguments", self.symbol, self.arity));
        }
        let callable = self.var.deref_value();
        if !matches!(callable, Value::Function(_)) {
            return Err(format!("prepared Var is not callable: {}", self.symbol));
        }
        let mut locals = Vec::with_capacity(arguments.len() + 1);
        locals.push(callable);
        locals.extend(arguments);
        Ok(VmFiber::start_call(
            self.program.clone(),
            0,
            locals,
            Vec::new(),
        ))
    }

    /// Invokes the prepared Var without the synthetic outer call machine.
    ///
    /// Compiled closures run their own machine and return a Promise only when
    /// execution actually suspends. Embedders that already own continuation
    /// scheduling can use this path to avoid wrapping every synchronous call
    /// in a second VM.
    pub fn invoke(&self, arguments: Vec<Value>) -> Result<Value, String> {
        if arguments.len() != usize::from(self.arity) {
            return Err(format!("{} expects {} arguments", self.symbol, self.arity));
        }
        let callable = self.var.deref_value();
        let Value::Function(function) = callable else {
            return Err(format!("prepared Var is not callable: {}", self.symbol));
        };
        crate::core::call_function(&function, arguments)
    }
}

/// Resolves a callable Var once and prepares a validated direct-argument stub.
pub fn prepare_call(
    registry: &NamespaceRegistry<Value>,
    symbol: &str,
    arity: u16,
) -> Result<PreparedCall, String> {
    if usize::from(arity) > MAX_PRIMITIVE_ARGUMENTS {
        return Err(format!(
            "prepared call arity exceeds {MAX_PRIMITIVE_ARGUMENTS}"
        ));
    }
    let parsed = Symbol::parse(symbol);
    let var = registry
        .resolve(&parsed)
        .ok_or_else(|| format!("unbound handler Var: {symbol}"))?;
    if !matches!(var.deref_value(), Value::Function(_)) {
        return Err(format!("prepared Var is not callable: {symbol}"));
    }

    let mut code = Vec::with_capacity(usize::from(arity) + 2);
    code.push(Instruction::LoadLocal(0));
    for argument in 0..arity {
        code.push(Instruction::LoadLocal(argument + 1));
    }
    code.push(Instruction::Call { argc: arity as u8 });
    code.push(Instruction::Return);
    let mut source_map = SourceMap::default();
    for _ in &code {
        source_map.record(None);
    }
    let program = Program {
        namespace: None,
        constants: Vec::new(),
        var_metadata: Vec::new(),
        schema_types: Default::default(),
        function_types: Default::default(),
        inferred_function_types: Default::default(),
        functions: vec![FunctionPrototype {
            name: Some(format!("prepared:{symbol}")),
            async_function: false,
            arity: arity + 1,
            variadic: false,
            capture_count: 0,
            local_count: arity + 1,
            max_stack: arity + 1,
            code,
            source_map,
            handlers: Vec::new(),
        }],
        entry: 0,
    };
    validate(&program).map_err(|error| error.to_string())?;
    Ok(PreparedCall {
        symbol: var.symbol().as_str().to_owned(),
        arity,
        var,
        program: Rc::new(program),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::native_function;

    #[test]
    fn prepared_calls_pass_arguments_without_globals_and_observe_redefinition() {
        let registry = NamespaceRegistry::new("app");
        let function = registry.current().intern(
            "handler",
            native_function("handler", 1, |args| Ok(args[0].clone())),
        );
        let prepared = prepare_call(&registry, "app/handler", 1).unwrap();

        assert_eq!(
            prepared
                .start(vec![Value::Number(42)])
                .unwrap()
                .drive_sync()
                .unwrap(),
            Value::Number(42)
        );
        assert_eq!(
            prepared.invoke(vec![Value::Number(42)]).unwrap(),
            Value::Number(42)
        );

        function.reset_value(native_function("handler", 1, |_| Ok(Value::Number(7))));
        assert_eq!(
            prepared
                .start(vec![Value::Number(42)])
                .unwrap()
                .drive_sync()
                .unwrap(),
            Value::Number(7)
        );
        assert!(registry
            .resolve(&Symbol::parse("__hoplite_request"))
            .is_none());
    }

    #[test]
    fn prepared_calls_validate_target_and_arity() {
        let registry = NamespaceRegistry::new("app");
        registry.current().intern("value", Value::Number(1));
        assert!(prepare_call(&registry, "app/missing", 1)
            .unwrap_err()
            .contains("unbound handler Var"));
        assert!(prepare_call(&registry, "app/value", 1)
            .unwrap_err()
            .contains("not callable"));

        registry.current().intern(
            "handler",
            native_function("handler", 1, |args| Ok(args[0].clone())),
        );
        let prepared = prepare_call(&registry, "app/handler", 1).unwrap();
        assert!(matches!(
            prepared.start(Vec::new()),
            Err(message) if message.contains("expects 1")
        ));
    }
}
