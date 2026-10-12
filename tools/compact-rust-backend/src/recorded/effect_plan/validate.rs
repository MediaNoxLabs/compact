// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Scope, use-before-definition, operation signatures and branch joins.
use super::{RecordingGap, declarations::Declarations, invalid, model::*};
use crate::ir::{LedgerFieldKind, Type};
use std::collections::{HashMap, HashSet};
mod aggregates;
mod calls;
mod fab;
pub(super) use fab::{aligned_fab_type, fixed_aligned_fab_type, keccak_input_type};
mod domain;
pub(super) mod lists;
pub(super) mod maps;
mod merkle;
mod operations;
pub(super) use merkle::root_type;
mod pure;
mod sets;
mod storage;
mod vector_map;
pub(super) use storage::{list_element_type, map_key_type, merkle_leaf_type, set_element_type};

struct Validation {
    allocated: HashSet<usize>,
    remaining: usize,
    active_calls: Vec<String>,
    block_depth: usize,
}
impl Validation {
    fn charge(&mut self, count: usize, path: &str) -> Result<(), RecordingGap> {
        self.remaining = self
            .remaining
            .checked_sub(count)
            .ok_or_else(|| invalid(path, "recording plan expanded work limit exceeded"))?;
        Ok(())
    }
}

pub(super) fn uint(max: &str) -> Type {
    Type::Unsigned { max: max.into() }
}
/// Canonical unsigned bounds for values, explicit casts and wide-result addition.
/// Subtraction and multiplication retain the u128-only `bound` policy.
pub(super) fn unsigned_bound(ty: &Type) -> Option<crate::UnsignedMaximum> {
    let Type::Unsigned { max } = ty else {
        return None;
    };
    crate::unsigned_maximum(max).ok()
}
pub(super) fn bound(ty: &Type) -> Option<u128> {
    match unsigned_bound(ty)? {
        crate::UnsignedMaximum::Small(n) => Some(n),
        _ => None,
    }
}
/// Matches native addition: all-small carriers, or a canonical wide result.
/// A wide operand with a small result still needs an explicit narrowing cast.
pub(super) fn unsigned_add_type(left: &Type, right: &Type, result: &Type) -> bool {
    matches!(
        (
            unsigned_bound(left),
            unsigned_bound(right),
            unsigned_bound(result)
        ),
        (Some(_), Some(_), Some(crate::UnsignedMaximum::Wide { .. }))
            | (
                Some(crate::UnsignedMaximum::Small(_)),
                Some(crate::UnsignedMaximum::Small(_)),
                Some(crate::UnsignedMaximum::Small(_)),
            )
    )
}
pub(super) fn scalar(ty: &Type) -> bool {
    matches!(ty, Type::Unit | Type::Boolean | Type::Field) || bound(ty).is_some()
}
/// Recorded ABI values; Cell storage and scalar arithmetic use separate predicates.
pub(super) fn surface_type(ty: &Type) -> bool {
    scalar(ty)
        || *ty == Type::JubjubPoint
        || match ty {
            Type::Bytes { length } => {
                *length <= crate::resource_limits::recording_byte_payload_limit()
            }
            Type::Enum { variants, .. } => !variants.is_empty(),
            Type::Struct { fields, .. } => {
                !fields.is_empty() && fields.iter().all(|f| surface_type(&f.ty))
            }
            _ => false,
        }
}
/// Public root signatures may own opaque values; other policies stay separate.
pub(super) fn public_interface_type(ty: &Type) -> bool {
    domain::public_interface_type(ty)
}
/// Internal roots exchange local Rust values; exported roots require public FAB.
/// Validation obtains this boundary only from the independent source declaration.
pub(super) fn root_interface_type(ty: &Type, internal: bool) -> bool {
    if internal {
        stateful_call_type(ty)
    } else {
        public_interface_type(ty)
    }
}
/// Declared Cell storage may own opaque values without widening interfaces.
pub(super) fn cell_type(ty: &Type) -> bool {
    storage::cell_value_type(ty)
}
/// Witness inputs are typed callback values, not private FAB output encodings.
/// Keep this policy named separately from roots, results and ledger storage.
pub(super) fn witness_parameter_type(ty: &Type) -> bool {
    value_type(ty)
}
/// Witness outputs have an encoding domain independent of public interfaces.
pub(super) fn witness_result_type(ty: &Type) -> bool {
    domain::witness_result_type(ty)
}
/// Same-type equality composes the runtime wrappers and supported Rust products.
pub(super) fn equality_type(ty: &Type) -> bool {
    match ty {
        Type::Struct { fields, .. } => {
            value_type(ty) && fields.iter().all(|field| equality_type(&field.ty))
        }
        Type::Vector { element, .. } => value_type(ty) && equality_type(element),
        // Rust's standard tuple PartialEq implementations stop at arity 12.
        // Larger tuple temporaries remain valid; only this == operation refuses.
        Type::Tuple { elements } => elements.len() <= 12 && elements.iter().all(equality_type),
        _ => value_type(ty),
    }
}
/// Named structs acquire representation derives even when used only in pure bodies.
pub(super) fn named_struct_derives_supported(ty: &Type) -> bool {
    matches!(ty, Type::Struct { .. }) && domain::derives_supported(ty)
}
/// Default has its own Rust trait bound; local ownership is not sufficient.
pub(super) fn default_value_type(ty: &Type) -> bool {
    domain::default_value_type(ty)
}
/// Reuse the same tuple trait limit for certified pure and checked defaults.
pub(super) fn default_tuple_traits_supported(nodes: &[&Type]) -> bool {
    domain::default_tuple_traits_supported(nodes)
}
pub(super) fn value_type(ty: &Type) -> bool {
    domain::value_type(ty)
}
/// Local stateful helpers exchange checked owned values without exporting an ABI.
/// Callers validate canonical definitions and resource budgets before this policy.
/// Root, witness and storage interfaces retain their own separate domains.
pub(super) fn stateful_call_type(ty: &Type) -> bool {
    value_type(ty)
}
/// Context-free helpers may exchange local owned values; public ABI boundaries
/// use public_interface_type. Wide and tuple helpers are intentional here.
pub(super) fn pure_interface_type(ty: &Type) -> bool {
    value_type(ty)
}
pub(super) fn coercible(source: &Type, target: &Type) -> bool {
    domain::coercible(source, target)
}
pub(super) fn wide_literal_fits(bytes: &[u8; 32], ty: &Type) -> bool {
    let Some(crate::UnsignedMaximum::Wide { high, low }) = unsigned_bound(ty) else {
        return false;
    };
    let value_low = u128::from_le_bytes(bytes[..16].try_into().expect("low limb"));
    let value_high = u128::from_le_bytes(bytes[16..].try_into().expect("high limb"));
    bytes[31] == 0
        && (value_high, value_low) <= (high, low)
        && midnight_transient_crypto::curve::Fr::from_le_bytes(bytes).is_some()
}
pub(super) fn check(plan: Plan, declarations: Declarations) -> Result<CheckedPlan, RecordingGap> {
    CheckedPlan::validate(plan, declarations)
}
pub(super) fn validate(plan: &Plan, declarations: &Declarations) -> Result<(), RecordingGap> {
    declarations
        .types()
        .validate_at(&plan.result_type, "return_value")?;
    for value in &plan.parameters {
        declarations.types().validate_at(&value.ty, "parameters")?;
    }
    let internal = declarations.root().is_some_and(|root| root.internal);
    if !root_interface_type(&plan.result_type, internal) {
        return Err(invalid("return_value", "unsupported circuit result type"));
    }
    let mut scope = HashMap::new();
    let mut state = Validation {
        allocated: HashSet::new(),
        remaining: crate::resource_limits::recording_call_limits().1,
        active_calls: Vec::new(),
        block_depth: 0,
    };
    if let Some(root) = declarations.root() {
        declarations
            .types()
            .validate_at(&root.result, "return_value")?;
        for parameter in &root.parameters {
            declarations
                .types()
                .validate_at(&parameter.ty, "parameters")?;
        }
        if root.result != plan.result_type
            || root.parameters.len() != plan.parameters.len()
            || !root
                .parameters
                .iter()
                .zip(&plan.parameters)
                .all(|(p, v)| p.ty == v.ty)
        {
            return Err(invalid(
                "parameters",
                "root plan differs from declared signature",
            ));
        }
        let mut names = HashSet::new();
        if root.parameters.iter().any(|p| !names.insert(&p.name)) {
            return Err(invalid("parameters", "duplicate root formal name"));
        }
        state.active_calls.push(root.name.clone());
    }
    state.charge(plan.parameters.len(), "parameters")?;
    for value in &plan.parameters {
        if !root_interface_type(&value.ty, internal) || !state.allocated.insert(value.id) {
            return Err(invalid(
                "parameters",
                "unsupported or duplicate plan parameter",
            ));
        }
        scope.insert(value.id, value.ty.clone());
    }
    block(&plan.body, declarations, &scope, &mut state, "return_value")?;
    if plan.body.result.ty != plan.result_type {
        return Err(invalid(
            "return_value",
            "plan result type differs from circuit result",
        ));
    }
    Ok(())
}
fn used(
    value: &Value,
    scope: &HashMap<usize, Type>,
    declarations: &Declarations,
    path: &str,
) -> Result<(), RecordingGap> {
    // References carry their own annotations and may be forged independently of
    // the definition. Bound and bind them before recursive structural equality.
    declarations.types().validate_at(&value.ty, path)?;
    if scope.get(&value.id) != Some(&value.ty) {
        return Err(invalid(
            path,
            "value is out of scope or has an inconsistent type",
        ));
    }
    Ok(())
}
fn block(
    body: &Block,
    declarations: &Declarations,
    incoming: &HashMap<usize, Type>,
    state: &mut Validation,
    path: &str,
) -> Result<(), RecordingGap> {
    if state.block_depth >= crate::resource_limits::recording_block_depth_limit() {
        return Err(invalid(path, "recording plan block depth limit exceeded"));
    }
    state.charge(body.bindings.len().saturating_add(1), path)?;
    state.block_depth += 1;
    let mut scope = incoming.clone();
    for binding in &body.bindings {
        let path = &binding.path;
        let ty = &binding.value.ty;
        declarations.types().validate_at(ty, path)?;
        if !value_type(ty) {
            return Err(invalid(path, "unsupported plan value type"));
        }
        let valid = match &binding.operation {
            Operation::VectorMap { .. } => {
                vector_map::validate(binding, declarations, &scope, state)?;
                true
            }
            Operation::Call { .. } => {
                calls::validate(binding, declarations, &scope, state)?;
                true
            }
            Operation::If {
                condition,
                then,
                otherwise,
            } => {
                used(condition, &scope, declarations, path)?;
                block(then, declarations, &scope, state, path)?;
                block(otherwise, declarations, &scope, state, path)?;
                value_type(ty)
                    && condition.ty == Type::Boolean
                    && then.result.ty == *ty
                    && otherwise.result.ty == *ty
            }
            _ => operations::validate(binding, declarations, &scope, state)?,
        };
        if !valid {
            return Err(invalid(
                path,
                "operation input/output types are incompatible",
            ));
        }
        if !state.allocated.insert(binding.value.id) {
            return Err(invalid(path, "plan value is defined twice"));
        }
        scope.insert(binding.value.id, ty.clone());
    }
    used(&body.result, &scope, declarations, path)?;
    state.block_depth -= 1;
    Ok(())
}

fn cell<'a>(
    declarations: &'a Declarations,
    slot: CellSlot,
    path: &str,
) -> Result<&'a Type, RecordingGap> {
    let field = declarations
        .field(slot.declaration)
        .ok_or_else(|| invalid(path, "unknown Cell declaration"))?;
    if field.index != slot.index {
        return Err(invalid(path, "Cell declaration index mismatch"));
    }
    let LedgerFieldKind::Cell { ty } = &field.declaration else {
        return Err(invalid(path, "expected Cell declaration"));
    };
    declarations.types().validate_at(ty, path)?;
    if !cell_type(ty) {
        return Err(invalid(path, "unsupported Cell value type"));
    }
    Ok(ty)
}
fn counter(declarations: &Declarations, slot: CounterSlot, path: &str) -> Result<(), RecordingGap> {
    let field = declarations
        .field(slot.declaration)
        .ok_or_else(|| invalid(path, "unknown Counter declaration"))?;
    if field.index != slot.index {
        return Err(invalid(path, "Counter declaration index mismatch"));
    }
    if field.declaration != LedgerFieldKind::Counter {
        return Err(invalid(path, "expected Counter declaration"));
    }
    Ok(())
}
