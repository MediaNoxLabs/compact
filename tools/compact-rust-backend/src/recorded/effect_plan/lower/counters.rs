// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Counter mutation operands are resolved in lexical scope, before frame effects.
use super::*;

impl Lowerer<'_> {
    pub(super) fn counter_action(
        &mut self,
        action: &StateAction,
        scope: &Scope,
        bindings: &mut Vec<Binding>,
        path: &str,
    ) -> Result<(), RecordingGap> {
        let operation = match action {
            StateAction::CounterIncrement {
                field,
                index,
                amount,
            }
            | StateAction::CounterDecrement {
                field,
                index,
                amount,
            } => {
                let slot = self.counter(field, *index, path)?;
                let amount =
                    self.counter_amount(amount, scope, bindings, &format!("{path}.amount"))?;
                if matches!(action, StateAction::CounterIncrement { .. }) {
                    Operation::CounterIncrement { slot, amount }
                } else {
                    Operation::CounterDecrement { slot, amount }
                }
            }
            StateAction::CounterReset { field, index } => Operation::CounterReset {
                slot: self.counter(field, *index, path)?,
            },
            _ => return Err(RecordingGap::action(action, path.into())),
        };
        self.emit(operation, Type::Unit, bindings, path);
        Ok(())
    }

    fn counter_amount(
        &mut self,
        amount: &CounterAmount,
        scope: &Scope,
        bindings: &mut Vec<Binding>,
        path: &str,
    ) -> Result<Value, RecordingGap> {
        let amount = match amount {
            CounterAmount::Literal { value } => self.emit(
                Operation::Literal(Literal::Unsigned(u128::from(*value))),
                uint("65535"),
                bindings,
                path,
            ),
            CounterAmount::Parameter { name } => scope
                .get(name)
                .cloned()
                .ok_or_else(|| invalid(path, "unbound counter amount"))?,
        };
        if amount.ty != uint("65535") {
            return Err(invalid(path, "counter amount must be Uint<16>"));
        }
        Ok(amount)
    }
}
