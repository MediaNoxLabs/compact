// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Declared trees and ordered operands; no contract-shape recognition.
use super::super::validate;
use super::*;

/// Borrowed source operands keep traversal, validation and syntax separate.
enum Mutation<'a> {
    Insert(&'a Expr, Option<&'a Expr>),
    InsertHash(&'a Expr, Option<&'a Expr>),
    InsertDefault(&'a Expr),
    ResetDefault,
    ResetHistory,
}

impl Lowerer<'_> {
    fn merkle_slot(
        &self,
        name: &str,
        index: u8,
        historic: bool,
        path: &str,
    ) -> Result<(MerkleSlot, Type), RecordingGap> {
        let (declaration, field) = self.field(name, index, path)?;
        let (depth, ty, declared_historic) = match &field.declaration {
            LedgerFieldKind::MerkleTree { depth, ty } => (*depth, ty, false),
            LedgerFieldKind::HistoricMerkleTree { depth, ty } => (*depth, ty, true),
            _ => return Err(invalid(path, "expected Merkle tree declaration")),
        };
        if historic != declared_historic || !(2..=32).contains(&depth) {
            return Err(invalid(path, "Merkle tree kind or depth mismatch"));
        }
        self.declarations.types().validate_at(ty, path)?;
        if !validate::merkle_leaf_type(ty) {
            return Err(invalid(path, "unsupported Merkle leaf type"));
        }
        Ok((
            MerkleSlot {
                declaration,
                index,
                depth,
                historic,
            },
            ty.clone(),
        ))
    }

    fn merkle_operand(
        &mut self,
        expression: &Expr,
        expected: &Type,
        scope: &Scope,
        bindings: &mut Vec<Binding>,
        path: &str,
    ) -> Result<Value, RecordingGap> {
        let value = self.expression(expression, scope, bindings, path)?;
        if value.ty != *expected {
            return Err(invalid(
                path,
                "Merkle operand type differs from operation signature",
            ));
        }
        Ok(value)
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "query carries declared tree identity and ordered lexical context"
    )]
    pub(super) fn merkle_query(
        &mut self,
        field: &str,
        index: u8,
        historic: bool,
        root: Option<&Expr>,
        scope: &Scope,
        bindings: &mut Vec<Binding>,
        path: &str,
    ) -> Result<(Operation, Type), RecordingGap> {
        let (slot, _) = self.merkle_slot(field, index, historic, path)?;
        let operation = match root {
            Some(root) => MerkleOperation::CheckRoot {
                root: self.merkle_operand(
                    root,
                    &validate::root_type(),
                    scope,
                    bindings,
                    &format!("{path}.root"),
                )?,
            },
            None => MerkleOperation::IsFull,
        };
        Ok((Operation::Merkle { slot, operation }, Type::Boolean))
    }

    pub(super) fn merkle_action(
        &mut self,
        action: &StateAction,
        scope: &Scope,
        bindings: &mut Vec<Binding>,
        path: &str,
    ) -> Result<(), RecordingGap> {
        use StateAction::*;
        let (field, index, historic, mutation) = match action {
            MerkleInsert {
                field,
                index,
                value,
            } => (field, *index, false, Mutation::Insert(value, None)),
            HistoricMerkleInsert {
                field,
                index,
                value,
            } => (field, *index, true, Mutation::Insert(value, None)),
            MerkleInsertIndex {
                field,
                index,
                value,
                position,
            } => (
                field,
                *index,
                false,
                Mutation::Insert(value, Some(position)),
            ),
            HistoricMerkleInsertIndex {
                field,
                index,
                value,
                position,
            } => (field, *index, true, Mutation::Insert(value, Some(position))),
            MerkleInsertHash { field, index, hash } => {
                (field, *index, false, Mutation::InsertHash(hash, None))
            }
            HistoricMerkleInsertHash { field, index, hash } => {
                (field, *index, true, Mutation::InsertHash(hash, None))
            }
            MerkleInsertHashIndex {
                field,
                index,
                hash,
                position,
            } => (
                field,
                *index,
                false,
                Mutation::InsertHash(hash, Some(position)),
            ),
            HistoricMerkleInsertHashIndex {
                field,
                index,
                hash,
                position,
            } => (
                field,
                *index,
                true,
                Mutation::InsertHash(hash, Some(position)),
            ),
            MerkleInsertIndexDefault {
                field,
                index,
                position,
            } => (field, *index, false, Mutation::InsertDefault(position)),
            HistoricMerkleInsertIndexDefault {
                field,
                index,
                position,
            } => (field, *index, true, Mutation::InsertDefault(position)),
            MerkleResetToDefault { field, index } => (field, *index, false, Mutation::ResetDefault),
            HistoricMerkleResetToDefault { field, index } => {
                (field, *index, true, Mutation::ResetDefault)
            }
            HistoricMerkleResetHistory { field, index } => {
                (field, *index, true, Mutation::ResetHistory)
            }
            _ => unreachable!("Merkle action dispatch"),
        };
        let (slot, leaf_type) = self.merkle_slot(field, index, historic, path)?;
        let operation = match mutation {
            Mutation::Insert(expression, position) | Mutation::InsertHash(expression, position) => {
                let hash = matches!(mutation, Mutation::InsertHash(..));
                let member = if hash { "hash" } else { "value" };
                let expected = if hash {
                    Type::Bytes { length: 32 }
                } else {
                    leaf_type
                };
                // Compact evaluates the leaf/hash before the position, exactly once.
                let value = self.merkle_operand(
                    expression,
                    &expected,
                    scope,
                    bindings,
                    &format!("{path}.{member}"),
                )?;
                let position = position
                    .map(|expression| {
                        self.merkle_operand(
                            expression,
                            &uint("18446744073709551615"),
                            scope,
                            bindings,
                            &format!("{path}.position"),
                        )
                    })
                    .transpose()?;
                if hash {
                    MerkleOperation::InsertHash {
                        hash: value,
                        position,
                    }
                } else {
                    MerkleOperation::Insert { value, position }
                }
            }
            Mutation::InsertDefault(position) => MerkleOperation::InsertDefault {
                position: self.merkle_operand(
                    position,
                    &uint("18446744073709551615"),
                    scope,
                    bindings,
                    &format!("{path}.position"),
                )?,
            },
            Mutation::ResetDefault => MerkleOperation::ResetDefault,
            Mutation::ResetHistory => MerkleOperation::ResetHistory,
        };
        self.emit(
            Operation::Merkle { slot, operation },
            Type::Unit,
            bindings,
            path,
        );
        Ok(())
    }
}
