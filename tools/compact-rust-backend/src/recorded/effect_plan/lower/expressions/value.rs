// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Original early-return owners: no extra validation or SSA binding.
use super::*;

impl Lowerer<'_> {
    pub(super) fn parameter_value(
        &mut self,
        expression: &Expr,
        scope: &Scope,
        _bindings: &mut Vec<Binding>,
        path: &str,
    ) -> Result<Value, RecordingGap> {
        if let Expr::Parameter { name } = expression {
            return scope
                .get(name)
                .cloned()
                .ok_or_else(|| invalid(path, "unbound value"));
        }
        unreachable!("expression dispatcher selects parameter_value");
    }
    pub(super) fn control_value(
        &mut self,
        expression: &Expr,
        scope: &Scope,
        bindings: &mut Vec<Binding>,
        path: &str,
    ) -> Result<Value, RecordingGap> {
        match expression {
            Expr::Let {
                bindings: locals,
                body,
            } => {
                let scope = self.locals(locals, scope, bindings, path)?;
                return self.expression(body, &scope, bindings, &format!("{path}.body"));
            }
            Expr::Sequence { steps, value } => {
                for (i, step) in steps.iter().enumerate() {
                    let v =
                        self.expression(step, scope, bindings, &format!("{path}.steps[{i}]"))?;
                    if v.ty != Type::Unit {
                        return Err(invalid(
                            &format!("{path}.steps[{i}]"),
                            "sequence step must be Unit",
                        ));
                    }
                }
                return self.expression(value, scope, bindings, &format!("{path}.value"));
            }
            Expr::If {
                condition,
                then,
                otherwise,
            } => {
                return self.conditional(
                    condition,
                    scope,
                    bindings,
                    path,
                    |lower, steps, child_path, is_then| {
                        lower.expression(
                            if is_then { then } else { otherwise },
                            scope,
                            steps,
                            child_path,
                        )
                    },
                );
            }
            _ => {}
        }
        unreachable!("expression dispatcher selects control_value");
    }
    pub(super) fn conversion_value(
        &mut self,
        expression: &Expr,
        scope: &Scope,
        bindings: &mut Vec<Binding>,
        path: &str,
    ) -> Result<Value, RecordingGap> {
        match expression {
            Expr::Coerce { value, ty } => {
                let value = self.expression(value, scope, bindings, &format!("{path}.value"))?;
                return self.coerce(value, ty, bindings, path);
            }
            Expr::FieldCast { value } => {
                let value = self.expression(value, scope, bindings, &format!("{path}.value"))?;
                if validate::unsigned_bound(&value.ty).is_none() {
                    return Err(invalid(path, "Field cast requires unsigned input"));
                }
                return self.coerce(value, &Type::Field, bindings, path);
            }
            _ => {}
        }
        unreachable!("expression dispatcher selects conversion_value");
    }
    pub(super) fn call_value(
        &mut self,
        expression: &Expr,
        scope: &Scope,
        bindings: &mut Vec<Binding>,
        path: &str,
    ) -> Result<Value, RecordingGap> {
        if let Expr::Call { name, arguments } = expression {
            return self.call(
                calls::CallSite::Expression(expression),
                name,
                arguments,
                scope,
                bindings,
                path,
            );
        }
        unreachable!("expression dispatcher selects call_value");
    }
}
