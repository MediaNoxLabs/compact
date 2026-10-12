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
mod legacy;
use legacy::actions::append_steps;
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
mod helpers;
mod item;
#[cfg(test)]
mod opaque_map_migration_tests;
#[cfg(test)]
mod opaque_set_migration_tests;
mod profile_attempt;
mod profile_selection;
#[cfg(test)]
mod proved_set_sequence_migration_tests;
mod pure_calls;
#[cfg(test)]
mod unsigned_product_migration_tests;
pub(crate) use helpers::{plan_recorded_helpers, retain_reachable_helpers};
#[cfg(test)]
use profile_attempt::ProfileAttempt;
use pure_calls::closed_pure_field_call;
#[cfg(test)]
mod admission_probe;
mod effect_plan;
#[cfg(test)]
mod effectful_migration_tests;
mod intent_effect;
#[cfg(test)]
mod kernel_plan;
mod typed_plan;
#[cfg(test)]
mod zswap_plan;

pub(crate) use facade::{
    render_borrowed_recorded_contract_method, render_observed_call_method,
    render_recorded_contract_method,
};

use proc_macro2::Span;
use std::collections::{HashMap, HashSet};

use crate::ir::{
    CounterAmount, Expr, LedgerField, LedgerFieldKind, LocalBinding, PureCircuit, StateAction,
    StateReturn, StatefulCircuit, Type, WitnessDeclaration,
};
use crate::stateful::circuit_uses_witness;
use crate::{RenderError, expression_with_calls, ident, retained_value, rust_type};

mod audited_local;
#[cfg(test)]
#[path = "recorded/audited_local_migration_tests.rs"]
mod audited_local_migration_tests;
mod read_only_verification;

// Direct call refusals and exact failed binding ordinals are distinct provenance.
// Only binding provenance propagates through recursive legacy action boundaries;
// ordinary concrete/call diagnostics retain their previous priority.
#[derive(Clone, Copy, PartialEq, Eq)]
enum LegacyActionPrecision {
    Concrete,
    Coarse,
    BindingValue(usize),
}

impl LegacyActionPrecision {
    fn inherit_binding(&mut self, child: Self) {
        if matches!(child, Self::BindingValue(_)) {
            *self = child;
        }
    }
}

fn unavailable_action(action: &StateAction, path: &str) -> RecordingOutcome<()> {
    RecordingOutcome::Unsupported(RecordingGap::action(action, path.to_owned()))
}

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
) -> Result<RecordingOutcome<typed_plan::TypedPlan>, RenderError> {
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

    // Independent family, retained until its operations have checked lowering.
    // Only Unsupported reaches this attempt; invariant failures propagate above.
    if let Some(plan) =
        typed_plan::lower_start_funding(circuit, ledger_fields, witnesses, pure_circuits, circuits)
    {
        #[cfg(test)]
        {
            admission_probe::observe(admission_probe::Route::LegacyStartFunding);
            admission_probe::trace::handled("effectful.start-funding");
        }
        return Ok(RecordingOutcome::Supported(plan));
    }
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
) -> Result<RecordingOutcome<typed_plan::TypedPlan>, RenderError> {
    match effect_plan::lower_with_calls(circuit, ledger_fields, witnesses, pure_circuits, circuits)
    {
        Ok(plan) => Ok(RecordingOutcome::Supported(plan)),
        Err(effect_plan::PlanError::Unsupported(gap)) => Ok(RecordingOutcome::Unsupported(gap)),
        Err(effect_plan::PlanError::Invalid(gap)) => Err(RenderError::RecordingPlanInvariant {
            path: gap.path,
            detail: gap.detail,
        }),
    }
}

fn render_recorded_item(
    circuit: &StatefulCircuit,
    ledger_fields: &HashMap<&str, &LedgerField>,
    witnesses: &HashMap<&str, &WitnessDeclaration>,
    pure_circuits: &HashMap<&str, &PureCircuit>,
    circuits: &HashMap<&str, &StatefulCircuit>,
    shared_callees: &HashSet<String>,
    helper: bool,
) -> Result<RecordingOutcome<syn::Item>, RenderError> {
    // Audit the complete ordered plan before any ordinary specialized gate.
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
    if !helper
        && let Some(item) =
            read_only_verification::render(circuit, ledger_fields, witnesses, circuits)?
    {
        #[cfg(test)]
        admission_probe::trace::handled("early.read-only-verification");
        return Ok(item);
    }

    let name = ident(&circuit.name)?;
    let mut parameters = HashMap::new();
    let mut args = Vec::<syn::FnArg>::new();
    for (index, parameter) in circuit.parameters.iter().enumerate() {
        ident(&parameter.name)?;
        let rust_name = syn::Ident::new(&format!("__compact_param_{index}"), Span::call_site());
        if parameters
            .insert(parameter.name.as_str(), (&parameter.ty, rust_name.clone()))
            .is_some()
        {
            return Err(RenderError::DuplicateParameter(parameter.name.clone()));
        }
        let arg_ty = rust_type(&parameter.ty)?;
        args.push(syn::parse_quote!(#rust_name: #arg_ty));
    }

    let profile_selection::ProfileSelection {
        organizer_steps,
        typed_result,
        shielded_send_rejection,
        unit_composition_rejection,
    } = profile_selection::select(
        circuit,
        ledger_fields,
        witnesses,
        pure_circuits,
        circuits,
        effectful_plan,
    )?;
    let organizer_gate = organizer_steps.is_some();
    let mut steps = organizer_steps.unwrap_or_default();
    let mut next_temp = 0;
    let mut visiting = HashSet::from([circuit.name.clone()]);
    // A root Let encloses both its ordered actions and its final return.
    // Capture this exact Field Cell read once, before the nested actions,
    // so the returned value cannot accidentally observe the post-write state.
    let root_field_return = if !organizer_gate {
        match (circuit.actions.as_slice(), &circuit.return_value) {
            (
                [StateAction::Let { bindings, action }],
                StateReturn::Expression {
                    value: Expr::Parameter { name },
                },
            ) if circuit.result == Type::Field => match bindings.as_slice() {
                [binding]
                    if binding.name == *name
                        && binding.ty == Type::Field
                        && matches!(&binding.value, Expr::CellRead { .. }) =>
                {
                    if let Expr::CellRead { field, index } = &binding.value {
                        let declaration = ledger_fields
                            .get(field.as_str())
                            .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
                        if declaration.declaration != (LedgerFieldKind::Cell { ty: Type::Field })
                            || declaration.index != *index
                        {
                            return Ok(RecordingOutcome::Unsupported(RecordingGap::returned(
                                &circuit.return_value,
                            )));
                        }
                        Some((binding.name.as_str(), field.as_str(), action.as_ref()))
                    } else {
                        None
                    }
                }
                _ => None,
            },
            _ => None,
        }
    } else {
        None
    };
    let mut recorded_root_return: Option<(&str, syn::Expr)> = None;
    if !organizer_gate {
        if let Some((name, field, action)) = root_field_return {
            #[cfg(test)]
            admission_probe::trace::action("guard.root-let-snapshot");
            let slot = ident(field)?;
            let saved = syn::Ident::new("__compact_recorded_root_return", Span::call_site());
            steps.push(syn::parse_quote! {
                let (frame, #saved): (_, runtime::Field) =
                    crate::ledger_slots::#slot.record_read(frame)?;
            });
            let locals = HashMap::from([(name.to_owned(), syn::parse_quote!(#saved))]);
            if let RecordingOutcome::Unsupported(gap) = append_steps(
                action,
                "actions[0].action",
                &locals,
                &parameters,
                ledger_fields,
                witnesses,
                pure_circuits,
                circuits,
                shared_callees,
                &mut steps,
                &mut next_temp,
                &mut visiting,
                &mut LegacyActionPrecision::Concrete,
            )? {
                return Ok(RecordingOutcome::Unsupported(gap));
            }
            recorded_root_return = Some((name, syn::parse_quote!(#saved)));
        } else {
            #[cfg(test)]
            if !circuit.actions.is_empty() {
                admission_probe::trace::action("legacy.action-dispatch");
            }
            for (index, action) in circuit.actions.iter().enumerate() {
                let mut legacy_precision = LegacyActionPrecision::Concrete;
                if let RecordingOutcome::Unsupported(gap) = append_steps(
                    action,
                    &format!("actions[{index}]"),
                    &HashMap::new(),
                    &parameters,
                    ledger_fields,
                    witnesses,
                    pure_circuits,
                    circuits,
                    shared_callees,
                    &mut steps,
                    &mut next_temp,
                    &mut visiting,
                    &mut legacy_precision,
                )? {
                    let gap = unit_composition_rejection.as_ref().map_or_else(
                        || gap.clone(),
                        |rejection| rejection.refine_action(index, legacy_precision, gap.clone()),
                    );
                    return Ok(RecordingOutcome::Unsupported(gap));
                }
            }
        }
    }
    // Checked composition has inspected the entire value tree. With no
    // accepted typed or legacy profile (and no actions), the generic return
    // arms cannot record a ShieldedSendResult. Keep the first concrete nested
    // composition failure instead of collapsing it to UnsupportedReturn for an outer If
    // or Call. Action diagnostics above still take precedence.
    if !organizer_gate
        && circuit.actions.is_empty()
        && steps.is_empty()
        && let Some(gap) = shielded_send_rejection.as_ref()
        && gap.code == RecordingGapCode::UnsupportedExpression
    {
        return Ok(RecordingOutcome::Unsupported(gap.clone()));
    }
    let result_ty = rust_type(&circuit.result)?;
    #[cfg(test)]
    if typed_result.is_none() {
        admission_probe::trace::returned();
    }
    let (return_steps, result): (Vec<syn::Stmt>, syn::Expr) = match &circuit.return_value {
        _ if typed_result.is_some() => (
            Vec::new(),
            typed_result.expect("typed plan result was checked"),
        ),
        StateReturn::Expression {
            value: Expr::Let { bindings, body },
        } if circuit.result == Type::Unit
            && circuit.actions.is_empty()
            && matches!(body.as_ref(), Expr::Unit)
            && bindings.iter().all(|binding| binding.ty == Type::Field)
            && bindings
                .iter()
                .any(|binding| contains_cell_read(&binding.value)) =>
        {
            // An unused value may still perform a public ledger read. Evaluate
            // each binding in lexical order before discarding the unit result.
            let mut return_steps = Vec::new();
            let mut locals = HashMap::new();
            for (index, binding) in bindings.iter().enumerate() {
                let Some(value) = field_expression(
                    &binding.value,
                    &locals,
                    &parameters,
                    ledger_fields,
                    witnesses,
                    circuits,
                    shared_callees,
                    &mut return_steps,
                    &mut next_temp,
                    &mut visiting,
                )?
                else {
                    return Ok(RecordingOutcome::Unsupported(RecordingGap::expression(
                        &binding.value,
                        format!("return_value.value.bindings[{index}].value"),
                    )));
                };
                let local = syn::Ident::new(
                    &format!("__compact_recorded_discarded_field_{next_temp}"),
                    Span::call_site(),
                );
                next_temp += 1;
                return_steps.push(syn::parse_quote!(let #local: runtime::Field = #value;));
                locals.insert(binding.name.clone(), syn::parse_quote!(#local));
            }
            (return_steps, syn::parse_quote!(()))
        }
        StateReturn::Expression {
            value: Expr::Parameter { name },
        } if recorded_root_return
            .as_ref()
            .is_some_and(|(bound_name, _)| *bound_name == name.as_str()) =>
        {
            (
                Vec::new(),
                recorded_root_return.expect("matching root local").1,
            )
        }
        StateReturn::Unit if circuit.result == Type::Unit => {
            if steps.is_empty() {
                return Ok(RecordingOutcome::Unsupported(RecordingGap::no_effect()));
            }
            (Vec::new(), syn::parse_quote!(()))
        }
        StateReturn::Expression {
            value: Expr::FieldToBytes32 { value },
        } if circuit.actions.is_empty() && circuit.result == (Type::Bytes { length: 32 }) => {
            let Expr::FieldCast { value } = value.as_ref() else {
                return Ok(RecordingOutcome::Unsupported(RecordingGap::returned(
                    &circuit.return_value,
                )));
            };
            let Expr::CounterRead { field, index } = value.as_ref() else {
                return Ok(RecordingOutcome::Unsupported(RecordingGap::returned(
                    &circuit.return_value,
                )));
            };
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            if declaration.declaration != LedgerFieldKind::Counter || declaration.index != *index {
                return Ok(RecordingOutcome::Unsupported(RecordingGap::returned(
                    &circuit.return_value,
                )));
            }
            let slot = ident(field)?;
            let converted =
                crate::field_to_bytes_32_syntax(syn::parse_quote!(runtime::Field::from(observed)));
            (
                vec![syn::parse_quote!(
                    let (frame, observed): (_, u64) =
                        crate::ledger_slots::#slot.record_read(frame)?;
                )],
                converted,
            )
        }
        StateReturn::CounterRead { field, index }
            if circuit.result
                == (Type::Unsigned {
                    max: u64::MAX.to_string(),
                }) =>
        {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            if declaration.declaration != LedgerFieldKind::Counter || declaration.index != *index {
                return Err(RenderError::UnknownLedgerField(field.clone()));
            }
            let slot = ident(field)?;
            (
                vec![syn::parse_quote!(
                    let (frame, observed): (_, u64) =
                        crate::ledger_slots::#slot.record_read(frame)?;
                )],
                syn::parse_quote!(
                    runtime::BoundedUint::<18446744073709551615>::new(observed as u128)
                        .expect("ledger Counter fits Uint<64>")
                ),
            )
        }
        StateReturn::CellRead { field, index } if recordable_cell_type(&circuit.result) => {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            if declaration.declaration
                != (LedgerFieldKind::Cell {
                    ty: circuit.result.clone(),
                })
                || declaration.index != *index
            {
                return Ok(RecordingOutcome::Unsupported(RecordingGap::returned(
                    &circuit.return_value,
                )));
            }
            let slot = ident(field)?;
            (
                vec![syn::parse_quote!(
                    let (frame, observed): (_, #result_ty) =
                        crate::ledger_slots::#slot.record_read(frame)?;
                )],
                syn::parse_quote!(observed),
            )
        }
        StateReturn::Expression {
            value: Expr::Call { name, arguments },
        } if circuit.result == Type::Field && !circuit.actions.is_empty() => {
            let Some(callee) = pure_circuits.get(name.as_str()) else {
                return Ok(RecordingOutcome::Unsupported(RecordingGap::returned(
                    &circuit.return_value,
                )));
            };
            if callee.result != Type::Field
                || !closed_pure_field_call(name, pure_circuits, &mut HashSet::new())
            {
                return Ok(RecordingOutcome::Unsupported(RecordingGap::returned(
                    &circuit.return_value,
                )));
            }
            if arguments.len() != callee.parameters.len() {
                return Err(RenderError::ArgumentCount {
                    circuit: name.clone(),
                    expected: callee.parameters.len(),
                    actual: arguments.len(),
                });
            }
            let mut return_steps = Vec::new();
            let mut args = Vec::new();
            for (index, (argument, parameter)) in
                arguments.iter().zip(&callee.parameters).enumerate()
            {
                if parameter.ty != Type::Field {
                    return Ok(RecordingOutcome::Unsupported(RecordingGap::expression(
                        argument,
                        format!("return_value.value.arguments[{index}]"),
                    )));
                }
                let Some(value) = field_expression(
                    argument,
                    &HashMap::new(),
                    &parameters,
                    ledger_fields,
                    witnesses,
                    circuits,
                    shared_callees,
                    &mut return_steps,
                    &mut next_temp,
                    &mut visiting,
                )?
                else {
                    return Ok(RecordingOutcome::Unsupported(RecordingGap::expression(
                        argument,
                        format!("return_value.value.arguments[{index}]"),
                    )));
                };
                let arg = syn::Ident::new(
                    &format!("__compact_recorded_arg_{}", next_temp),
                    Span::call_site(),
                );
                next_temp += 1;
                return_steps.push(syn::parse_quote!(let #arg: runtime::Field = #value;));
                args.push(arg);
            }
            let method = ident(name)?;
            let result = syn::Ident::new(
                &format!("__compact_recorded_pure_return_{}", next_temp),
                Span::call_site(),
            );
            return_steps.push(syn::parse_quote! {
                let #result: runtime::Field = crate::pure_circuits::#method(#(#args),*)?;
            });
            (return_steps, syn::parse_quote!(#result))
        }
        StateReturn::Expression { value }
            if circuit.result == Type::Field
                && circuit.actions.is_empty()
                && (helper || (contains_cell_read(value) && !matches!(value, Expr::If { .. }))) =>
        {
            let mut return_steps = Vec::new();
            let Some(result) = field_expression(
                value,
                &HashMap::new(),
                &parameters,
                ledger_fields,
                witnesses,
                circuits,
                shared_callees,
                &mut return_steps,
                &mut next_temp,
                &mut visiting,
            )?
            else {
                return Ok(RecordingOutcome::Unsupported(RecordingGap::expression(
                    value,
                    "return_value.value".to_owned(),
                )));
            };
            (return_steps, result)
        }
        StateReturn::Expression { value }
            if contains_cell_read(value)
                && circuit.result == Type::Boolean
                && circuit.actions.is_empty()
                && !matches!(value, Expr::If { .. }) =>
        {
            let mut return_steps = Vec::new();
            let Some(result) = boolean_expression(
                value,
                &HashMap::new(),
                &parameters,
                ledger_fields,
                witnesses,
                circuits,
                &mut return_steps,
                &mut next_temp,
                &mut visiting,
            )?
            else {
                return Ok(RecordingOutcome::Unsupported(RecordingGap::expression(
                    value,
                    "return_value.value".to_owned(),
                )));
            };
            (return_steps, result)
        }
        StateReturn::Expression {
            value:
                Expr::If {
                    condition,
                    then,
                    otherwise,
                },
        } => {
            match legacy::returns::conditional::lower(
                condition,
                then,
                otherwise,
                &circuit.result,
                &circuit.return_value,
                &result_ty,
                &parameters,
                ledger_fields,
                witnesses,
                pure_circuits,
                circuits,
                &mut next_temp,
                &mut visiting,
            )? {
                RecordingOutcome::Supported(result) => result,
                RecordingOutcome::Unsupported(gap) => {
                    return Ok(RecordingOutcome::Unsupported(gap));
                }
            }
        }
        StateReturn::SetMember {
            field,
            index,
            value,
        } if circuit.result == Type::Boolean => {
            match legacy::returns::collections::set::member(
                field,
                *index,
                value,
                &circuit.return_value,
                &parameters,
                ledger_fields,
                witnesses,
                circuits,
                &mut next_temp,
                &mut visiting,
            )? {
                RecordingOutcome::Supported(result) => result,
                RecordingOutcome::Unsupported(gap) => {
                    return Ok(RecordingOutcome::Unsupported(gap));
                }
            }
        }
        StateReturn::SetSize { field, index }
            if circuit.result
                == (Type::Unsigned {
                    max: u64::MAX.to_string(),
                }) =>
        {
            match legacy::returns::collections::set::size(
                field,
                *index,
                &circuit.return_value,
                ledger_fields,
            )? {
                RecordingOutcome::Supported(result) => result,
                RecordingOutcome::Unsupported(gap) => {
                    return Ok(RecordingOutcome::Unsupported(gap));
                }
            }
        }
        StateReturn::SetIsEmpty { field, index } if circuit.result == Type::Boolean => {
            match legacy::returns::collections::set::is_empty(
                field,
                *index,
                &circuit.return_value,
                ledger_fields,
            )? {
                RecordingOutcome::Supported(result) => result,
                RecordingOutcome::Unsupported(gap) => {
                    return Ok(RecordingOutcome::Unsupported(gap));
                }
            }
        }
        StateReturn::ListLength { field, index }
            if circuit.result
                == (Type::Unsigned {
                    max: u64::MAX.to_string(),
                }) =>
        {
            match legacy::returns::collections::list::length(
                field,
                *index,
                &circuit.return_value,
                ledger_fields,
            )? {
                RecordingOutcome::Supported(result) => result,
                RecordingOutcome::Unsupported(gap) => {
                    return Ok(RecordingOutcome::Unsupported(gap));
                }
            }
        }
        StateReturn::ListIsEmpty { field, index } if circuit.result == Type::Boolean => {
            match legacy::returns::collections::list::is_empty(
                field,
                *index,
                &circuit.return_value,
                ledger_fields,
            )? {
                RecordingOutcome::Supported(result) => result,
                RecordingOutcome::Unsupported(gap) => {
                    return Ok(RecordingOutcome::Unsupported(gap));
                }
            }
        }
        StateReturn::ListHead { field, index } => {
            match legacy::returns::collections::list::head(
                field,
                *index,
                &circuit.return_value,
                &circuit.result,
                &result_ty,
                ledger_fields,
            )? {
                RecordingOutcome::Supported(result) => result,
                RecordingOutcome::Unsupported(gap) => {
                    return Ok(RecordingOutcome::Unsupported(gap));
                }
            }
        }
        StateReturn::MapMember { field, index, key }
        | StateReturn::MapLookup { field, index, key } => {
            match legacy::returns::collections::map::member_or_lookup(
                field,
                *index,
                key,
                &circuit.return_value,
                &circuit.result,
                &result_ty,
                &parameters,
                ledger_fields,
                witnesses,
                circuits,
                &mut next_temp,
                &mut visiting,
            )? {
                RecordingOutcome::Supported(result) => result,
                RecordingOutcome::Unsupported(gap) => {
                    return Ok(RecordingOutcome::Unsupported(gap));
                }
            }
        }
        StateReturn::MapSize { field, index }
            if circuit.result
                == (Type::Unsigned {
                    max: u64::MAX.to_string(),
                }) =>
        {
            match legacy::returns::collections::map::size(
                field,
                *index,
                &circuit.return_value,
                ledger_fields,
            )? {
                RecordingOutcome::Supported(result) => result,
                RecordingOutcome::Unsupported(gap) => {
                    return Ok(RecordingOutcome::Unsupported(gap));
                }
            }
        }
        StateReturn::MapIsEmpty { field, index } if circuit.result == Type::Boolean => {
            match legacy::returns::collections::map::is_empty(
                field,
                *index,
                &circuit.return_value,
                ledger_fields,
            )? {
                RecordingOutcome::Supported(result) => result,
                RecordingOutcome::Unsupported(gap) => {
                    return Ok(RecordingOutcome::Unsupported(gap));
                }
            }
        }
        StateReturn::MerkleCheckRoot { field, index, root }
        | StateReturn::HistoricMerkleCheckRoot { field, index, root }
            if circuit.result == Type::Boolean && circuit.actions.is_empty() =>
        {
            match legacy::returns::merkle::check_root(
                field,
                *index,
                root,
                &circuit.return_value,
                &parameters,
                ledger_fields,
            )? {
                RecordingOutcome::Supported(result) => result,
                RecordingOutcome::Unsupported(gap) => {
                    return Ok(RecordingOutcome::Unsupported(gap));
                }
            }
        }
        StateReturn::MerkleIsFull { field, index }
        | StateReturn::HistoricMerkleIsFull { field, index }
            if circuit.result == Type::Boolean && circuit.actions.is_empty() =>
        {
            match legacy::returns::merkle::is_full(
                field,
                *index,
                &circuit.return_value,
                ledger_fields,
            )? {
                RecordingOutcome::Supported(result) => result,
                RecordingOutcome::Unsupported(gap) => {
                    return Ok(RecordingOutcome::Unsupported(gap));
                }
            }
        }
        StateReturn::Expression {
            value: Expr::Let { bindings, body },
        } if circuit.result == Type::Boolean
            && circuit.actions.is_empty()
            && bindings.len() == 1
            && matches!(body.as_ref(), Expr::MerkleCheckRoot { .. }) =>
        {
            match legacy::returns::merkle::witnessed_path_root(
                bindings,
                body,
                ledger_fields,
                pure_circuits,
                witnesses,
            )? {
                RecordingOutcome::Supported(result) => result,
                RecordingOutcome::Unsupported(gap) => {
                    return Ok(RecordingOutcome::Unsupported(gap));
                }
            }
        }
        _ => {
            let gap = shielded_send_rejection
                .filter(|gap| gap.code == RecordingGapCode::UnsupportedExpression)
                .unwrap_or_else(|| RecordingGap::returned(&circuit.return_value));
            return Ok(RecordingOutcome::Unsupported(gap));
        }
    };
    if !organizer_gate && steps.is_empty() && return_steps.is_empty() {
        return Ok(RecordingOutcome::Unsupported(RecordingGap::no_effect()));
    }
    if circuit
        .parameters
        .iter()
        .any(|parameter| parameter.ty == Type::OpaqueString)
        && !organizer_gate
    {
        return Ok(RecordingOutcome::Unsupported(
            match circuit.actions.first() {
                Some(action) => RecordingGap::action(action, "actions[0]".to_owned()),
                None => RecordingGap::returned(&circuit.return_value),
            },
        ));
    }

    let uses_witness = circuit_uses_witness(circuit, circuits, &mut HashSet::new())?;
    let item = item::assemble(
        circuit,
        circuits,
        shared_callees,
        helper,
        uses_witness,
        &name,
        &args,
        &steps,
        &return_steps,
        &result,
        &result_ty,
    )?;
    Ok(RecordingOutcome::Supported(item))
}
