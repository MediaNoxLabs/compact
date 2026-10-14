// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Original queries bodies, without new checks or traversal.
use super::super::borrowed_operations::LeafQuery;
use super::*;

impl Lowerer<'_> {
    pub(super) fn query_operation(
        &mut self,
        expression: &Expr,
        scope: &Scope,
        bindings: &mut Vec<Binding>,
        path: &str,
    ) -> Result<(Operation, Type), RecordingGap> {
        let value = match expression {
            Expr::NativeWitnessCall {
                builtin: NativeWitnessBuiltin::OwnPublicKey,
            } => {
                let ty = NativeWitnessBuiltin::OwnPublicKey.result_type();
                self.declarations.types().validate_at(&ty, path)?;
                (Operation::OwnPublicKey, ty)
            }
            Expr::KernelSelf { ty } => {
                self.declarations.types().validate_at(ty, path)?;
                if *ty != crate::ir::contract_address_type() {
                    return Err(invalid(
                        path,
                        "kernel self requires canonical ContractAddress",
                    ));
                }
                (Operation::KernelSelf, ty.clone())
            }
            Expr::MerkleCheckRoot { field, index, root }
            | Expr::HistoricMerkleCheckRoot { field, index, root } => self.merkle_query(
                field,
                *index,
                matches!(expression, Expr::HistoricMerkleCheckRoot { .. }),
                Some(root),
                scope,
                bindings,
                path,
            )?,
            Expr::SetMember {
                field,
                index,
                value,
            } => self.set_member(field, *index, value, scope, bindings, path)?,
            Expr::SetSize { field, index } => self.leaf_query_operation(
                LeafQuery::SetSize {
                    field,
                    index: *index,
                },
                path,
            )?,
            Expr::SetIsEmpty { field, index } => self.leaf_query_operation(
                LeafQuery::SetIsEmpty {
                    field,
                    index: *index,
                },
                path,
            )?,
            Expr::ListLength { .. } | Expr::ListIsEmpty { .. } | Expr::ListHead { .. } => {
                self.list_expression(expression, path)?
            }
            Expr::CellRead { field, index } => self.leaf_query_operation(
                LeafQuery::CellRead {
                    field,
                    index: *index,
                },
                path,
            )?,
            Expr::CounterLessThan {
                field,
                index,
                threshold,
            } => {
                let slot = self.counter(field, *index, path)?;
                let threshold_path = format!("{path}.threshold");
                let threshold = self.expression(threshold, scope, bindings, &threshold_path)?;
                if threshold.ty != uint("18446744073709551615") {
                    return Err(invalid(
                        &threshold_path,
                        "counter threshold must be Uint<64>",
                    ));
                }
                (
                    Operation::CounterLessThan { slot, threshold },
                    Type::Boolean,
                )
            }
            Expr::CounterRead { field, index } => self.leaf_query_operation(
                LeafQuery::CounterRead {
                    field,
                    index: *index,
                },
                path,
            )?,
            _ => unreachable!("expression dispatcher selects queries"),
        };
        Ok(value)
    }

    /// The existing four no-operand checks, shared without synthetic Expr nodes.
    pub(in crate::recorded::effect_plan::lower) fn leaf_query_operation(
        &mut self,
        query: LeafQuery<'_>,
        path: &str,
    ) -> Result<(Operation, Type), RecordingGap> {
        Ok(match query {
            LeafQuery::CellRead { field, index } => {
                let (slot, ty) = self.cell(field, index, path)?;
                (Operation::CellRead(slot), ty)
            }
            LeafQuery::CounterRead { field, index } => {
                let slot = self.counter(field, index, path)?;
                (
                    Operation::CounterRead { slot },
                    uint("18446744073709551615"),
                )
            }
            LeafQuery::SetSize { field, index } => {
                let (slot, _) = self.set(field, index, path)?;
                (Operation::SetSize { slot }, uint("18446744073709551615"))
            }
            LeafQuery::SetIsEmpty { field, index } => {
                let (slot, _) = self.set(field, index, path)?;
                (Operation::SetIsEmpty { slot }, Type::Boolean)
            }
        })
    }
}
