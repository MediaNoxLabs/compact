// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Existing CellValue codecs, separated from public FAB and Merkle policies.
use super::{Type, domain, surface_type};

/// Canonical definitions and resource limits must be validated by each caller.
/// Recursive codecs stop at tuple8, including inside zero-length vectors; public
/// FAB alone is insufficient. Named generated products derive the same codecs.
fn codec_type(ty: &Type) -> bool {
    domain::value_type(ty) && domain::derives_supported(ty)
}

/// Keep the established top-level Unit restriction; Unit members and tuple0
/// retain their actual recursive codec representation.
fn stored_value(ty: &Type) -> bool {
    *ty != Type::Unit && codec_type(ty)
}

pub(super) fn cell_value_type(ty: &Type) -> bool {
    stored_value(ty)
}
pub(in super::super) fn set_element_type(ty: &Type) -> bool {
    stored_value(ty)
}
pub(in super::super) fn map_key_type(ty: &Type) -> bool {
    stored_value(ty)
}
/// Identical to the separately qualified #595 scalar Map value relation.
pub(super) fn map_value_type(ty: &Type) -> bool {
    stored_value(ty)
}

/// Merkle leaf eligibility remains the previously qualified storage relation.
/// A top-level Unit storage value stays refused, while Unit members retain the prior
/// surface policy. Canonical definitions and resource bounds are checked first.
pub(in super::super) fn merkle_leaf_type(ty: &Type) -> bool {
    fn member(ty: &Type) -> bool {
        match ty {
            Type::OpaqueString | Type::OpaqueBytes => true,
            Type::Struct { fields, .. } => {
                !fields.is_empty() && fields.iter().all(|field| member(&field.ty))
            }
            _ => surface_type(ty),
        }
    }
    *ty != Type::Unit && member(ty)
}
