//! Global, struct, and multi-arity instruction helpers.

use super::{constant_string, constant_string_vector, Machine, Program, Value};
use crate::lang::data::Symbol;

impl Machine {
    #[inline(never)]
    pub(super) fn exec_get_global(&mut self, program: &Program, index: u32) -> Result<(), String> {
        let Some(name) = constant_string(program, index) else {
            return Err(format!("constant index {index} out of range"));
        };
        let var = crate::core::vm_resolve_global(name)?;
        self.stack.push(var.deref_value());
        Ok(())
    }

    #[inline(never)]
    pub(super) fn exec_def_global(
        &mut self,
        program: &Program,
        name: u32,
        metadata: Option<u16>,
    ) -> Result<(), String> {
        let Some(name) = constant_string(program, name) else {
            return Err(format!("constant index {name} out of range"));
        };
        let Some(value) = self.stack.pop() else {
            return Err("stack underflow".to_string());
        };
        let metadata = metadata.map(|index| program.var_metadata[usize::from(index)].clone());
        crate::core::vm_def_global(name, value.clone(), metadata)?;
        self.stack.push(value);
        Ok(())
    }

    #[inline(never)]
    pub(super) fn exec_set_global(&mut self, program: &Program, index: u32) -> Result<(), String> {
        let Some(name) = constant_string(program, index) else {
            return Err(format!("constant index {index} out of range"));
        };
        let Some(value) = self.stack.pop() else {
            return Err("stack underflow".to_string());
        };
        let var = crate::core::namespace_registry().and_then(|registry| {
            registry
                .resolve(&Symbol::parse(name))
                .ok_or_else(|| format!("unbound var: {name}"))
        })?;
        if !crate::core::binding_is_local(&var) {
            return Err(format!(
                "Cannot replace referred Var without ns omission: {name}"
            ));
        }
        var.reset_value(value.clone());
        self.stack.push(value);
        Ok(())
    }

    #[inline(never)]
    pub(super) fn exec_var_global(&mut self, program: &Program, index: u32) -> Result<(), String> {
        let Some(name) = constant_string(program, index) else {
            return Err(format!("constant index {index} out of range"));
        };
        let var = crate::core::vm_resolve_global(name)?;
        self.stack.push(Value::Var(var));
        Ok(())
    }

    #[inline(never)]
    pub(super) fn exec_declare_global(
        &mut self,
        program: &Program,
        index: u32,
    ) -> Result<(), String> {
        let Some(name) = constant_string(program, index) else {
            return Err(format!("constant index {index} out of range"));
        };
        crate::core::vm_declare_global(name)?;
        self.stack.push(Value::Nil);
        Ok(())
    }

    #[inline(never)]
    pub(super) fn exec_def_struct(
        &mut self,
        program: &Program,
        name: u32,
        fields: u32,
    ) -> Result<(), String> {
        let Some(name) = constant_string(program, name) else {
            return Err(format!("constant index {name} out of range"));
        };
        let Some(field_names) = constant_string_vector(program, fields) else {
            return Err(format!("constant index {fields} out of range"));
        };
        let value = crate::core::vm_defstruct(name, field_names)?;
        self.stack.push(value);
        Ok(())
    }

    #[inline(never)]
    pub(super) fn exec_struct_field(
        &mut self,
        program: &Program,
        index: u32,
    ) -> Result<(), String> {
        let Some(field) = constant_string(program, index) else {
            return Err(format!("constant index {index} out of range"));
        };
        let Some(value) = self.stack.pop() else {
            return Err("stack underflow".to_string());
        };
        let value = crate::core::struct_field_value(&value, field)?;
        self.stack.push(value);
        Ok(())
    }

    #[inline(never)]
    pub(super) fn exec_instance_of(&mut self) -> Result<(), String> {
        let Some(value) = self.stack.pop() else {
            return Err("stack underflow".to_string());
        };
        let Some(ty) = self.stack.pop() else {
            return Err("stack underflow".to_string());
        };
        let value = crate::core::struct_instance_of(&ty, &value)?;
        self.stack.push(value);
        Ok(())
    }

    #[inline(never)]
    pub(super) fn exec_make_multi_arity(
        &mut self,
        program: &Program,
        name: u32,
        count: u8,
    ) -> Result<(), String> {
        let count = usize::from(count);
        if self.stack.len() < count {
            return Err("stack underflow".to_string());
        }
        let Some(name) = constant_string(program, name).map(str::to_owned) else {
            return Err(format!("constant index {name} out of range"));
        };
        let start = self.stack.len() - count;
        if self.stack[start..]
            .iter()
            .any(|value| !matches!(value, Value::Function(_)))
        {
            return Err("multi-arity clauses must be functions".to_string());
        }
        let functions = self
            .stack
            .drain(start..)
            .map(|value| match value {
                Value::Function(function) => function,
                _ => unreachable!("checked above"),
            })
            .collect();
        self.stack
            .push(crate::core::arity_dispatcher(&name, functions, false));
        Ok(())
    }
}
