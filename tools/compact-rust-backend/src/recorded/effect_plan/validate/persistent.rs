// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Persistent hashing uses fixed-byte FAB products, not arbitrary local values.
use crate::ir::Type;

/// Called only after the used type has passed canonical and resource validation.
/// Standalone tuple arity follows pinned ledger FAB Aligned/Into<Value> (0..=11).
/// Named structs retain the separate representation-derive limit (tuple arity 8).
/// Existing byte/layout limits apply to the whole used type. Empty structs/vectors
/// and zero-byte leaves retain their
/// ordinary empty FAB representation. Unit is the empty product emitted by the
/// frontend for a zero-length vector; no Field/opaque/curve leaf is admitted.
pub(in crate::recorded::effect_plan) fn hash_input_type(ty: &Type) -> bool {
    match ty {
        Type::Bytes { .. } | Type::Unit => true,
        Type::Struct { fields, .. } => fields.iter().all(|field| hash_input_type(&field.ty)),
        Type::Vector { element, .. } => hash_input_type(element),
        Type::Tuple { elements } => elements.len() <= 11 && elements.iter().all(hash_input_type),
        _ => false,
    }
}
