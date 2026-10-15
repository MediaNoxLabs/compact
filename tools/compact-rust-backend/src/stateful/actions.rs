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
use crate::stateful::expression::NativeExpressionContext;
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
    let environment = NativeExpressionContext {
        witnesses,
        circuits,
        stateful_circuits,
        ledger_fields,
        observer: None,
    };
    lower_observed(
        action,
        parameters,
        &environment,
        ActionOutput {
            statements,
            next_temp,
            uses_witness,
        },
    )
}

pub(super) struct ActionOutput<'output> {
    pub(super) statements: &'output mut Vec<syn::Stmt>,
    pub(super) next_temp: &'output mut usize,
    pub(super) uses_witness: &'output mut bool,
}

/// Route one original action; Counter/Cell, collections, coins and Merkle retain borrowed source observation.
pub(super) fn lower_observed(
    action: &StateAction,
    parameters: &HashMap<&str, (&Type, syn::Ident)>,
    environment: &NativeExpressionContext<'_, '_>,
    output: ActionOutput<'_>,
) -> Result<(), RenderError> {
    let ActionOutput {
        statements,
        next_temp,
        uses_witness,
    } = output;
    let ledger_fields = environment.ledger_fields;
    let witnesses = environment.witnesses;
    let circuits = environment.circuits;
    let stateful_circuits = environment.stateful_circuits;
    match action {
        StateAction::NativeWitnessCall {
            builtin: NativeWitnessBuiltin::OwnPublicKey,
        }
        | StateAction::Expression { .. }
        | StateAction::PureCall { .. }
        | StateAction::CircuitCall { .. }
        | StateAction::Assert { .. } => {
            if environment.observer.is_some() {
                effects::lower_observed(
                    action,
                    parameters,
                    environment,
                    ActionOutput {
                        statements,
                        next_temp,
                        uses_witness,
                    },
                )
            } else {
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
                )
            }?;
        }
        StateAction::Let { .. } | StateAction::Sequence { .. } | StateAction::If { .. } => {
            unreachable!("control actions were expanded before rendering")
        }
        StateAction::CounterIncrement { .. }
        | StateAction::CounterDecrement { .. }
        | StateAction::CounterReset { .. }
        | StateAction::CellWrite { .. } => {
            if environment.observer.is_some() {
                cells_counters::lower_observed(
                    action,
                    parameters,
                    environment,
                    ActionOutput {
                        statements,
                        next_temp,
                        uses_witness,
                    },
                )
            } else {
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
                )
            }?;
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
            if environment.observer.is_some() {
                collections::lower_observed(
                    action,
                    parameters,
                    environment,
                    ActionOutput {
                        statements,
                        next_temp,
                        uses_witness,
                    },
                )
            } else {
                collections::lower(action, parameters, ledger_fields, circuits, statements)
            }?;
        }
        StateAction::SetInsertCoin { .. } | StateAction::CellWriteCoin { .. } => {
            if environment.observer.is_some() {
                coins::lower_observed(
                    action,
                    parameters,
                    environment,
                    ActionOutput {
                        statements,
                        next_temp,
                        uses_witness,
                    },
                )
            } else {
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
                )
            }?;
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
            if environment.observer.is_some() {
                merkle::lower_observed(
                    action,
                    parameters,
                    environment,
                    ActionOutput {
                        statements,
                        next_temp,
                        uses_witness,
                    },
                )
            } else {
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
                )
            }?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod diagnostic_tests;

#[cfg(test)]
mod workers;

#[cfg(test)]
mod collection_diagnostic_tests;

#[cfg(test)]
mod coin_merkle_diagnostic_tests;

#[cfg(test)]
mod effect_diagnostic_tests;
