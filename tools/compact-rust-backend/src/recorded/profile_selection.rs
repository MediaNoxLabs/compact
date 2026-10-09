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
use super::{effect_plan, kernel_plan, zswap_plan};

#[cfg(test)]
#[path = "profile_selection/jubjub_migration_tests.rs"]
mod jubjub_migration_tests;
#[cfg(test)]
#[path = "profile_selection/terminal_migration_tests.rs"]
mod terminal_migration_tests;
#[cfg(test)]
#[path = "profile_selection/unit_migration_tests.rs"]
mod unit_migration_tests;

/// An admitted step sequence and the evidence retained from declined profiles.
/// Legacy organizers have no typed result; absence of steps permits legacy
/// action lowering. The caller owns whether a retained gap can refine a failure.
pub(super) struct ProfileSelection {
    pub(super) organizer_steps: Option<Vec<syn::Stmt>>,
    pub(super) typed_result: Option<syn::Expr>,
    pub(super) shielded_send_rejection: Option<RecordingGap>,
    pub(super) composition_rejection: Option<CompositionRejection>,
}

/// Roll out the declared point result domain at its existing selector priority.
/// Shape evidence never authorizes this route. Unsupported can try independent
/// later families; a checked-plan invariant failure aborts before any fallback.
fn checked_jubjub_domain(
    circuit: &StatefulCircuit,
    ledger: &HashMap<&str, &LedgerField>,
    witnesses: &HashMap<&str, &WitnessDeclaration>,
    pure: &HashMap<&str, &PureCircuit>,
    circuits: &HashMap<&str, &StatefulCircuit>,
) -> Result<Option<TypedPlan>, RenderError> {
    if circuit.result != crate::ir::Type::JubjubPoint {
        return Ok(None);
    }
    resolve_checked_plan(effect_plan::lower_with_calls(
        circuit, ledger, witnesses, pure, circuits,
    ))
}

/// Ordinary Field interfaces are the migration boundary; body support follows
/// checked operations. No one-slot or syntactic read/write evidence is required.
fn checked_terminal_domain(
    circuit: &StatefulCircuit,
    ledger: &HashMap<&str, &LedgerField>,
    witnesses: &HashMap<&str, &WitnessDeclaration>,
    pure: &HashMap<&str, &PureCircuit>,
    circuits: &HashMap<&str, &StatefulCircuit>,
) -> Result<Option<TypedPlan>, RenderError> {
    use crate::ir::{StateReturn, Type};
    if circuit.result != Type::Field
        || !matches!(circuit.return_value, StateReturn::Expression { .. })
        || !circuit
            .parameters
            .iter()
            .all(|p| matches!(p.ty, Type::Field | Type::Boolean))
    {
        return Ok(None);
    }
    resolve_checked_plan(effect_plan::lower_with_calls(
        circuit, ledger, witnesses, pure, circuits,
    ))
}

/// Unit expression interfaces enter at the former unit-action priority. Body
/// support is compositional: no helper/write counts or flat Cell paths authorize it.
fn checked_unit_domain(
    circuit: &StatefulCircuit,
    ledger: &HashMap<&str, &LedgerField>,
    witnesses: &HashMap<&str, &WitnessDeclaration>,
    pure: &HashMap<&str, &PureCircuit>,
    circuits: &HashMap<&str, &StatefulCircuit>,
) -> Result<Option<TypedPlan>, RenderError> {
    use crate::ir::{StateReturn, Type};
    if circuit.result != Type::Unit
        || !matches!(circuit.return_value, StateReturn::Expression { .. })
        || !circuit
            .parameters
            .iter()
            .all(|p| matches!(p.ty, Type::Field | Type::Boolean))
    {
        return Ok(None);
    }
    resolve_checked_plan(effect_plan::lower_with_calls(
        circuit, ledger, witnesses, pure, circuits,
    ))
}

fn resolve_checked_plan(
    result: Result<TypedPlan, effect_plan::PlanError>,
) -> Result<Option<TypedPlan>, RenderError> {
    match result {
        Ok(plan) => Ok(Some(plan)),
        Err(effect_plan::PlanError::Unsupported(_)) => Ok(None),
        Err(effect_plan::PlanError::Invalid(gap)) => Err(RenderError::RecordingPlanInvariant {
            path: gap.path,
            detail: gap.detail,
        }),
    }
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
    let typed_prefix = if organizer_steps.is_none() {
        match record_option!(
            "selector.membership-optional",
            typed_plan::lower(circuit, ledger_fields, witnesses, pure_circuits, circuits)
        ) {
            Some(plan) => Some(plan),
            None => record_option!(
                "algebra.jubjub-values",
                checked_jubjub_domain(circuit, ledger_fields, witnesses, pure_circuits, circuits)?
            ),
        }
    } else {
        None
    };
    if organizer_steps.is_none() {
        let before_terminal = typed_prefix
            .or_else(|| record_option!("selector.kernel", kernel_plan::lower(circuit, witnesses)))
            .or_else(|| {
                record_option!(
                    "selector.context-query",
                    typed_plan::lower_context_query(
                        circuit,
                        ledger_fields,
                        witnesses,
                        pure_circuits,
                    )
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
            });
        // Preserve the lazy selector boundary. Invariant errors abort here,
        // before any independent later family can hide a checked-plan defect.
        let through_terminal = match before_terminal {
            Some(plan) => Some(plan),
            None => record_option!(
                "algebra.terminal-returns",
                checked_terminal_domain(
                    circuit,
                    ledger_fields,
                    witnesses,
                    pure_circuits,
                    circuits
                )?
            ),
        };
        let through_phase_reset = through_terminal.or_else(|| {
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
        });
        // An invalid checked plan is fatal before later legacy alternatives.
        let through_unit = match through_phase_reset {
            Some(plan) => Some(plan),
            None => record_option!(
                "algebra.unit-actions",
                checked_unit_domain(circuit, ledger_fields, witnesses, pure_circuits, circuits)?
            ),
        };
        if let Some(plan) = through_unit
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
                    ProfileAttempt::NotApplicable => {
                        record_option!("selector.unit-composition", None)
                    }
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
    }
    Ok(ProfileSelection {
        organizer_steps,
        typed_result,
        shielded_send_rejection,
        composition_rejection,
    })
}
