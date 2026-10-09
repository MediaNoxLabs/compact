// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Source-order lowering into typed blocks; unsupported children retain their paths.
use super::{
    RecordingGap,
    declarations::Declarations,
    invalid,
    model::*,
    validate::{cell_type, surface_type, uint},
};
use crate::ir::*;
use std::collections::HashMap;
mod aggregates;
mod calls;
mod counters;
mod expressions;
mod jubjub;
mod kernel;
mod ordinary;
mod pure;
mod zswap;
use super::pure_certification::{Certificates, Certifier};

type Scope = HashMap<String, Value>;
struct Lowerer<'a> {
    declarations: &'a Declarations,
    circuits: &'a HashMap<&'a str, &'a StatefulCircuit>,
    pure: Certifier<'a>,
    call_stack: Vec<String>,
    expanded_steps: usize,
    max_call_depth: usize,
    max_expanded_steps: usize,
    next: usize,
}

#[cfg(test)]
pub(super) fn lower_with_calls<'a>(
    circuit: &StatefulCircuit,
    declarations: &'a Declarations,
    circuits: &'a HashMap<&'a str, &'a StatefulCircuit>,
) -> Result<Plan, RecordingGap> {
    lower_with_pure_calls(circuit, declarations, &HashMap::new(), circuits).map(|(plan, _)| plan)
}

pub(super) fn lower_with_pure_calls<'a>(
    circuit: &StatefulCircuit,
    declarations: &'a Declarations,
    pure: &'a HashMap<&'a str, &'a PureCircuit>,
    circuits: &'a HashMap<&'a str, &'a StatefulCircuit>,
) -> Result<(Plan, Certificates), RecordingGap> {
    declarations
        .types()
        .validate_at(&circuit.result, "return_value")?;
    if !surface_type(&circuit.result) {
        return Err(invalid("return_value", "unsupported circuit result type"));
    }
    let (max_call_depth, max_expanded_steps) = crate::resource_limits::recording_call_limits();
    let mut lower = Lowerer {
        declarations,
        circuits,
        pure: Certifier::new(pure, circuits),
        call_stack: vec![circuit.name.clone()],
        expanded_steps: 0,
        max_call_depth,
        max_expanded_steps,
        next: 0,
    };
    let mut scope = Scope::new();
    let mut parameters = Vec::new();
    for (i, p) in circuit.parameters.iter().enumerate() {
        lower.spend(&format!("parameters[{i}]"))?;
        declarations
            .types()
            .validate_at(&p.ty, &format!("parameters[{i}]"))?;
        if !surface_type(&p.ty) {
            return Err(invalid(
                &format!("parameters[{i}]"),
                "unsupported parameter type",
            ));
        }
        let value = lower.fresh(p.ty.clone());
        if scope.insert(p.name.clone(), value.clone()).is_some() {
            return Err(invalid(&format!("parameters[{i}]"), "duplicate parameter"));
        }
        parameters.push(value);
    }
    let mut bindings = Vec::new();
    let result = lower.circuit_body(circuit, &scope, &mut bindings, "")?;
    let plan = Plan {
        parameters,
        body: Block { bindings, result },
        result_type: circuit.result.clone(),
    };
    Ok((plan, lower.pure.into_certificates()))
}

impl Lowerer<'_> {
    /// Defensive fuel for direct candidate callers as well as public preflight.
    /// Visits are charged before expansion; shared DAG descendants are charged
    /// for every actual invocation, not once per declaration.
    fn spend(&mut self, path: &str) -> Result<(), RecordingGap> {
        if self.expanded_steps >= self.max_expanded_steps {
            return Err(invalid(
                path,
                format!(
                    "recording expansion work exceeds {}",
                    self.max_expanded_steps
                ),
            ));
        }
        self.expanded_steps += 1;
        Ok(())
    }
    fn circuit_body(
        &mut self,
        circuit: &StatefulCircuit,
        scope: &Scope,
        bindings: &mut Vec<Binding>,
        base: &str,
    ) -> Result<Value, RecordingGap> {
        let returned_path = path_at(base, "return_value");
        let value = match &circuit.return_value {
            StateReturn::Effectful { body } => {
                if !circuit.actions.is_empty() {
                    return Err(invalid(
                        &path_at(base, "actions"),
                        "effectful return must own all actions",
                    ));
                }
                self.returned(body, scope, bindings, &format!("{returned_path}.body"))?
            }
            returned => self.ordinary(&circuit.actions, returned, scope, bindings, base)?,
        };
        self.coerce(value, &circuit.result, bindings, &returned_path)
    }

    fn fresh(&mut self, ty: Type) -> Value {
        let id = self.next;
        self.next += 1;
        Value { id, ty }
    }
    fn emit(
        &mut self,
        operation: Operation,
        ty: Type,
        bindings: &mut Vec<Binding>,
        path: &str,
    ) -> Value {
        let value = self.fresh(ty);
        bindings.push(Binding {
            value: value.clone(),
            operation,
            path: path.into(),
        });
        value
    }
    fn coerce(
        &mut self,
        value: Value,
        ty: &Type,
        bindings: &mut Vec<Binding>,
        path: &str,
    ) -> Result<Value, RecordingGap> {
        self.declarations.types().validate_at(&value.ty, path)?;
        self.declarations.types().validate_at(ty, path)?;
        if value.ty == *ty {
            return Ok(value);
        }
        let valid = super::validate::coercible(&value.ty, ty);
        if !valid {
            return Err(invalid(
                path,
                format!("cannot coerce {:?} to {ty:?}", value.ty),
            ));
        }
        Ok(self.emit(
            Operation::Convert {
                value,
                kind: Conversion::Coerce,
            },
            ty.clone(),
            bindings,
            path,
        ))
    }
    fn field(
        &self,
        name: &str,
        index: u8,
        path: &str,
    ) -> Result<(LedgerId, &LedgerField), RecordingGap> {
        self.declarations
            .resolve_field(name)
            .filter(|(_, f)| f.index == index)
            .ok_or_else(|| invalid(path, "unknown ledger field or inconsistent index"))
    }
    fn cell(&self, name: &str, index: u8, path: &str) -> Result<(CellSlot, Type), RecordingGap> {
        let (declaration, field) = self.field(name, index, path)?;
        let LedgerFieldKind::Cell { ty } = &field.declaration else {
            return Err(invalid(path, "expected Cell field"));
        };
        self.declarations.types().validate_at(ty, path)?;
        if !cell_type(ty) {
            return Err(invalid(path, "unsupported Cell value type"));
        }
        Ok((CellSlot { declaration, index }, ty.clone()))
    }
    fn counter(&self, name: &str, index: u8, path: &str) -> Result<CounterSlot, RecordingGap> {
        let (declaration, field) = self.field(name, index, path)?;
        if field.declaration != LedgerFieldKind::Counter {
            return Err(invalid(path, "expected Counter field"));
        }
        Ok(CounterSlot { declaration, index })
    }
    fn locals(
        &mut self,
        locals: &[LocalBinding],
        scope: &Scope,
        bindings: &mut Vec<Binding>,
        path: &str,
    ) -> Result<Scope, RecordingGap> {
        let mut scope = scope.clone();
        for (i, local) in locals.iter().enumerate() {
            let path = format!("{path}.bindings[{i}].value");
            let value = self.expression(&local.value, &scope, bindings, &path)?;
            let value = self.coerce(value, &local.ty, bindings, &path)?;
            scope.insert(local.name.clone(), value);
        }
        Ok(scope)
    }
    fn returned(
        &mut self,
        plan: &ReturnPlan,
        scope: &Scope,
        bindings: &mut Vec<Binding>,
        path: &str,
    ) -> Result<Value, RecordingGap> {
        self.spend(path)?;
        match plan {
            ReturnPlan::Value { value } => {
                self.expression(value, scope, bindings, &format!("{path}.value"))
            }
            ReturnPlan::Sequence { actions, result } => {
                for (i, a) in actions.iter().enumerate() {
                    self.action(a, scope, bindings, &format!("{path}.actions[{i}]"))?;
                }
                self.returned(result, scope, bindings, &format!("{path}.result"))
            }
            ReturnPlan::Let {
                bindings: locals,
                result,
            } => {
                let scope = self.locals(locals, scope, bindings, path)?;
                self.returned(result, &scope, bindings, &format!("{path}.result"))
            }
            ReturnPlan::Conditional {
                condition,
                then,
                otherwise,
            } => self.conditional(
                condition,
                scope,
                bindings,
                path,
                |lower, steps, child_path, is_then| {
                    lower.returned(
                        if is_then { then } else { otherwise },
                        scope,
                        steps,
                        child_path,
                    )
                },
            ),
        }
    }
    fn conditional<F>(
        &mut self,
        condition: &Expr,
        scope: &Scope,
        bindings: &mut Vec<Binding>,
        path: &str,
        mut child: F,
    ) -> Result<Value, RecordingGap>
    where
        F: FnMut(&mut Self, &mut Vec<Binding>, &str, bool) -> Result<Value, RecordingGap>,
    {
        let condition =
            self.expression(condition, scope, bindings, &format!("{path}.condition"))?;
        if condition.ty != Type::Boolean {
            return Err(invalid(
                &format!("{path}.condition"),
                "expected Boolean condition",
            ));
        }
        let mut then_bindings = Vec::new();
        let then_result = child(self, &mut then_bindings, &format!("{path}.then"), true)?;
        let mut else_bindings = Vec::new();
        let else_result = child(
            self,
            &mut else_bindings,
            &format!("{path}.otherwise"),
            false,
        )?;
        if then_result.ty != else_result.ty {
            return Err(invalid(path, "branch result types differ"));
        }
        let ty = then_result.ty.clone();
        if !super::validate::value_type(&ty) {
            return Err(invalid(path, "unsupported branch result type"));
        }
        Ok(self.emit(
            Operation::If {
                condition,
                then: Block {
                    bindings: then_bindings,
                    result: then_result,
                },
                otherwise: Block {
                    bindings: else_bindings,
                    result: else_result,
                },
            },
            ty,
            bindings,
            path,
        ))
    }
    fn action(
        &mut self,
        action: &StateAction,
        scope: &Scope,
        bindings: &mut Vec<Binding>,
        path: &str,
    ) -> Result<(), RecordingGap> {
        self.spend(path)?;
        match action {
            StateAction::Sequence { actions } => {
                for (i, a) in actions.iter().enumerate() {
                    self.action(a, scope, bindings, &format!("{path}.actions[{i}]"))?;
                }
            }
            StateAction::Let {
                bindings: locals,
                action,
            } => {
                let scope = self.locals(locals, scope, bindings, path)?;
                self.action(action, &scope, bindings, &format!("{path}.action"))?;
            }
            StateAction::If {
                condition,
                then,
                otherwise,
            } => {
                self.conditional(
                    condition,
                    scope,
                    bindings,
                    path,
                    |lower, steps, child_path, is_then| {
                        lower.action(
                            if is_then { then } else { otherwise },
                            scope,
                            steps,
                            child_path,
                        )?;
                        Ok(lower.emit(
                            Operation::Literal(Literal::Unit),
                            Type::Unit,
                            steps,
                            child_path,
                        ))
                    },
                )?;
            }
            StateAction::Expression { value } => {
                self.expression(value, scope, bindings, &format!("{path}.value"))?;
            }
            StateAction::CircuitCall { name, arguments } => {
                // Statement calls resolve only stateful declarations, just as native
                // lowering does. The shared call checker owns arguments and body;
                // discarding its result does not discard the body's ordered effects.
                self.call(
                    calls::CallSite::Statement(action),
                    name,
                    arguments,
                    scope,
                    bindings,
                    path,
                )?;
            }
            StateAction::Assert { condition, message } => {
                self.expression(
                    &Expr::Assert {
                        condition: Box::new(condition.clone()),
                        message: message.clone(),
                    },
                    scope,
                    bindings,
                    path,
                )?;
            }
            StateAction::CellWrite {
                field,
                index,
                value,
            } => {
                let (slot, ty) = self.cell(field, *index, path)?;
                let value = self.expression(value, scope, bindings, &format!("{path}.value"))?;
                let value = self.coerce(value, &ty, bindings, &format!("{path}.value"))?;
                self.emit(
                    Operation::CellWrite { slot, value },
                    Type::Unit,
                    bindings,
                    path,
                );
            }
            StateAction::CounterIncrement { .. }
            | StateAction::CounterDecrement { .. }
            | StateAction::CounterReset { .. } => {
                self.counter_action(action, scope, bindings, path)?;
            }
            _ => return Err(RecordingGap::action(action, path.into())),
        }
        Ok(())
    }
}

fn path_at(base: &str, field: &str) -> String {
    if base.is_empty() {
        field.into()
    } else {
        format!("{base}.{field}")
    }
}
