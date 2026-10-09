// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Local ownership and conversion policy; exported/storage surfaces stay separate.
use super::{Type, surface_type, unsigned_bound};

pub(super) fn value_type(ty: &Type) -> bool {
    match ty {
        Type::OpaqueString | Type::OpaqueBytes => true,
        Type::Vector { element, .. } => value_type(element),
        Type::Tuple { elements } => elements.iter().all(value_type),
        Type::Struct { fields, .. } => fields
            .iter()
            .all(|field| value_type(&field.ty) && derives_supported(&field.ty)),
        _ => surface_type(ty) || unsigned_bound(ty).is_some(),
    }
}

// Generated structs also derive CompactCellValue. Its tuple implementations stop
// at arity8 (stricter than Rust's Eq/Default bound12). Standalone tuples do not
// acquire these named-struct representation derives.
pub(super) fn derives_supported(ty: &Type) -> bool {
    match ty {
        Type::Tuple { elements } => elements.len() <= 8 && elements.iter().all(derives_supported),
        Type::Vector { element, .. } => derives_supported(element),
        Type::Struct { fields, .. } => fields.iter().all(|field| derives_supported(&field.ty)),
        _ => true,
    }
}

/// Exact native aggregate conversion relation, with no syntax construction.
/// Callers independently bound/canonicalize both types before recursive comparison.
pub(super) fn coercible(source: &Type, target: &Type) -> bool {
    if !value_type(source) || !value_type(target) {
        return false;
    }
    if source == target {
        return true;
    }
    match (source, target) {
        (Type::Unsigned { .. }, Type::Field) => unsigned_bound(source).is_some(),
        (Type::Unsigned { .. }, Type::Unsigned { .. }) => {
            fn limbs(bound: crate::UnsignedMaximum) -> (u128, u128) {
                match bound {
                    crate::UnsignedMaximum::Small(low) => (0, low),
                    crate::UnsignedMaximum::Wide { high, low } => (high, low),
                }
            }
            matches!((unsigned_bound(source),unsigned_bound(target)),(Some(a),Some(b)) if limbs(a)<=limbs(b))
        }
        (
            Type::Vector {
                element: a,
                length: n,
            },
            Type::Vector {
                element: b,
                length: m,
            },
        ) => n == m && coercible(a, b),
        (Type::Tuple { elements: a }, Type::Tuple { elements: b }) => {
            !a.is_empty() && a.len() == b.len() && a.iter().zip(b).all(|(a, b)| coercible(a, b))
        }
        (Type::Tuple { elements }, Type::Vector { element, length }) => {
            !elements.is_empty()
                && elements.len() == *length
                && elements.iter().all(|source| coercible(source, element))
        }
        (Type::Vector { element, length }, Type::Tuple { elements }) => {
            !elements.is_empty()
                && elements.len() == *length
                && elements.iter().all(|target| coercible(element, target))
        }
        _ => false,
    }
}

/// Storage eligibility is independent of public/stateful/witness interfaces.
/// A top-level Unit Cell stays refused, while Unit members retain the prior
/// surface policy. Canonical definitions and resource bounds are checked first.
pub(super) fn cell_type(ty: &Type) -> bool {
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
