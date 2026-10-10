// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Fixed FAB representation domain; opaque/dynamic inputs need separate qualification.
use super::{Type, value_type};

/// Used types are canonically resolved and resource checked before this policy.
/// Standalone FAB tuples implement Aligned/Into<Value> through arity11; named
/// structs retain their stricter representation-derive rules through value_type.
pub(in crate::recorded::effect_plan) fn fixed_fab_type(ty: &Type) -> bool {
    fn fixed_layout(ty: &Type) -> bool {
        match ty {
            Type::OpaqueString | Type::OpaqueBytes => false,
            Type::Struct { fields, .. } => fields.iter().all(|f| fixed_layout(&f.ty)),
            Type::Vector { element, .. } => fixed_layout(element),
            Type::Tuple { elements } => elements.len() <= 11 && elements.iter().all(fixed_layout),
            _ => true,
        }
    }
    // Validate the full value domain once, avoiding repeated subtree walks.
    value_type(ty) && fixed_layout(ty)
}
