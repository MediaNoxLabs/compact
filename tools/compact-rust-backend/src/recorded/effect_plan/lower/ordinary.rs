// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Ordinary expression returns retain only their terminal lexical continuation.
use super::*;

impl Lowerer<'_> {
    pub(super) fn ordinary(
        &mut self,
        actions: &[StateAction],
        returned: &StateReturn,
        scope: &Scope,
        bindings: &mut Vec<Binding>,
    ) -> Result<Value, RecordingGap> {
        let return_scope = self.terminal_actions(actions, scope, bindings, "actions")?;
        match returned {
            StateReturn::Expression { value } => {
                self.expression(value, &return_scope, bindings, "return_value.value")
            }
            StateReturn::Unit => Ok(self.emit(
                Operation::Literal(Literal::Unit),
                Type::Unit,
                bindings,
                "return_value",
            )),
            // Specialized ledger returns use the original parameter scope, as native does.
            StateReturn::CellRead { field, index } => self.expression(
                &Expr::CellRead {
                    field: field.clone(),
                    index: *index,
                },
                scope,
                bindings,
                "return_value",
            ),
            StateReturn::CounterRead { field, index } => self.expression(
                &Expr::CounterRead {
                    field: field.clone(),
                    index: *index,
                },
                scope,
                bindings,
                "return_value",
            ),
            _ => Err(RecordingGap::returned(returned)),
        }
    }

    /// Earlier siblings restore their lexical scopes before the continuation.
    fn terminal_actions(
        &mut self,
        actions: &[StateAction],
        scope: &Scope,
        bindings: &mut Vec<Binding>,
        path: &str,
    ) -> Result<Scope, RecordingGap> {
        let Some((last, prefix)) = actions.split_last() else {
            return Ok(scope.clone());
        };
        for (i, action) in prefix.iter().enumerate() {
            self.action(action, scope, bindings, &format!("{path}[{i}]"))?;
        }
        self.terminal_action(last, scope, bindings, &format!("{path}[{}]", prefix.len()))
    }

    /// Follow only final Sequence children and Let bodies; If branches stay local.
    fn terminal_action(
        &mut self,
        action: &StateAction,
        scope: &Scope,
        bindings: &mut Vec<Binding>,
        path: &str,
    ) -> Result<Scope, RecordingGap> {
        match action {
            StateAction::Sequence { actions } => {
                self.terminal_actions(actions, scope, bindings, &format!("{path}.actions"))
            }
            StateAction::Let {
                bindings: locals,
                action,
            } => {
                let scope = self.locals(locals, scope, bindings, path)?;
                self.terminal_action(action, &scope, bindings, &format!("{path}.action"))
            }
            _ => {
                self.action(action, scope, bindings, path)?;
                Ok(scope.clone())
            }
        }
    }
}
