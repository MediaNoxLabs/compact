// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Recheck declaration identity, codec/Default capabilities and independent SSA uses.
use super::*;

pub(in crate::recorded::effect_plan) fn element<'a>(
    declarations: &'a Declarations,
    slot: ListSlot,
    path: &str,
) -> Result<&'a Type, RecordingGap> {
    let field = declarations
        .field(slot.declaration)
        .ok_or_else(|| invalid(path, "unknown List declaration"))?;
    let LedgerFieldKind::List { ty } = &field.declaration else {
        return Err(invalid(path, "expected List declaration"));
    };
    if field.index != slot.index {
        return Err(invalid(path, "List declaration index mismatch"));
    }
    declarations.types().validate_at(ty, path)?;
    if !list_element_type(ty) {
        return Err(invalid(path, "unsupported List element codec"));
    }
    Ok(ty)
}

pub(in crate::recorded::effect_plan) fn head_type(
    declarations: &Declarations,
    element: &Type,
    result: &Type,
    path: &str,
) -> Result<(), RecordingGap> {
    declarations.types().validate_at(result, path)?;
    if !default_value_type(element) {
        return Err(invalid(path, "List head requires a defaultable element"));
    }
    // Preserve native Maybe/MaybeCompactN naming and the exact is_some/value layout.
    if *result != crate::list_head_result_type(element, result) || !cell_type(result) {
        return Err(invalid(
            path,
            "List head result differs from native Maybe type",
        ));
    }
    Ok(())
}

pub(super) fn validate(
    slot: ListSlot,
    operation: &ListOperation,
    result: &Type,
    declarations: &Declarations,
    scope: &HashMap<usize, Type>,
    path: &str,
) -> Result<bool, RecordingGap> {
    let element = element(declarations, slot, path)?;
    Ok(match operation {
        ListOperation::PushFront(value) => {
            used(value, scope, declarations, path)?;
            value.ty == *element && *result == Type::Unit
        }
        ListOperation::PopFront | ListOperation::Reset => *result == Type::Unit,
        ListOperation::Length => *result == uint("18446744073709551615"),
        ListOperation::IsEmpty => *result == Type::Boolean,
        ListOperation::Head => {
            head_type(declarations, element, result, path)?;
            true
        }
    })
}
