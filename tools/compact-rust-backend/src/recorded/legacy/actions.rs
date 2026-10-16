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

//! Ordered legacy action dispatch; specialized domains retain their admission rules.
//!
//! This module preserves the legacy lowering semantics. Explicit arguments own
//! scopes, temporary identifiers, declarations, and the current recording frame.

use crate::RenderError;
use crate::ir::{LedgerField, PureCircuit, StateAction, StatefulCircuit, Type, WitnessDeclaration};
use crate::recorded::legacy::actions::assertions::append_assertion_steps;
use crate::recorded::legacy::actions::bindings::append_binding_steps;
use crate::recorded::legacy::actions::collections::append_collection_steps;
use crate::recorded::legacy::actions::control::append_control_steps;
use crate::recorded::legacy::actions::merkle::append_merkle_steps;
use crate::recorded::{LegacyActionPrecision, RecordingOutcome, unavailable_action};
use std::collections::{HashMap, HashSet};

mod assertions;
mod bindings;
mod collections;
mod control;
mod merkle;

#[expect(
    clippy::too_many_arguments,
    reason = "recursive action lowering keeps typed declarations and ordered recording state explicit"
)]
pub(in crate::recorded) fn append_steps(
    action: &StateAction,
    path: &str,
    locals: &HashMap<String, syn::Expr>,
    parameters: &HashMap<&str, (&Type, syn::Ident)>,
    ledger_fields: &HashMap<&str, &LedgerField>,
    witnesses: &HashMap<&str, &WitnessDeclaration>,
    pure_circuits: &HashMap<&str, &PureCircuit>,
    circuits: &HashMap<&str, &StatefulCircuit>,
    shared_callees: &HashSet<String>,
    steps: &mut Vec<syn::Stmt>,
    next_temp: &mut usize,
    visiting: &mut HashSet<String>,
    failure_precision: &mut LegacyActionPrecision,
) -> Result<RecordingOutcome<()>, RenderError> {
    match action {
        StateAction::Sequence { .. } | StateAction::If { .. } | StateAction::CircuitCall { .. } => {
            append_control_steps(
                action,
                path,
                locals,
                parameters,
                ledger_fields,
                witnesses,
                pure_circuits,
                circuits,
                shared_callees,
                steps,
                next_temp,
                visiting,
                failure_precision,
            )
        }
        StateAction::Let { .. } => append_binding_steps(
            action,
            path,
            locals,
            parameters,
            ledger_fields,
            witnesses,
            pure_circuits,
            circuits,
            shared_callees,
            steps,
            next_temp,
            visiting,
            failure_precision,
        ),
        StateAction::Expression { .. }
        | StateAction::PureCall { .. }
        | StateAction::Assert { .. } => append_assertion_steps(
            action,
            path,
            locals,
            parameters,
            ledger_fields,
            witnesses,
            pure_circuits,
            circuits,
            steps,
            next_temp,
            visiting,
        ),
        StateAction::CounterIncrement { .. }
        | StateAction::CounterDecrement { .. }
        | StateAction::CounterReset { .. }
        | StateAction::SetInsert { .. }
        | StateAction::SetRemove { .. }
        | StateAction::SetReset { .. }
        | StateAction::ListPushFront { .. }
        | StateAction::ListPopFront { .. }
        | StateAction::ListReset { .. }
        | StateAction::MapInsert { .. }
        | StateAction::MapInsertDefault { .. }
        | StateAction::MapRemove { .. }
        | StateAction::MapReset { .. }
        | StateAction::CellWrite { .. } => append_collection_steps(
            action,
            path,
            locals,
            parameters,
            ledger_fields,
            witnesses,
            circuits,
            shared_callees,
            steps,
            next_temp,
            visiting,
        ),
        StateAction::HistoricMerkleResetHistory { .. }
        | StateAction::HistoricMerkleResetToDefault { .. }
        | StateAction::MerkleInsert { .. }
        | StateAction::HistoricMerkleInsert { .. }
        | StateAction::MerkleResetToDefault { .. }
        | StateAction::MerkleInsertHash { .. }
        | StateAction::HistoricMerkleInsertHash { .. }
        | StateAction::MerkleInsertHashIndex { .. }
        | StateAction::HistoricMerkleInsertHashIndex { .. }
        | StateAction::MerkleInsertIndex { .. }
        | StateAction::HistoricMerkleInsertIndex { .. }
        | StateAction::MerkleInsertIndexDefault { .. }
        | StateAction::HistoricMerkleInsertIndexDefault { .. } => {
            append_merkle_steps(action, path, locals, parameters, ledger_fields, steps)
        }
        _ => Ok(unavailable_action(action, path)),
    }
}
