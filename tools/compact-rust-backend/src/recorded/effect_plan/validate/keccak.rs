// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Keccak consumes FAB atoms without requiring fixed alignment or byte-only leaves.
use super::{Type, value_type};

/// Used types must first pass canonical declaration and resource validation.
/// Official Into<Value> supports standalone tuples through arity11. Named
/// products retain their stricter representation-derive rules via value_type.
pub(in crate::recorded::effect_plan) fn keccak_input_type(ty: &Type) -> bool {
    fn encodable_product(ty: &Type) -> bool {
        match ty {
            Type::Struct { fields, .. } => fields.iter().all(|f| encodable_product(&f.ty)),
            Type::Vector { element, .. } => encodable_product(element),
            Type::Tuple { elements } => {
                elements.len() <= 11 && elements.iter().all(encodable_product)
            }
            _ => true,
        }
    }
    value_type(ty) && encodable_product(ty)
}
