// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Source-order lowering into typed blocks; unsupported children retain their paths.
use super::{
    RecordingGap,
    declarations::Declarations,
    invalid,
    model::*,
    validate::{scalar, uint},
};
use crate::ir::*;
use std::collections::HashMap;
mod expressions;

type Scope = HashMap<String, Value>;
struct Lowerer<'a> {
    declarations: &'a Declarations,
    next: usize,
}

pub(super) fn lower(
    circuit: &StatefulCircuit,
    declarations: &Declarations,
) -> Result<Plan, RecordingGap> {
    if !scalar(&circuit.result) {
        return Err(invalid("return_value", "unsupported circuit result type"));
    }
    let mut lower = Lowerer {
        declarations,
        next: 0,
    };
    let mut scope = Scope::new();
    let mut parameters = Vec::new();
    for (i, p) in circuit.parameters.iter().enumerate() {
        if !scalar(&p.ty) {
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
    let value = match &circuit.return_value {
        StateReturn::Effectful { body } => {
            if !circuit.actions.is_empty() {
                return Err(invalid("actions", "effectful return must own all actions"));
            }
            lower.returned(body, &scope, &mut bindings, "return_value.body")?
        }
        returned => {
            for (i, action) in circuit.actions.iter().enumerate() {
                lower.action(action, &scope, &mut bindings, &format!("actions[{i}]"))?;
            }
            match returned {
                StateReturn::Unit => lower.emit(
                    Operation::Literal(Literal::Unit),
                    Type::Unit,
                    &mut bindings,
                    "return_value",
                ),
                StateReturn::Expression { value } => {
                    lower.expression(value, &scope, &mut bindings, "return_value.value")?
                }
                StateReturn::CellRead { field, index } => lower.expression(
                    &Expr::CellRead {
                        field: field.clone(),
                        index: *index,
                    },
                    &scope,
                    &mut bindings,
                    "return_value",
                )?,
                StateReturn::CounterRead { field, index } => lower.expression(
                    &Expr::CounterRead {
                        field: field.clone(),
                        index: *index,
                    },
                    &scope,
                    &mut bindings,
                    "return_value",
                )?,
                _ => return Err(RecordingGap::returned(returned)),
            }
        }
    };
    let result = lower.coerce(value, &circuit.result, &mut bindings, "return_value")?;
    Ok(Plan {
        parameters,
        body: Block { bindings, result },
        result_type: circuit.result.clone(),
    })
}
impl Lowerer<'_> {
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
        if value.ty == *ty {
            return Ok(value);
        }
        let valid = scalar(ty)
            && ((super::validate::unsigned_bound(&value.ty).is_some() && *ty == Type::Field)
                || matches!((super::validate::bound(&value.ty), super::validate::bound(ty)), (Some(a),Some(b)) if a <= b));
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
        if !scalar(ty) || *ty == Type::Unit {
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
        if !scalar(&ty) {
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
            StateAction::CounterIncrement {
                field,
                index,
                amount,
            } => {
                let slot = self.counter(field, *index, path)?;
                let amount = match amount {
                    CounterAmount::Literal { value } => self.emit(
                        Operation::Literal(Literal::Unsigned(u128::from(*value))),
                        uint("65535"),
                        bindings,
                        &format!("{path}.amount"),
                    ),
                    CounterAmount::Parameter { name } => {
                        scope.get(name).cloned().ok_or_else(|| {
                            invalid(&format!("{path}.amount"), "unbound counter amount")
                        })?
                    }
                };
                if amount.ty != uint("65535") {
                    return Err(invalid(
                        &format!("{path}.amount"),
                        "counter amount must be Uint<16>",
                    ));
                }
                self.emit(
                    Operation::CounterIncrement { slot, amount },
                    Type::Unit,
                    bindings,
                    path,
                );
            }
            _ => return Err(RecordingGap::action(action, path.into())),
        }
        Ok(())
    }
}
