// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Original queries bodies, without new checks or traversal.
use super::super::*;

impl Lowerer<'_> {
    pub(in crate::recorded::effect_plan::lower::borrowed_operations) fn old_query_operation(
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
            Expr::SetSize { field, index } => {
                let (slot, _) = self.set(field, *index, path)?;
                (Operation::SetSize { slot }, uint("18446744073709551615"))
            }
            Expr::SetIsEmpty { field, index } => {
                let (slot, _) = self.set(field, *index, path)?;
                (Operation::SetIsEmpty { slot }, Type::Boolean)
            }
            Expr::ListLength { .. } | Expr::ListIsEmpty { .. } | Expr::ListHead { .. } => {
                self.list_expression(expression, path)?
            }
            Expr::CellRead { field, index } => {
                let (slot, ty) = self.cell(field, *index, path)?;
                (Operation::CellRead(slot), ty)
            }
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
            Expr::CounterRead { field, index } => {
                let slot = self.counter(field, *index, path)?;
                (
                    Operation::CounterRead { slot },
                    uint("18446744073709551615"),
                )
            }
            _ => unreachable!("expression dispatcher selects queries"),
        };
        Ok(value)
    }
}
