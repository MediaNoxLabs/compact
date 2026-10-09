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

//! Ordered organizer and typed-profile admission for recorded circuits.
//!
//! Organizer attempts intentionally use eager `or`: their validation errors
//! remain observable even when an earlier organizer returned steps. Typed
//! attempts use lazy `or_else` and stop at the first admitted plan. Rejections
//! from checked profiles are retained for later legacy diagnostic refinement;
//! a rejection does not prevent an independent later profile from admitting.
//!
//! Effectful-return admission is performed earlier by the caller, before helper
//! routing and parameter validation. Keep that ordering outside this selector.

use std::collections::HashMap;

use crate::RenderError;
use crate::ir::{LedgerField, PureCircuit, StatefulCircuit, WitnessDeclaration};

use super::RecordingGap;
use super::guarded_mutations::{
    closed_authorized_enum_advance_steps, closed_authorized_optional_write_steps,
    closed_organizer_gate_steps, closed_witness_admitted_merkle_insert_steps,
};
use super::profile_attempt::ProfileAttempt;
use super::typed_plan::{self, CompositionRejection, TypedPlan};
use super::{kernel_plan, zswap_plan};

/// An admitted step sequence and the evidence retained from declined profiles.
/// Legacy organizers have no typed result; absence of steps permits legacy
/// action lowering. The caller owns whether a retained gap can refine a failure.
pub(super) struct ProfileSelection {
    pub(super) organizer_steps: Option<Vec<syn::Stmt>>,
    pub(super) typed_result: Option<syn::Expr>,
    pub(super) shielded_send_rejection: Option<RecordingGap>,
    pub(super) composition_rejection: Option<CompositionRejection>,
}

pub(super) fn select(
    circuit: &StatefulCircuit,
    ledger_fields: &HashMap<&str, &LedgerField>,
    witnesses: &HashMap<&str, &WitnessDeclaration>,
    pure_circuits: &HashMap<&str, &PureCircuit>,
    circuits: &HashMap<&str, &StatefulCircuit>,
    effectful_plan: Option<TypedPlan>,
) -> Result<ProfileSelection, RenderError> {
    let mut typed_result = None;
    // A closed profile may find a precise nested rejection while a later
    // profile can still admit the same circuit. Retain it only for a coarse
    // generic return gap after every existing profile has declined.
    let mut shielded_send_rejection = None;
    let mut composition_rejection = None;
    let mut organizer_steps = if let Some(plan) = effectful_plan {
        typed_result = Some(plan.result);
        Some(plan.steps)
    } else {
        record_option!(
            "organizer.organizer",
            closed_organizer_gate_steps(
                circuit,
                ledger_fields,
                witnesses,
                pure_circuits,
                circuits
            )?
        )
        .or(record_option!(
            "organizer.optional-write",
            closed_authorized_optional_write_steps(
                circuit,
                ledger_fields,
                witnesses,
                pure_circuits,
            )?
        ))
        .or(record_option!(
            "organizer.enum-advance",
            closed_authorized_enum_advance_steps(circuit, ledger_fields, witnesses, pure_circuits,)?
        ))
        .or(record_option!(
            "organizer.witness-merkle",
            closed_witness_admitted_merkle_insert_steps(
                circuit,
                ledger_fields,
                witnesses,
                pure_circuits,
            )?
        ))
    };
    if organizer_steps.is_none()
        && let Some(plan) = record_option!(
            "selector.membership-optional",
            typed_plan::lower(circuit, ledger_fields, witnesses, pure_circuits, circuits)
        )
        .or_else(|| {
            record_option!(
                "selector.jubjub-values",
                typed_plan::lower_jubjub_values(
                    circuit,
                    ledger_fields,
                    witnesses,
                    pure_circuits,
                    circuits,
                )
            )
        })
        .or_else(|| record_option!("selector.kernel", kernel_plan::lower(circuit, witnesses)))
        .or_else(|| {
            record_option!(
                "selector.context-query",
                typed_plan::lower_context_query(circuit, ledger_fields, witnesses, pure_circuits,)
            )
        })
        .or_else(|| {
            record_option!(
                "selector.composite-values",
                typed_plan::lower_composite(
                    circuit,
                    ledger_fields,
                    witnesses,
                    pure_circuits,
                    circuits,
                )
            )
        })
        .or_else(|| {
            record_option!(
                "selector.shielded-receive",
                typed_plan::lower_shielded_receive(
                    circuit,
                    ledger_fields,
                    witnesses,
                    pure_circuits,
                    circuits,
                )
            )
        })
        .or_else(|| {
            record_option!(
                "selector.voting-commit",
                typed_plan::lower_voting_commit(
                    circuit,
                    ledger_fields,
                    witnesses,
                    pure_circuits,
                    circuits,
                )
            )
        })
        .or_else(|| {
            record_option!(
                "selector.funded-mint",
                typed_plan::lower_funded_mint(
                    circuit,
                    ledger_fields,
                    witnesses,
                    pure_circuits,
                    circuits,
                )
            )
        })
        .or_else(|| {
            record_option!(
                "selector.guarded-deposit",
                typed_plan::lower_guarded_deposit(
                    circuit,
                    ledger_fields,
                    witnesses,
                    pure_circuits,
                    circuits,
                )
            )
        })
        .or_else(|| {
            record_option!(
                "selector.shielded-merge",
                typed_plan::lower_shielded_merge(
                    circuit,
                    ledger_fields,
                    witnesses,
                    pure_circuits,
                    circuits,
                )
            )
        })
        .or_else(|| {
            record_option!(
                "selector.immediate-send",
                typed_plan::lower_immediate_shielded_send(
                    circuit,
                    ledger_fields,
                    witnesses,
                    pure_circuits,
                    circuits,
                )
            )
        })
        .or_else(|| {
            match typed_plan::lower_shielded_send_checked(
                circuit,
                ledger_fields,
                witnesses,
                pure_circuits,
                circuits,
            ) {
                ProfileAttempt::Admitted(plan) => {
                    record_option!("selector.shielded-send", Some(plan))
                }
                ProfileAttempt::Rejected(gap) => {
                    shielded_send_rejection = Some(gap);
                    record_option!("selector.shielded-send", None)
                }
                ProfileAttempt::NotApplicable => record_option!("selector.shielded-send", None),
            }
        })
        .or_else(|| {
            record_option!(
                "selector.reset-payout",
                typed_plan::lower_reset_payout(
                    circuit,
                    ledger_fields,
                    witnesses,
                    pure_circuits,
                    circuits,
                )
            )
        })
        .or_else(|| {
            record_option!(
                "selector.shielded-payout",
                typed_plan::lower_shielded_payout(
                    circuit,
                    ledger_fields,
                    witnesses,
                    pure_circuits,
                    circuits,
                )
            )
        })
        .or_else(|| {
            record_option!(
                "selector.terminal-returns",
                typed_plan::lower_terminal_returns(
                    circuit,
                    ledger_fields,
                    witnesses,
                    pure_circuits,
                    circuits,
                )
            )
        })
        .or_else(|| {
            record_option!(
                "selector.phase-reset",
                typed_plan::lower_phase_reset(
                    circuit,
                    ledger_fields,
                    witnesses,
                    pure_circuits,
                    circuits,
                )
            )
        })
        .or_else(|| {
            record_option!(
                "selector.unit-actions",
                typed_plan::lower_unit_actions(
                    circuit,
                    ledger_fields,
                    witnesses,
                    pure_circuits,
                    circuits,
                )
            )
        })
        .or_else(|| {
            match typed_plan::lower_unit_composition_checked(
                circuit,
                ledger_fields,
                witnesses,
                pure_circuits,
                circuits,
            ) {
                ProfileAttempt::Admitted(plan) => {
                    record_option!("selector.unit-composition", Some(plan))
                }
                ProfileAttempt::Rejected(rejection) => {
                    composition_rejection = Some(rejection);
                    record_option!("selector.unit-composition", None)
                }
                ProfileAttempt::NotApplicable => record_option!("selector.unit-composition", None),
            }
        })
        .or_else(|| {
            record_option!(
                "selector.zswap",
                zswap_plan::lower(circuit, ledger_fields, circuits, witnesses)
            )
        })
    {
        organizer_steps = Some(plan.steps);
        typed_result = Some(plan.result);
    }
    Ok(ProfileSelection {
        organizer_steps,
        typed_result,
        shielded_send_rejection,
        composition_rejection,
    })
}
