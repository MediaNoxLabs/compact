// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Exact dynamic encoding policy, separate from tuple capability and transcripts.
//! Callers retain existing borrowed resource gates. LedgerMap is intentionally
//! opaque to this predicate even when a tuple-policy scan visits its contents.
use crate::ir::Type;

pub(crate) fn contains_opaque(ty: &Type) -> bool {
    match ty {
        Type::OpaqueString | Type::OpaqueBytes => true,
        Type::Struct { fields, .. } => fields.iter().any(|field| contains_opaque(&field.ty)),
        Type::Tuple { elements } => elements.iter().any(contains_opaque),
        Type::Vector { element, .. } => contains_opaque(element),
        _ => false,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DynamicInputEncoding {
    Direct,
    Exact,
}
impl DynamicInputEncoding {
    pub(crate) fn for_type(ty: &Type) -> Self {
        if contains_opaque(ty) {
            Self::Exact
        } else {
            Self::Direct
        }
    }
}
