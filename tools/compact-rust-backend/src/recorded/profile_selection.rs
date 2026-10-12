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
use super::effect_plan;
use super::guarded_mutations::{
    closed_organizer_gate_steps, closed_witness_admitted_merkle_insert_steps,
};
use super::profile_attempt::ProfileAttempt;
use super::typed_plan;
use super::typed_plan::TypedPlan;

#[path = "profile_selection/unit_domain.rs"]
mod unit_domain;

#[cfg(test)]
#[path = "profile_selection/enum_migration_tests.rs"]
mod enum_migration_tests;

#[cfg(test)]
#[path = "profile_selection/optional_write_migration_tests.rs"]
mod optional_write_migration_tests;

#[cfg(test)]
#[path = "profile_selection/broad_unit_inclusion_tests.rs"]
mod broad_unit_inclusion_tests;
#[cfg(test)]
#[path = "profile_selection/composite_migration_tests.rs"]
mod composite_migration_tests;
#[cfg(test)]
#[path = "profile_selection/context_migration_tests.rs"]
mod context_migration_tests;
#[cfg(test)]
#[path = "profile_selection/jubjub_migration_tests.rs"]
mod jubjub_migration_tests;
#[cfg(test)]
#[path = "profile_selection/kernel_migration_tests.rs"]
mod kernel_migration_tests;
#[cfg(test)]
#[path = "profile_selection/receive_migration_tests.rs"]
mod receive_migration_tests;
#[cfg(test)]
#[path = "profile_selection/terminal_migration_tests.rs"]
mod terminal_migration_tests;
#[cfg(test)]
#[path = "profile_selection/unit_migration_tests.rs"]
mod unit_migration_tests;
#[cfg(test)]
#[path = "profile_selection/voting_migration_tests.rs"]
mod voting_migration_tests;
#[cfg(test)]
#[path = "profile_selection/zswap_migration_tests.rs"]
mod zswap_migration_tests;

#[cfg(test)]
#[path = "profile_selection/membership_migration_tests.rs"]
mod membership_migration_tests;

#[cfg(test)]
#[path = "profile_selection/send_migration_tests.rs"]
mod send_migration_tests;

#[cfg(test)]
#[path = "profile_selection/merge_migration_tests.rs"]
mod merge_migration_tests;

#[cfg(test)]
#[path = "profile_selection/guarded_deposit_migration_tests.rs"]
mod guarded_deposit_migration_tests;

#[cfg(test)]
#[path = "profile_selection/reset_payout_migration_tests.rs"]
mod reset_payout_migration_tests;

#[cfg(test)]
#[path = "profile_selection/shielded_payout_migration_tests.rs"]
mod shielded_payout_migration_tests;

#[cfg(test)]
#[path = "profile_selection/funded_mint_migration_tests.rs"]
mod funded_mint_migration_tests;

/// An admitted step sequence and the evidence retained from declined profiles.
/// Legacy organizers have no typed result; absence of steps permits legacy
/// action lowering. The caller owns whether a retained gap can refine a failure.
pub(super) struct ProfileSelection {
    pub(super) organizer_steps: Option<Vec<syn::Stmt>>,
    pub(super) typed_result: Option<syn::Expr>,
    pub(super) shielded_send_rejection: Option<RecordingGap>,
    pub(super) unit_composition_rejection: Option<effect_plan::LoweringRejection>,
}

/// Membership interfaces enter at the old first typed priority, including helpers.
/// Independent effect composition replaces whole-body and operation-count admission.
fn checked_membership_domain(
    circuit: &StatefulCircuit,
    ledger: &HashMap<&str, &LedgerField>,
    witnesses: &HashMap<&str, &WitnessDeclaration>,
    pure: &HashMap<&str, &PureCircuit>,
    circuits: &HashMap<&str, &StatefulCircuit>,
) -> Result<Option<TypedPlan>, RenderError> {
    use crate::ir::{StateReturn, Type};
    let uint64 = Type::Unsigned {
        max: u64::MAX.to_string(),
    };
    let parameters = circuit.parameters.as_slice();
    let counter_parameters = parameters
        .iter()
        .all(|p| p.ty == Type::Boolean || p.ty == uint64);
    let eligible = match &circuit.result {
        Type::Unit => {
            counter_parameters
                || matches!(parameters, [p] if matches!(p.ty, Type::Enum { .. } | Type::OpaqueString))
                || matches!(parameters, [a, b] if matches!(a.ty, Type::Struct { .. }) && matches!(b.ty, Type::Struct { .. }))
        }
        Type::Boolean => counter_parameters,
        Type::OpaqueString => parameters.is_empty(),
        Type::Field => matches!(parameters, [p] if p.ty == Type::Field),
        result if result == &uint64 => parameters.iter().all(|p| p.ty == Type::Boolean),
        _ => false,
    };
    let supported_return = matches!(circuit.return_value, StateReturn::Expression { .. })
        || (circuit.result == Type::Unit && circuit.return_value == StateReturn::Unit);
    if !eligible || !supported_return {
        return Ok(None);
    }
    resolve_checked_plan(effect_plan::lower_with_calls(
        circuit, ledger, witnesses, pure, circuits,
    ))
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

/// Ordinary Unit interfaces enter at the former unit-action priority. Body
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
        || !matches!(
            circuit.return_value,
            StateReturn::Unit | StateReturn::Expression { .. }
        )
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

/// Bytes32 expression interfaces enter at the former context-query priority.
/// Observation counts and whole-body shape audits do not authorize composition.
fn checked_context_domain(
    circuit: &StatefulCircuit,
    ledger: &HashMap<&str, &LedgerField>,
    witnesses: &HashMap<&str, &WitnessDeclaration>,
    pure: &HashMap<&str, &PureCircuit>,
    circuits: &HashMap<&str, &StatefulCircuit>,
) -> Result<Option<TypedPlan>, RenderError> {
    use crate::ir::{StateReturn, Type};
    if circuit.result != (Type::Bytes { length: 32 })
        || !circuit.parameters.is_empty()
        || !matches!(circuit.return_value, StateReturn::Expression { .. })
    {
        return Ok(None);
    }
    resolve_checked_plan(effect_plan::lower_with_calls(
        circuit, ledger, witnesses, pure, circuits,
    ))
}

/// Kernel interfaces roll out here; actual effects follow checked composition.
fn checked_kernel_domain(
    circuit: &StatefulCircuit,
    ledger: &HashMap<&str, &LedgerField>,
    witnesses: &HashMap<&str, &WitnessDeclaration>,
    pure: &HashMap<&str, &PureCircuit>,
    circuits: &HashMap<&str, &StatefulCircuit>,
) -> Result<Option<TypedPlan>, RenderError> {
    use crate::ir::{StateReturn, Type};
    if circuit.internal
        || circuit.result != Type::Unit
        || !matches!(
            circuit.return_value,
            StateReturn::Unit | StateReturn::Expression { .. }
        )
        || !circuit.parameters.iter().all(|p| {
            matches!(p.ty, Type::Boolean | Type::Bytes { length: 32 })
                || p.ty
                    == (Type::Unsigned {
                        max: u64::MAX.to_string(),
                    })
        })
    {
        return Ok(None);
    }
    resolve_checked_plan(effect_plan::lower_with_calls(
        circuit, ledger, witnesses, pure, circuits,
    ))
}

/// Struct expression interfaces compose at the old composite selector priority.
/// Internal helpers retain eligibility; body effects, not evidence counts, decide.
fn checked_composite_attempt(
    circuit: &StatefulCircuit,
    ledger: &HashMap<&str, &LedgerField>,
    witnesses: &HashMap<&str, &WitnessDeclaration>,
    pure: &HashMap<&str, &PureCircuit>,
    circuits: &HashMap<&str, &StatefulCircuit>,
) -> Result<ProfileAttempt<TypedPlan>, RenderError> {
    use crate::ir::{StateReturn, Type};
    if !matches!(circuit.result, Type::Struct { .. })
        || !matches!(circuit.return_value, StateReturn::Expression { .. })
    {
        return Ok(ProfileAttempt::NotApplicable);
    }
    resolve_checked_attempt(effect_plan::lower_with_calls(
        circuit, ledger, witnesses, pure, circuits,
    ))
}

/// Retain unsupported diagnostics without treating them as global admission errors.
fn resolve_checked_attempt(
    result: Result<TypedPlan, effect_plan::PlanError>,
) -> Result<ProfileAttempt<TypedPlan>, RenderError> {
    match result {
        Ok(plan) => Ok(ProfileAttempt::Admitted(plan)),
        Err(effect_plan::PlanError::Unsupported(gap)) => Ok(ProfileAttempt::Rejected(gap)),
        Err(effect_plan::PlanError::Invalid(gap)) => Err(RenderError::RecordingPlanInvariant {
            path: gap.path,
            detail: gap.detail,
        }),
    }
}

#[cfg(test)]
fn checked_composite_domain(
    circuit: &StatefulCircuit,
    ledger: &HashMap<&str, &LedgerField>,
    witnesses: &HashMap<&str, &WitnessDeclaration>,
    pure: &HashMap<&str, &PureCircuit>,
    circuits: &HashMap<&str, &StatefulCircuit>,
) -> Result<Option<TypedPlan>, RenderError> {
    checked_composite_attempt(circuit, ledger, witnesses, pure, circuits)
        .map(ProfileAttempt::into_option)
}

/// Receive interfaces enter at the old receive priority, including internal helpers.
/// Typed composition decides body support without wrapper or intent-count shape gates.
fn checked_receive_domain(
    circuit: &StatefulCircuit,
    ledger: &HashMap<&str, &LedgerField>,
    witnesses: &HashMap<&str, &WitnessDeclaration>,
    pure: &HashMap<&str, &PureCircuit>,
    circuits: &HashMap<&str, &StatefulCircuit>,
) -> Result<Option<TypedPlan>, RenderError> {
    use crate::ir::{StateReturn, Type};
    if circuit.result != Type::Unit
        || circuit.return_value != StateReturn::Unit
        || !matches!(circuit.parameters.as_slice(), [parameter]
            if parameter.ty == crate::coin_shapes::shielded_coin_type())
    {
        return Ok(None);
    }
    resolve_checked_plan(effect_plan::lower_with_calls(
        circuit, ledger, witnesses, pure, circuits,
    ))
}

/// Voting interfaces enter at the old voting priority, including internal helpers.
/// Typed composition decides body support without operation or witness-count gates.
fn checked_voting_domain(
    circuit: &StatefulCircuit,
    ledger: &HashMap<&str, &LedgerField>,
    witnesses: &HashMap<&str, &WitnessDeclaration>,
    pure: &HashMap<&str, &PureCircuit>,
    circuits: &HashMap<&str, &StatefulCircuit>,
) -> Result<Option<TypedPlan>, RenderError> {
    use crate::ir::{StateReturn, Type};
    if circuit.result != Type::Unit
        || circuit.return_value != StateReturn::Unit
        || !matches!(circuit.parameters.as_slice(), [ballot, coin]
            if ballot.ty == Type::Boolean
                && coin.ty == crate::coin_shapes::shielded_coin_type())
    {
        return Ok(None);
    }
    resolve_checked_plan(effect_plan::lower_with_calls(
        circuit, ledger, witnesses, pure, circuits,
    ))
}

/// Guarded deposit interfaces enter at their existing selector priority.
/// The beneficiary is any declared one-field Bytes<32> product. Typed effects,
/// rather than a receive/merge prefix or static evidence counts, admit the body.
fn checked_guarded_deposit_domain(
    circuit: &StatefulCircuit,
    ledger: &HashMap<&str, &LedgerField>,
    witnesses: &HashMap<&str, &WitnessDeclaration>,
    pure: &HashMap<&str, &PureCircuit>,
    circuits: &HashMap<&str, &StatefulCircuit>,
) -> Result<Option<TypedPlan>, RenderError> {
    use crate::ir::{StateReturn, Type};
    if circuit.result != Type::Unit
        || circuit.return_value != StateReturn::Unit
        || !matches!(circuit.parameters.as_slice(), [topic, beneficiary, seed]
            if topic.ty == Type::OpaqueString
                && matches!(&beneficiary.ty, Type::Struct { fields, .. }
                    if matches!(fields.as_slice(), [member]
                        if member.ty == (Type::Bytes { length: 32 })))
                && seed.ty == crate::coin_shapes::shielded_coin_type())
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

/// External Unit carrier interfaces enter at the former Zswap priority.
/// Typed effects decide body support; intent/query counts do not grant admission.
fn checked_zswap_domain(
    circuit: &StatefulCircuit,
    ledger: &HashMap<&str, &LedgerField>,
    witnesses: &HashMap<&str, &WitnessDeclaration>,
    pure: &HashMap<&str, &PureCircuit>,
    circuits: &HashMap<&str, &StatefulCircuit>,
) -> Result<Option<TypedPlan>, RenderError> {
    use crate::coin_shapes::{qualified_coin_type, shielded_coin_type, shielded_recipient_type};
    use crate::ir::{StateReturn, Type};
    if circuit.internal
        || circuit.result != Type::Unit
        || !matches!(
            circuit.return_value,
            StateReturn::Unit | StateReturn::Expression { .. }
        )
        || !circuit.parameters.iter().all(|parameter| {
            parameter.ty == shielded_coin_type()
                || parameter.ty == qualified_coin_type()
                || parameter.ty == shielded_recipient_type()
                || matches!(parameter.ty, Type::Boolean | Type::Bytes { length: 32 })
                || parameter.ty
                    == (Type::Unsigned {
                        max: u64::MAX.to_string(),
                    })
        })
    {
        return Ok(None);
    }
    resolve_checked_plan(effect_plan::lower_with_calls(
        circuit, ledger, witnesses, pure, circuits,
    ))
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
    // Checked composition may find a precise nested rejection while a later
    // profile can still admit the same circuit. Retain it only for a coarse
    // generic return gap after every existing profile has declined.
    let mut shielded_send_rejection = None;
    let mut unit_composition_rejection = None;
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
            "algebra.membership",
            checked_membership_domain(circuit, ledger_fields, witnesses, pure_circuits, circuits)?
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
        let before_context = match typed_prefix {
            Some(plan) => Some(plan),
            None => record_option!(
                "algebra.kernel",
                checked_kernel_domain(circuit, ledger_fields, witnesses, pure_circuits, circuits)?
            ),
        };
        let through_context = match before_context {
            Some(plan) => Some(plan),
            None => record_option!(
                "algebra.context-query",
                checked_context_domain(circuit, ledger_fields, witnesses, pure_circuits, circuits)?
            ),
        };
        let through_composite = match through_context {
            Some(plan) => Some(plan),
            None => {
                let attempt = checked_composite_attempt(
                    circuit,
                    ledger_fields,
                    witnesses,
                    pure_circuits,
                    circuits,
                )?;
                let plan = match attempt {
                    ProfileAttempt::Admitted(plan) => Some(plan),
                    ProfileAttempt::Rejected(gap) => {
                        // The old send domain is a subset of this Struct/Expression
                        // interface. Preserve its useful failure through later
                        // alternatives without rerunning any legacy body recognizer.
                        if typed_plan::shielded_send_result(&circuit.result) {
                            shielded_send_rejection = Some(gap);
                        }
                        None
                    }
                    ProfileAttempt::NotApplicable => None,
                };
                record_option!("algebra.composite-values", plan)
            }
        };
        let through_receive = match through_composite {
            Some(plan) => Some(plan),
            None => record_option!(
                "algebra.shielded-receive",
                checked_receive_domain(circuit, ledger_fields, witnesses, pure_circuits, circuits)?
            ),
        };
        let through_voting = match through_receive {
            Some(plan) => Some(plan),
            None => record_option!(
                "algebra.voting-commit",
                checked_voting_domain(circuit, ledger_fields, witnesses, pure_circuits, circuits)?
            ),
        };
        let through_deposit = match through_voting {
            Some(plan) => Some(plan),
            None => record_option!(
                "algebra.guarded-deposit",
                checked_guarded_deposit_domain(
                    circuit,
                    ledger_fields,
                    witnesses,
                    pure_circuits,
                    circuits,
                )?
            ),
        };
        // Preserve the lazy selector boundary. Invariant errors abort here,
        // before any independent later family can hide a checked-plan defect.
        let through_terminal = match through_deposit {
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
        // An invalid checked plan is fatal before later legacy alternatives.
        let through_unit = match through_terminal {
            Some(plan) => Some(plan),
            None => record_option!(
                "algebra.unit-actions",
                checked_unit_domain(circuit, ledger_fields, witnesses, pure_circuits, circuits)?
            ),
        };
        let through_composition = match through_unit {
            Some(plan) => Some(plan),
            None => match unit_domain::attempt(
                circuit,
                ledger_fields,
                witnesses,
                pure_circuits,
                circuits,
            )? {
                ProfileAttempt::Admitted(plan) => {
                    record_option!("algebra.unit-composition", Some(plan))
                }
                ProfileAttempt::Rejected(rejection) => {
                    unit_composition_rejection = Some(rejection);
                    record_option!("algebra.unit-composition", None)
                }
                ProfileAttempt::NotApplicable => {
                    record_option!("algebra.unit-composition", None)
                }
            },
        };
        // A checked invariant failure must escape before any emission fallback.
        let through_zswap = match through_composition {
            Some(plan) => Some(plan),
            None => record_option!(
                "algebra.zswap",
                checked_zswap_domain(circuit, ledger_fields, witnesses, pure_circuits, circuits)?
            ),
        };
        if let Some(plan) = through_zswap {
            organizer_steps = Some(plan.steps);
            typed_result = Some(plan.result);
        }
    }
    Ok(ProfileSelection {
        organizer_steps,
        typed_result,
        shielded_send_rejection,
        unit_composition_rejection,
    })
}
