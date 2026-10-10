// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Independently bind slot metadata, operand definitions and tree operation signatures.
use super::*;

pub(in crate::recorded::effect_plan) fn root_type() -> Type {
    Type::Struct {
        name: "MerkleTreeDigest".into(),
        fields: vec![crate::ir::StructField {
            name: "field".into(),
            ty: Type::Field,
        }],
    }
}

fn leaf<'a>(
    declarations: &'a Declarations,
    slot: MerkleSlot,
    path: &str,
) -> Result<&'a Type, RecordingGap> {
    let field = declarations
        .field(slot.declaration)
        .ok_or_else(|| invalid(path, "unknown Merkle tree declaration"))?;
    let (depth, ty, historic) = match &field.declaration {
        LedgerFieldKind::MerkleTree { depth, ty } => (*depth, ty, false),
        LedgerFieldKind::HistoricMerkleTree { depth, ty } => (*depth, ty, true),
        _ => return Err(invalid(path, "expected Merkle tree declaration")),
    };
    if field.index != slot.index
        || historic != slot.historic
        || depth != slot.depth
        || !(2..=32).contains(&depth)
    {
        return Err(invalid(
            path,
            "Merkle declaration identity, kind or depth mismatch",
        ));
    }
    declarations.types().validate_at(ty, path)?;
    if !storage_value_type(ty) {
        return Err(invalid(path, "unsupported Merkle leaf type"));
    }
    Ok(ty)
}

pub(super) fn validate(
    slot: MerkleSlot,
    operation: &MerkleOperation,
    result: &Type,
    declarations: &Declarations,
    scope: &HashMap<usize, Type>,
    path: &str,
) -> Result<bool, RecordingGap> {
    let leaf_type = leaf(declarations, slot, path)?;
    let operand = |value: &Value, expected: &Type| -> Result<bool, RecordingGap> {
        used(value, scope, declarations, path)?;
        Ok(value.ty == *expected)
    };
    let position = |position: &Option<Value>| -> Result<bool, RecordingGap> {
        position
            .as_ref()
            .map(|value| operand(value, &uint("18446744073709551615")))
            .unwrap_or(Ok(true))
    };
    let valid = match operation {
        MerkleOperation::Insert {
            value,
            position: index,
        } => operand(value, leaf_type)? && position(index)? && *result == Type::Unit,
        MerkleOperation::InsertHash {
            hash,
            position: index,
        } => {
            operand(hash, &Type::Bytes { length: 32 })? && position(index)? && *result == Type::Unit
        }
        MerkleOperation::InsertDefault { position } => {
            operand(position, &uint("18446744073709551615"))? && *result == Type::Unit
        }
        MerkleOperation::ResetDefault => *result == Type::Unit,
        MerkleOperation::ResetHistory => slot.historic && *result == Type::Unit,
        MerkleOperation::CheckRoot { root } => {
            operand(root, &root_type())? && *result == Type::Boolean
        }
        MerkleOperation::IsFull => *result == Type::Boolean,
    };
    Ok(valid)
}
