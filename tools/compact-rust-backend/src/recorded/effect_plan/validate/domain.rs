// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Local ownership and conversion policy; exported/storage surfaces stay separate.
use super::{Type, surface_type, unsigned_bound};

/// Root ABI eligibility is separate from witness, storage and local ownership.
/// Callers must first validate canonical types and existing static budgets.
pub(super) fn public_interface_type(ty: &Type) -> bool {
    match ty {
        Type::OpaqueString | Type::OpaqueBytes => true,
        // Official FAB tuple encoding supports arities zero through eleven.
        Type::Tuple { elements } => {
            elements.len() <= 11 && elements.iter().all(public_interface_type)
        }
        Type::Vector { element, .. } => public_interface_type(element),
        Type::Struct { fields, .. } => fields
            .iter()
            .all(|field| public_interface_type(&field.ty) && derives_supported(&field.ty)),
        _ => surface_type(ty) || unsigned_bound(ty).is_some(),
    }
}

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
    // Private callers receive the same defensive root bounds as native coercion.
    // Domain validity still precedes identity: malformed native identity values
    // must not become recordable through the shared conversion relation.
    crate::resource_limits::recording_type_nodes(source).is_ok()
        && crate::resource_limits::recording_type_nodes(target).is_ok()
        && value_type(source)
        && value_type(target)
        && crate::pure_semantics::coercion::plan_coercion(source, target).is_ok()
}

/// Recorded witness outputs use the qualified official aligned FAB domain.
/// This includes owned opaque leaves and standalone tuples through arity11;
/// named products retain their stricter generated-derive representation limits.
/// Canonical declarations and size/depth budgets are checked by each caller first.
/// Parameters, public interfaces, storage and fixed-only hashes stay separate.
pub(super) fn witness_result_type(ty: &Type) -> bool {
    super::fab::aligned_fab_type(ty)
}

/// Rust Default implementations cover tuples through arity12. The caller's
/// bounded borrowed type walk includes nested members, even empty vectors.
pub(super) fn default_tuple_traits_supported(nodes: &[&Type]) -> bool {
    nodes
        .iter()
        .all(|ty| !matches!(ty, Type::Tuple { elements } if elements.len() > 12))
}

/// Default is an operation capability, independent of FAB/interface policies.
/// Canonical named definitions and resource budgets are still caller authority.
pub(super) fn default_value_type(ty: &Type) -> bool {
    value_type(ty)
        && crate::resource_limits::recording_type_nodes(ty)
            .is_ok_and(|nodes| default_tuple_traits_supported(&nodes))
}

#[cfg(test)]
mod coercion_tests;
