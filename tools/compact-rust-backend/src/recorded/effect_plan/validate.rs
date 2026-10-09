// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Scope, use-before-definition, operation signatures and branch joins.
use super::{RecordingGap, declarations::Declarations, invalid, model::*};
use crate::ir::{LedgerFieldKind, Type};
use std::collections::{HashMap, HashSet};

pub(super) fn uint(max: &str) -> Type {
    Type::Unsigned { max: max.into() }
}
/// Value-only unsigned bounds. ABI surfaces and arithmetic still use `bound`.
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
pub(super) fn scalar(ty: &Type) -> bool {
    matches!(ty, Type::Unit | Type::Boolean | Type::Field) || bound(ty).is_some()
}
fn value_type(ty: &Type) -> bool {
    scalar(ty) || unsigned_bound(ty).is_some()
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
    if !scalar(&plan.result_type) {
        return Err(invalid("return_value", "unsupported circuit result type"));
    }
    let mut scope = HashMap::new();
    let mut allocated = HashSet::new();
    for value in &plan.parameters {
        if !scalar(&value.ty) || !allocated.insert(value.id) {
            return Err(invalid(
                "parameters",
                "unsupported or duplicate plan parameter",
            ));
        }
        scope.insert(value.id, value.ty.clone());
    }
    block(
        &plan.body,
        declarations,
        &scope,
        &mut allocated,
        "return_value",
    )?;
    if plan.body.result.ty != plan.result_type {
        return Err(invalid(
            "return_value",
            "plan result type differs from circuit result",
        ));
    }
    Ok(())
}
fn used(value: &Value, scope: &HashMap<usize, Type>, path: &str) -> Result<(), RecordingGap> {
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
    allocated: &mut HashSet<usize>,
    path: &str,
) -> Result<(), RecordingGap> {
    let mut scope = incoming.clone();
    for binding in &body.bindings {
        let path = &binding.path;
        let ty = &binding.value.ty;
        let use_value = |v: &Value| used(v, &scope, path);
        if !value_type(ty) {
            return Err(invalid(path, "unsupported plan value type"));
        }
        let valid = match &binding.operation {
            Operation::Literal(literal) => match literal {
                Literal::Unit => *ty == Type::Unit,
                Literal::Boolean(_) => *ty == Type::Boolean,
                Literal::Field(bytes) => {
                    *ty == Type::Field
                        && midnight_transient_crypto::curve::Fr::from_le_bytes(bytes).is_some()
                }
                Literal::Unsigned(n) => bound(ty).is_some_and(|max| *n <= max),
                Literal::WideUnsigned(bytes) => wide_literal_fits(bytes, ty),
            },
            Operation::Add(a, b) => {
                use_value(a)?;
                use_value(b)?;
                a.ty == Type::Field && b.ty == Type::Field && *ty == Type::Field
            }
            Operation::UnsignedAdd(a, b) => {
                use_value(a)?;
                use_value(b)?;
                bound(&a.ty).is_some() && bound(&b.ty).is_some() && bound(ty).is_some()
            }
            Operation::Equal(a, b) => {
                use_value(a)?;
                use_value(b)?;
                a.ty == b.ty && *ty == Type::Boolean
            }
            Operation::Convert { value, kind } => {
                use_value(value)?;
                match kind {
                    Conversion::Coerce => {
                        value.ty == *ty
                            || (unsigned_bound(&value.ty).is_some() && *ty == Type::Field)
                            || matches!((bound(&value.ty), bound(ty)), (Some(a),Some(b)) if a <= b)
                    }
                    Conversion::UnsignedCast => bound(&value.ty).is_some() && bound(ty).is_some(),
                }
            }
            Operation::CellRead(slot) => {
                cell(declarations, *slot, path)? == ty && *ty != Type::Unit
            }
            Operation::CellWrite { slot, value } => {
                use_value(value)?;
                let declared = cell(declarations, *slot, path)?;
                *declared == value.ty && *declared != Type::Unit && *ty == Type::Unit
            }
            Operation::CounterRead { slot } => {
                counter(declarations, *slot, path)?;
                *ty == uint("18446744073709551615")
            }
            Operation::CounterIncrement { slot, amount } => {
                counter(declarations, *slot, path)?;
                use_value(amount)?;
                amount.ty == uint("65535") && *ty == Type::Unit
            }
            Operation::Witness {
                declaration,
                arguments,
            } => {
                let witness = declarations
                    .witness(*declaration)
                    .ok_or_else(|| invalid(path, "unknown witness declaration"))?;
                for value in arguments {
                    use_value(value)?;
                }
                witness.result == Type::Field
                    && witness.result == *ty
                    && witness.parameters.len() == arguments.len()
                    && witness
                        .parameters
                        .iter()
                        .zip(arguments)
                        .all(|(p, a)| scalar(&p.ty) && p.ty == a.ty)
            }
            Operation::Assert { condition, .. } => {
                use_value(condition)?;
                condition.ty == Type::Boolean && *ty == Type::Unit
            }
            Operation::If {
                condition,
                then,
                otherwise,
            } => {
                use_value(condition)?;
                block(then, declarations, &scope, allocated, path)?;
                block(otherwise, declarations, &scope, allocated, path)?;
                scalar(ty)
                    && condition.ty == Type::Boolean
                    && then.result.ty == *ty
                    && otherwise.result.ty == *ty
            }
        };
        if !valid {
            return Err(invalid(
                path,
                "operation input/output types are incompatible",
            ));
        }
        if !allocated.insert(binding.value.id) {
            return Err(invalid(path, "plan value is defined twice"));
        }
        scope.insert(binding.value.id, ty.clone());
    }
    used(&body.result, &scope, path)
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
    if !scalar(ty) || *ty == Type::Unit {
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
