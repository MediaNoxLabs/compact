// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Map identity and active operand types come from immutable declarations.
use super::*;

/// Scalar Map values are ledger Cells, not necessarily scalar Rust primitives.
/// Existing CellValue implementations cover owned products, fixed vectors and
/// canonical wide integers; tuple codecs stop at arity8 (FAB alone reaches11).
/// Canonical definitions and resource preflight remain the caller's authority.
/// Keep the established top-level Unit refusal; Unit members and empty named
/// products still have recursive CellValue representations.
/// This capability does not widen Map keys, ordinary Cells or Sets.
pub(in super::super) fn scalar_value_type(ty: &Type) -> bool {
    *ty != Type::Unit && value_type(ty) && super::domain::derives_supported(ty)
}

pub(in super::super) fn types<'a>(
    declarations: &'a Declarations,
    slot: MapSlot,
    scalar: bool,
    path: &str,
) -> Result<(&'a Type, &'a Type), RecordingGap> {
    let field = declarations
        .field(slot.declaration)
        .ok_or_else(|| invalid(path, "unknown Map declaration"))?;
    if field.index != slot.index {
        return Err(invalid(path, "Map declaration index mismatch"));
    }
    let LedgerFieldKind::Map { key, value } = &field.declaration else {
        return Err(invalid(path, "expected Map declaration"));
    };
    declarations.types().validate_at(key, path)?;
    if !storage_value_type(key) {
        return Err(invalid(path, "unsupported Map key type"));
    }
    if scalar {
        declarations.types().validate_at(value, path)?;
        if !scalar_value_type(value) {
            return Err(invalid(path, "unsupported scalar Map value type"));
        }
    } else {
        // Use the native slot authority, not a second approximation of its
        // representation rules. The bounded result is discarded; this grants
        // neither a scalar value codec nor a checked SSA value for nested Maps.
        crate::resource_limits::recording_type_nodes(value)
            .map_err(|e| invalid(path, e.to_string()))?;
        if crate::map_slot_types(key, value)
            .map_err(|e| invalid(path, e.to_string()))?
            .is_none()
        {
            return Err(invalid(path, "Map value has no native slot representation"));
        }
    }
    Ok((key, value))
}
