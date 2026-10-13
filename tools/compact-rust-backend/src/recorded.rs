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

//! Emit complete replayable traces from supported typed stateful IR.
//! Unsupported effect shapes have no generated recorded entry point.

// Pass-through observations compile out of production. The expression is
// evaluated once in its original scope, preserving `?`, eager/lazy selection,
// and validation order. Hooks never choose a plan.
#[cfg(test)]
macro_rules! record_option {
    ($route:literal, $value:expr) => {{
        let value = $value;
        #[cfg(test)]
        $crate::recorded::admission_probe::trace::option($route, value.is_some());
        value
    }};
}

#[cfg(test)]
pub(crate) mod test_support;

mod diagnostics;
pub(crate) use diagnostics::RecordingOutcome;
pub use diagnostics::{
    RecordingGap, RecordingGapCode, RecordingResource, RecordingResourceKind,
    RecordingResourcePhase,
};
#[cfg(test)]
mod legacy;
#[cfg(test)]
use legacy::actions::append_steps;
#[cfg(test)]
use legacy::expressions::{boolean_expression, field_expression};

#[cfg(test)]
mod asset_removal_migration_tests;
mod facade;
#[cfg(test)]
mod guarded_map_migration_tests;
#[cfg(test)]
mod guarded_mutations;
#[cfg(test)]
mod guarded_pure_migration_tests;
#[cfg(test)]
mod helper_discovery_tests;
mod helpers;
#[cfg(test)]
mod helpers_reference;
mod item;
#[cfg(test)]
mod opaque_map_migration_tests;
#[cfg(test)]
mod opaque_set_migration_tests;
#[cfg(test)]
mod profile_attempt;
mod profile_selection;
#[cfg(test)]
mod proved_set_sequence_migration_tests;
#[cfg(test)]
mod pure_calls;
#[cfg(test)]
mod unsigned_product_migration_tests;
pub(crate) use helpers::{plan_recorded_helpers, retain_reachable_helpers};
#[cfg(test)]
use profile_attempt::ProfileAttempt;
#[cfg(test)]
use pure_calls::closed_pure_field_call;
#[cfg(test)]
mod admission_probe;
mod effect_plan;
#[cfg(test)]
mod effectful_migration_tests;
#[cfg(test)]
mod generic_migration_tests;
#[cfg(test)]
mod generic_reference;
#[cfg(test)]
mod intent_effect;
#[cfg(test)]
mod kernel_plan;
mod recording_plan;
#[cfg(test)]
mod start_funding_migration_tests;
#[cfg(test)]
mod typed_plan;
#[cfg(test)]
mod zswap_plan;

pub(crate) use facade::{
    render_borrowed_recorded_contract_method, render_observed_call_method,
    render_recorded_contract_method,
};

use proc_macro2::Span;
use std::collections::{HashMap, HashSet};

#[cfg(test)]
use crate::expression_with_calls;
#[cfg(test)]
use crate::ir::{CounterAmount, LocalBinding};
#[cfg(test)]
use crate::ir::{Expr, LedgerFieldKind, StateAction, Type};
use crate::ir::{LedgerField, PureCircuit, StateReturn, StatefulCircuit, WitnessDeclaration};
#[cfg(test)]
use crate::retained_value;
use crate::stateful::circuit_uses_witness;
use crate::{RenderError, ident, rust_type};

#[cfg(test)]
mod audited_local;
#[cfg(test)]
#[path = "recorded/audited_local_migration_tests.rs"]
mod audited_local_migration_tests;
#[cfg(test)]
#[path = "recorded/read_only_migration_tests.rs"]
mod read_only_migration_tests;
#[cfg(test)]
mod read_only_verification;

// Direct call refusals and exact failed binding ordinals are distinct provenance.
// Only binding provenance propagates through recursive legacy action boundaries;
// ordinary concrete/call diagnostics retain their previous priority.
#[cfg(test)]
#[derive(Clone, Copy, PartialEq, Eq)]
enum LegacyActionPrecision {
    Concrete,
    Coarse,
    BindingValue(usize),
}

#[cfg(test)]
impl LegacyActionPrecision {
    fn inherit_binding(&mut self, child: Self) {
        if matches!(child, Self::BindingValue(_)) {
            *self = child;
        }
    }
}

#[cfg(test)]
fn unavailable_action(action: &StateAction, path: &str) -> RecordingOutcome<()> {
    RecordingOutcome::Unsupported(RecordingGap::action(action, path.to_owned()))
}

#[cfg(test)]
fn set_size_operand(value: &Expr) -> Option<(&str, u8)> {
    match value {
        Expr::SetSize { field, index } => Some((field, *index)),
        Expr::Coerce { value, ty }
            if *ty
                == (Type::Unsigned {
                    max: u64::MAX.to_string(),
                }) =>
        {
            set_size_operand(value)
        }
        _ => None,
    }
}

#[cfg(test)]
fn list_length_operand(value: &Expr) -> Option<(&str, u8)> {
    match value {
        Expr::ListLength { field, index } => Some((field, *index)),
        Expr::Coerce { value, ty }
            if *ty
                == (Type::Unsigned {
                    max: u64::MAX.to_string(),
                }) =>
        {
            list_length_operand(value)
        }
        _ => None,
    }
}

#[cfg(test)]
fn list_head_field<'a>(
    value: &'a Expr,
    expected_field: &str,
    expected_index: usize,
) -> Option<(&'a str, u8, &'a Type)> {
    let Expr::StructField {
        value,
        field,
        index,
    } = value
    else {
        return None;
    };
    if field != expected_field || *index != expected_index {
        return None;
    }
    let Expr::ListHead { field, index, ty } = value.as_ref() else {
        return None;
    };
    Some((field, *index, ty))
}

#[cfg(test)]
fn uint64_literal(value: &Expr) -> Option<u64> {
    match value {
        Expr::UnsignedLiteral { value, max } => {
            let value = value.parse::<u64>().ok()?;
            (value as u128 <= max.parse::<u128>().ok()?).then_some(value)
        }
        Expr::Coerce { value, ty }
            if *ty
                == (Type::Unsigned {
                    max: u64::MAX.to_string(),
                }) =>
        {
            uint64_literal(value)
        }
        _ => None,
    }
}

/// Keep exported scalar-expression recording tied to a ledger read. Other
/// action-free expressions may still be lowered as private shared helpers.
#[cfg(test)]
fn contains_cell_read(value: &Expr) -> bool {
    match value {
        Expr::CellRead { .. } => true,
        Expr::Coerce { value, .. } | Expr::FieldCast { value } => contains_cell_read(value),
        Expr::Add { left, right }
        | Expr::Subtract { left, right }
        | Expr::Multiply { left, right }
        | Expr::Equal { left, right }
        | Expr::NotEqual { left, right } => contains_cell_read(left) || contains_cell_read(right),
        Expr::Let { bindings, body } => {
            bindings
                .iter()
                .any(|binding| contains_cell_read(&binding.value))
                || contains_cell_read(body)
        }
        Expr::If {
            condition,
            then,
            otherwise,
        } => {
            contains_cell_read(condition)
                || contains_cell_read(then)
                || contains_cell_read(otherwise)
        }
        Expr::Sequence { steps, value } => {
            steps.iter().any(contains_cell_read) || contains_cell_read(value)
        }
        _ => false,
    }
}

/// These generated Rust types implement the runtime's existing `CellValue`
/// contract, including aligned FAB encoding and checked readback.
#[cfg(test)]
fn recordable_cell_type(ty: &Type) -> bool {
    match ty {
        Type::Boolean
        | Type::Field
        | Type::JubjubPoint
        | Type::Bytes { .. }
        | Type::Unsigned { .. }
        | Type::Enum { .. } => true,
        Type::Struct { fields, .. } => fields.iter().all(|field| recordable_cell_type(&field.ty)),
        Type::Tuple { elements } => {
            !elements.is_empty() && elements.len() <= 8 && elements.iter().all(recordable_cell_type)
        }
        Type::Vector { element, .. } => recordable_cell_type(element),
        Type::Unit | Type::OpaqueString | Type::OpaqueBytes | Type::LedgerMap { .. } => false,
    }
}

/// Opaque string recording is admitted for the proved assertion → insertion
/// → Unit witness sequence. Other opaque APIs need their own TS/ledger proof
/// parity before the capability report can advertise them.
#[cfg(test)]
fn proved_opaque_set_sequence(circuit: &StatefulCircuit) -> bool {
    let [parameter] = circuit.parameters.as_slice() else {
        return false;
    };
    if parameter.ty != Type::OpaqueString
        || circuit.result != Type::Unit
        || circuit.return_value != StateReturn::Unit
    {
        return false;
    }
    let [
        StateAction::Assert {
            condition: Expr::SetMember { value: member, .. },
            ..
        },
        StateAction::SetInsert {
            value: inserted, ..
        },
        StateAction::Expression {
            value: Expr::WitnessCall { arguments, .. },
        },
    ] = circuit.actions.as_slice()
    else {
        return false;
    };
    let [
        Expr::Coerce {
            value: witnessed,
            ty: Type::OpaqueString,
        },
    ] = arguments.as_slice()
    else {
        return false;
    };
    [member.as_ref(), inserted, witnessed.as_ref()]
        .iter()
        .all(|value| matches!(value, Expr::Parameter { name } if name == &parameter.name))
}

/// The one-operation OpaqueString Set shapes proved by the set oracle. Keep
/// the field, physical index, and parameter source tied to the typed IR so
/// general opaque expressions remain outside the recording surface.
#[cfg(test)]
fn closed_opaque_set_operation(
    circuit: &StatefulCircuit,
    ledger_fields: &HashMap<&str, &LedgerField>,
) -> bool {
    let [parameter] = circuit.parameters.as_slice() else {
        return false;
    };
    if parameter.ty != Type::OpaqueString {
        return false;
    }
    let operation = match (&circuit.actions[..], &circuit.return_value, &circuit.result) {
        (
            [
                StateAction::SetInsert {
                    field,
                    index,
                    value,
                },
            ],
            StateReturn::Unit,
            Type::Unit,
        ) => Some((field, index, value)),
        (
            [],
            StateReturn::SetMember {
                field,
                index,
                value,
            },
            Type::Boolean,
        ) => Some((field, index, value)),
        _ => None,
    };
    let Some((field, index, value)) = operation else {
        return false;
    };
    let Some(declaration) = ledger_fields.get(field.as_str()) else {
        return false;
    };
    declaration.index == *index
        && matches!(&declaration.declaration, LedgerFieldKind::Set { ty } if *ty == Type::OpaqueString)
        && matches!(value, Expr::Parameter { name } if name == &parameter.name)
}

/// The original opaque-key Map oracle has two closed shapes. The Map key is
/// only encoded as an input; the lookup result is the fixed-width Field value.
#[cfg(test)]
enum ClosedOpaqueMapOperation {
    Put,
    Ensure { field: String, key: String },
}

#[cfg(test)]
fn closed_opaque_map_operation(
    circuit: &StatefulCircuit,
    ledger_fields: &HashMap<&str, &LedgerField>,
) -> Option<ClosedOpaqueMapOperation> {
    let map_matches = |field: &str, index| {
        let declaration = ledger_fields.get(field)?;
        (declaration.index == index
            && matches!(
                &declaration.declaration,
                LedgerFieldKind::Map {
                    key: Type::OpaqueString,
                    value: Type::Field,
                }
            ))
        .then_some(())
    };
    match (
        circuit.parameters.as_slice(),
        &circuit.result,
        &circuit.return_value,
        circuit.actions.as_slice(),
    ) {
        (
            [key, value],
            Type::Unit,
            StateReturn::Unit,
            [
                StateAction::MapInsert {
                    field,
                    index,
                    key: Expr::Parameter { name: key_name },
                    value: Expr::Parameter { name: value_name },
                },
            ],
        ) if key.ty == Type::OpaqueString
            && value.ty == Type::Field
            && key_name == &key.name
            && value_name == &value.name =>
        {
            map_matches(field, *index)?;
            Some(ClosedOpaqueMapOperation::Put)
        }
        (
            [key],
            Type::Field,
            StateReturn::Expression {
                value: Expr::Let { bindings, body },
            },
            [
                StateAction::Assert {
                    condition:
                        Expr::MapMember {
                            field: member_field,
                            index: member_index,
                            key: member_key,
                        },
                    ..
                },
            ],
        ) if key.ty == Type::OpaqueString => {
            let [binding] = bindings.as_slice() else {
                return None;
            };
            let Expr::MapLookup {
                field,
                index,
                key: lookup_key,
            } = &binding.value
            else {
                return None;
            };
            if binding.ty != Type::Field
                || field != member_field
                || index != member_index
                || !matches!(member_key.as_ref(), Expr::Parameter { name } if name == &key.name)
                || !matches!(lookup_key.as_ref(), Expr::Parameter { name } if name == &key.name)
                || !matches!(body.as_ref(), Expr::Parameter { name } if name == &binding.name)
            {
                return None;
            }
            map_matches(field, *index)?;
            Some(ClosedOpaqueMapOperation::Ensure {
                field: field.clone(),
                key: key.name.clone(),
            })
        }
        _ => None,
    }
}

/// The asset removal path keeps a disclosed OpaqueString key scoped across
/// ordered guards and mutations. Only this exact shape has an oracle and a
/// ledger proof; unrelated OpaqueString Lets remain unavailable.
#[cfg(test)]
fn closed_opaque_asset_removal(
    circuit: &StatefulCircuit,
    ledger_fields: &HashMap<&str, &LedgerField>,
) -> bool {
    let [parameter] = circuit.parameters.as_slice() else {
        return false;
    };
    if parameter.ty != Type::OpaqueString
        || circuit.result != Type::Unit
        || circuit.return_value != StateReturn::Unit
    {
        return false;
    }
    let [StateAction::Let { bindings, action }] = circuit.actions.as_slice() else {
        return false;
    };
    let [binding] = bindings.as_slice() else {
        return false;
    };
    if binding.ty != Type::OpaqueString
        || !matches!(&binding.value, Expr::Parameter { name } if name == &parameter.name)
    {
        return false;
    }
    let StateAction::Sequence { actions } = action.as_ref() else {
        return false;
    };
    let [
        StateAction::CircuitCall {
            name: writable,
            arguments: writable_args,
        },
        StateAction::Assert {
            condition:
                Expr::MapMember {
                    field: records_member,
                    index: records_member_index,
                    key: member_key,
                },
            ..
        },
        StateAction::Assert {
            condition:
                Expr::If {
                    condition: watched,
                    then: unguarded,
                    otherwise: guarded,
                },
            ..
        },
        StateAction::MapRemove {
            field: records_remove,
            index: records_remove_index,
            key: removed_key,
        },
        StateAction::SetInsert {
            field: retired,
            index: retired_index,
            value: retired_key,
        },
        StateAction::CircuitCall {
            name: write,
            arguments: write_args,
        },
    ] = actions.as_slice()
    else {
        return false;
    };
    let Expr::SetMember {
        field: watch_field,
        index: watch_index,
        value: watched_key,
    } = watched.as_ref()
    else {
        return false;
    };
    let key_is_bound =
        |value: &Expr| matches!(value, Expr::Parameter { name } if name == &binding.name);
    if writable != "assertWritable"
        || !writable_args.is_empty()
        || write != "recordWrite"
        || !write_args.is_empty()
        || records_member != "records"
        || records_remove != records_member
        || records_member_index != records_remove_index
        || watch_field != "watchList"
        || retired != "retiredKeys"
        || !matches!(unguarded.as_ref(), Expr::Boolean { value: false })
        || !matches!(guarded.as_ref(), Expr::Boolean { value: true })
        || ![
            member_key.as_ref(),
            watched_key.as_ref(),
            removed_key,
            retired_key,
        ]
        .into_iter()
        .all(key_is_bound)
    {
        return false;
    }
    matches!(ledger_fields.get(records_member.as_str()), Some(field)
        if field.index == *records_member_index
            && matches!(&field.declaration, LedgerFieldKind::Map { key: Type::OpaqueString, .. }))
        && matches!(ledger_fields.get(watch_field.as_str()), Some(field)
            if field.index == *watch_index
                && matches!(&field.declaration, LedgerFieldKind::Set { ty: Type::OpaqueString }))
        && matches!(ledger_fields.get(retired.as_str()), Some(field)
            if field.index == *retired_index
                && matches!(&field.declaration, LedgerFieldKind::Set { ty: Type::OpaqueString }))
}

/// A closed typed witness → Bytes32 hash → Set authorization followed by a
/// single Set insertion. The matched callees are inspected transitively so
/// recording never silently skips an assertion or private output.
// Native validation requires struct coercions to preserve the exact type. Other
// coercions may need a conversion of an untyped recorded local; leave these
// unavailable until recorded local bindings carry their source types.
/// Emit a replayable public VM trace for supported root Cell, Counter, Set, Map, List,
/// and plain/historic Merkle append
/// operations, including witnessed Cell values. Unsupported circuits have no
/// recorded entry point.
pub(crate) fn render_recorded_circuit(
    circuit: &StatefulCircuit,
    ledger_fields: &HashMap<&str, &LedgerField>,
    witnesses: &HashMap<&str, &WitnessDeclaration>,
    pure_circuits: &HashMap<&str, &PureCircuit>,
    circuits: &HashMap<&str, &StatefulCircuit>,
    shared_callees: &HashSet<String>,
) -> Result<RecordingOutcome<syn::Item>, RenderError> {
    #[cfg(test)]
    let trace = admission_probe::trace::Scope::begin(circuit, false);
    let result = render_recorded_item(
        circuit,
        ledger_fields,
        witnesses,
        pure_circuits,
        circuits,
        shared_callees,
        false,
    );
    #[cfg(test)]
    trace.finish(&result);
    result
}

pub(crate) fn render_recorded_helper(
    circuit: &StatefulCircuit,
    ledger_fields: &HashMap<&str, &LedgerField>,
    witnesses: &HashMap<&str, &WitnessDeclaration>,
    pure_circuits: &HashMap<&str, &PureCircuit>,
    circuits: &HashMap<&str, &StatefulCircuit>,
    shared_callees: &HashSet<String>,
) -> Result<RecordingOutcome<syn::Item>, RenderError> {
    #[cfg(test)]
    let trace = admission_probe::trace::Scope::begin(circuit, true);
    let result = render_recorded_item(
        circuit,
        ledger_fields,
        witnesses,
        pure_circuits,
        circuits,
        shared_callees,
        true,
    );
    #[cfg(test)]
    trace.finish(&result);
    result
}

/// Checked effect composition is the admission authority for Effectful roots.
/// Native validation and resource preflight have already run. The compatibility
/// feature no longer changes this route; ordinary root routing remains separate.
fn lower_effectful_return(
    circuit: &StatefulCircuit,
    ledger_fields: &HashMap<&str, &LedgerField>,
    witnesses: &HashMap<&str, &WitnessDeclaration>,
    pure_circuits: &HashMap<&str, &PureCircuit>,
    circuits: &HashMap<&str, &StatefulCircuit>,
) -> Result<RecordingOutcome<recording_plan::TypedPlan>, RenderError> {
    let algebra_gap =
        match checked_effectful_return(circuit, ledger_fields, witnesses, pure_circuits, circuits)?
        {
            RecordingOutcome::Supported(plan) => {
                #[cfg(test)]
                {
                    admission_probe::observe(admission_probe::Route::Algebra);
                    admission_probe::trace::handled("effectful.checked-migration");
                }
                return Ok(RecordingOutcome::Supported(plan));
            }
            RecordingOutcome::Unsupported(gap) => gap,
        };

    #[cfg(test)]
    admission_probe::observe(admission_probe::Route::Unsupported);
    Ok(RecordingOutcome::Unsupported(algebra_gap))
}

fn checked_effectful_return(
    circuit: &StatefulCircuit,
    ledger_fields: &HashMap<&str, &LedgerField>,
    witnesses: &HashMap<&str, &WitnessDeclaration>,
    pure_circuits: &HashMap<&str, &PureCircuit>,
    circuits: &HashMap<&str, &StatefulCircuit>,
) -> Result<RecordingOutcome<recording_plan::TypedPlan>, RenderError> {
    match effect_plan::lower_with_calls_diagnostic(
        circuit,
        ledger_fields,
        witnesses,
        pure_circuits,
        circuits,
    ) {
        Ok(plan) => Ok(RecordingOutcome::Supported(plan)),
        Err(effect_plan::DiagnosticPlanError::Unsupported(rejection)) => {
            Ok(RecordingOutcome::Unsupported(rejection.into_checked_gap()))
        }
        Err(effect_plan::DiagnosticPlanError::Invalid(gap)) => {
            Err(RenderError::RecordingPlanInvariant {
                path: gap.path,
                detail: gap.detail,
            })
        }
    }
}

/// Assemble one independently checked plan. Refusal cannot enter a legacy
/// emitter; the frozen generic implementation is available only to tests.
fn render_recorded_item(
    circuit: &StatefulCircuit,
    ledger_fields: &HashMap<&str, &LedgerField>,
    witnesses: &HashMap<&str, &WitnessDeclaration>,
    pure_circuits: &HashMap<&str, &PureCircuit>,
    circuits: &HashMap<&str, &StatefulCircuit>,
    shared_callees: &HashSet<String>,
    helper: bool,
) -> Result<RecordingOutcome<syn::Item>, RenderError> {
    // Preserve Effectful validation before helper routing and parameters.
    let effectful_plan = if matches!(circuit.return_value, StateReturn::Effectful { .. }) {
        match lower_effectful_return(circuit, ledger_fields, witnesses, pure_circuits, circuits)? {
            RecordingOutcome::Supported(plan) => Some(plan),
            RecordingOutcome::Unsupported(gap) => return Ok(RecordingOutcome::Unsupported(gap)),
        }
    } else {
        None
    };
    if circuit.internal && !helper {
        return Ok(RecordingOutcome::Unsupported(RecordingGap::no_effect()));
    }

    let name = ident(&circuit.name)?;
    let mut parameters = HashSet::new();
    let mut args = Vec::<syn::FnArg>::new();
    for (index, parameter) in circuit.parameters.iter().enumerate() {
        ident(&parameter.name)?;
        let rust_name = syn::Ident::new(&format!("__compact_param_{index}"), Span::call_site());
        if !parameters.insert(parameter.name.as_str()) {
            return Err(RenderError::DuplicateParameter(parameter.name.clone()));
        }
        let arg_ty = rust_type(&parameter.ty)?;
        args.push(syn::parse_quote!(#rust_name: #arg_ty));
    }

    let plan = match profile_selection::select_checked(
        circuit,
        ledger_fields,
        witnesses,
        pure_circuits,
        circuits,
        effectful_plan,
    )? {
        profile_selection::CheckedSelection::Admitted(plan) => plan,
        profile_selection::CheckedSelection::Refused(rejection) => {
            return Ok(RecordingOutcome::Unsupported(rejection.into_checked_gap()));
        }
    };
    let result_ty = rust_type(&circuit.result)?;
    let uses_witness = circuit_uses_witness(circuit, circuits, &mut HashSet::new())?;
    let item = item::assemble(
        circuit,
        circuits,
        shared_callees,
        helper,
        uses_witness,
        &name,
        &args,
        &plan.steps,
        &[],
        &plan.result,
        &result_ty,
    )?;
    Ok(RecordingOutcome::Supported(item))
}
