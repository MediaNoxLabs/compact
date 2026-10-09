// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Slot identity and element eligibility come from the immutable declaration registry.
use super::*;

pub(super) fn element<'a>(
    declarations: &'a Declarations,
    slot: SetSlot,
    path: &str,
) -> Result<&'a Type, RecordingGap> {
    let field = declarations
        .field(slot.declaration)
        .ok_or_else(|| invalid(path, "unknown Set declaration"))?;
    if field.index != slot.index {
        return Err(invalid(path, "Set declaration index mismatch"));
    }
    let LedgerFieldKind::Set { ty } = &field.declaration else {
        return Err(invalid(path, "expected Set declaration"));
    };
    declarations.types().validate_at(ty, path)?;
    if !storage_value_type(ty) {
        return Err(invalid(path, "unsupported Set element type"));
    }
    Ok(ty)
}
