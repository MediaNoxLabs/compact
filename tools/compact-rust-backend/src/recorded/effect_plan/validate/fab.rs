// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! FAB representation capabilities remain separate from ordinary local ownership.
use super::{Type, value_type};

#[derive(Clone, Copy)]
enum Capability {
    Atoms,
    Aligned,
    FixedAligned,
}

/// Canonical declarations and resource checks precede this representation policy.
/// Official tuple conversion stops at arity11; value_type retains the stricter
/// named-product representation derives, including nested tuple arity8.
fn supports(ty: &Type, capability: Capability) -> bool {
    fn product(ty: &Type, capability: Capability) -> bool {
        match ty {
            Type::OpaqueString | Type::OpaqueBytes => {
                matches!(capability, Capability::Atoms | Capability::Aligned)
            }
            Type::Struct { fields, .. } => fields.iter().all(|f| product(&f.ty, capability)),
            Type::Vector { element, .. } => product(element, capability),
            Type::Tuple { elements } => {
                elements.len() <= 11 && elements.iter().all(|ty| product(ty, capability))
            }
            _ => true,
        }
    }
    value_type(ty) && product(ty, capability)
}

/// Keccak consumes Into<Value> atoms without requiring aligned conversion.
pub(in crate::recorded::effect_plan) fn keccak_input_type(ty: &Type) -> bool {
    supports(ty, Capability::Atoms)
}

/// Qualified Aligned + Into<Value> domain for curve/transient hashes. Opaque
/// wrappers carry Compress alignment; source/runtime qualification is separate
/// from Keccak's atom conversion even where the current leaf sets coincide.
pub(in crate::recorded::effect_plan) fn aligned_fab_type(ty: &Type) -> bool {
    supports(ty, Capability::Aligned)
}

/// Persistent hashing's source semantics exclude opaque leaves even when their
/// Rust wrappers implement both traits. Preserve that restriction recursively.
pub(in crate::recorded::effect_plan) fn fixed_aligned_fab_type(ty: &Type) -> bool {
    supports(ty, Capability::FixedAligned)
}
