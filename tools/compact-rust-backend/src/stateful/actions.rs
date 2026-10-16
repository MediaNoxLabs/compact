// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//  	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Exhaustive native leaf-action routing after the traversal expands control nodes.
//!
//! Pending actions, ReturnPlan scheduling, branch frames, and lexical restoration
//! remain owned by the traversal. Domain functions receive its current parameter
//! snapshot and explicit statement, temporary, and witness state.

use crate::RenderError;
use crate::ir::{
    LedgerField, NativeWitnessBuiltin, PureCircuit, StateAction, StatefulCircuit, Type,
    WitnessDeclaration,
};
use std::collections::HashMap;

mod cells_counters;
mod coins;
mod collections;
mod effects;
mod merkle;

#[expect(
    clippy::too_many_arguments,
    reason = "native action dispatch preserves explicit declaration scope and ordered effect state"
)]
pub(super) fn lower(
    action: &StateAction,
    parameters: &HashMap<&str, (&Type, syn::Ident)>,
    ledger_fields: &HashMap<&str, &LedgerField>,
    witnesses: &HashMap<&str, &WitnessDeclaration>,
    circuits: &HashMap<&str, &PureCircuit>,
    stateful_circuits: &HashMap<&str, &StatefulCircuit>,
    statements: &mut Vec<syn::Stmt>,
    next_temp: &mut usize,
    uses_witness: &mut bool,
) -> Result<(), RenderError> {
    match action {
        StateAction::NativeWitnessCall {
            builtin: NativeWitnessBuiltin::OwnPublicKey,
        }
        | StateAction::Expression { .. }
        | StateAction::PureCall { .. }
        | StateAction::CircuitCall { .. }
        | StateAction::Assert { .. } => {
            effects::lower(
                action,
                parameters,
                ledger_fields,
                witnesses,
                circuits,
                stateful_circuits,
                statements,
                next_temp,
                uses_witness,
            )?;
        }
        StateAction::Let { .. } | StateAction::Sequence { .. } | StateAction::If { .. } => {
            unreachable!("control actions were expanded before rendering")
        }
        StateAction::CounterIncrement { .. }
        | StateAction::CounterDecrement { .. }
        | StateAction::CounterReset { .. }
        | StateAction::CellWrite { .. } => {
            cells_counters::lower(
                action,
                parameters,
                ledger_fields,
                witnesses,
                circuits,
                stateful_circuits,
                statements,
                next_temp,
                uses_witness,
            )?;
        }
        StateAction::ListPushFront { .. }
        | StateAction::ListPopFront { .. }
        | StateAction::ListReset { .. }
        | StateAction::SetInsert { .. }
        | StateAction::SetRemove { .. }
        | StateAction::SetReset { .. }
        | StateAction::MapInsert { .. }
        | StateAction::MapInsertDefault { .. }
        | StateAction::MapRemove { .. }
        | StateAction::MapReset { .. } => {
            collections::lower(action, parameters, ledger_fields, circuits, statements)?;
        }
        StateAction::SetInsertCoin { .. } | StateAction::CellWriteCoin { .. } => {
            coins::lower(
                action,
                parameters,
                ledger_fields,
                witnesses,
                circuits,
                stateful_circuits,
                statements,
                next_temp,
                uses_witness,
            )?;
        }
        StateAction::HistoricMerkleResetHistory { .. }
        | StateAction::HistoricMerkleResetToDefault { .. }
        | StateAction::MerkleResetToDefault { .. }
        | StateAction::HistoricMerkleInsert { .. }
        | StateAction::MerkleInsert { .. }
        | StateAction::HistoricMerkleInsertIndex { .. }
        | StateAction::MerkleInsertIndex { .. }
        | StateAction::HistoricMerkleInsertHash { .. }
        | StateAction::MerkleInsertHash { .. }
        | StateAction::HistoricMerkleInsertHashIndex { .. }
        | StateAction::MerkleInsertHashIndex { .. }
        | StateAction::HistoricMerkleInsertIndexDefault { .. }
        | StateAction::MerkleInsertIndexDefault { .. } => {
            merkle::lower(
                action,
                parameters,
                ledger_fields,
                witnesses,
                circuits,
                stateful_circuits,
                statements,
                next_temp,
                uses_witness,
            )?;
        }
    }
    Ok(())
}
