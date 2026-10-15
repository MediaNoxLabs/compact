// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Close named-type dependencies introduced by generated witness ledger views.
use std::collections::BTreeMap;

use crate::ir::{Contract, LedgerFieldKind, StructField, Type};
use crate::{RenderError, collect_named_types, located};

/// The metered Merkle view uses this exact standard-library representation.
fn merkle_digest() -> Type {
    Type::Struct {
        name: "MerkleTreeDigest".into(),
        fields: vec![StructField {
            name: "field".into(),
            ty: Type::Field,
        }],
    }
}

pub(crate) fn is_witness_merkle_digest(name: &str, fields: &[StructField]) -> bool {
    name == "MerkleTreeDigest"
        && matches!(fields, [field] if field.name == "field" && field.ty == Type::Field)
}

/// Returns whether generated witness views require the digest conversion.
/// Use the ordinary collector so explicit declarations keep their authority:
/// matching types deduplicate, while conflicting names/layouts remain errors.
pub(crate) fn collect_witness_view_types(
    contract: &Contract,
    structs: &mut BTreeMap<String, Vec<StructField>>,
    enums: &mut BTreeMap<String, Vec<String>>,
) -> Result<bool, RenderError> {
    if contract.witnesses.is_empty() {
        return Ok(false);
    }
    let Some(field) = contract.ledger_fields.iter().find(|field| {
        matches!(
            field.declaration,
            LedgerFieldKind::MerkleTree { .. } | LedgerFieldKind::HistoricMerkleTree { .. }
        )
    }) else {
        return Ok(false);
    };
    located(field.source.as_ref(), || {
        collect_named_types(&merkle_digest(), structs, enums)
    })?;
    Ok(true)
}

#[cfg(test)]
mod tests;
