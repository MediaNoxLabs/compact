// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Small recursive dispatch; domain owners retain exact checks and child order.
use super::super::validate;
use super::*;
mod assertions;
mod conversions;
mod literals;
mod persistent;
mod queries;
mod value;
mod witness;

type OperationLowerer<'env> = fn(
    &mut Lowerer<'env>,
    &Expr,
    &Scope,
    &mut Vec<Binding>,
    &str,
) -> Result<(Operation, Type), RecordingGap>;
type ValueLowerer<'env> =
    fn(&mut Lowerer<'env>, &Expr, &Scope, &mut Vec<Binding>, &str) -> Result<Value, RecordingGap>;
impl Lowerer<'_> {
    pub(super) fn expression(
        &mut self,
        expression: &Expr,
        scope: &Scope,
        bindings: &mut Vec<Binding>,
        path: &str,
    ) -> Result<Value, RecordingGap> {
        self.spend(path)?;
        // Preserve original early returns: no additional validation or emit.
        let direct: Option<ValueLowerer<'_>> = match expression {
            Expr::Parameter { .. } => Some(Self::parameter_value),
            Expr::Let { .. } => Some(Self::control_value),
            Expr::Sequence { .. } => Some(Self::control_value),
            Expr::If { .. } => Some(Self::control_value),
            Expr::Coerce { .. } => Some(Self::conversion_value),
            Expr::FieldCast { .. } => Some(Self::conversion_value),
            Expr::Call { .. } => Some(Self::call_value),
            _ => None,
        };
        if let Some(lower) = direct {
            return lower(self, expression, scope, bindings, path).inspect_err(|_| {
                refused(self.observer, RefusalSite::node(Owner::Expr(expression)))
            });
        }
        // Select before calling; operation families share one Result slot.
        let lower: OperationLowerer<'_> = match expression {
            Expr::Unit => Self::literal_operation,
            Expr::Boolean { .. } => Self::literal_operation,
            Expr::FieldLiteral { .. } => Self::literal_operation,
            Expr::UnsignedLiteral { .. } => Self::literal_operation,
            Expr::BytesLiteral { .. }
            | Expr::EnumVariant { .. }
            | Expr::Default { .. }
            | Expr::StructLiteral { .. }
            | Expr::StructField { .. }
            | Expr::TupleIndex { .. }
            | Expr::Tuple { .. }
            | Expr::Vector { .. } => Self::aggregate,
            Expr::VectorMap { .. } => Self::vector_map,
            Expr::CreateZswapInput { .. } | Expr::CreateZswapOutput { .. } => Self::zswap,
            Expr::KernelClaim { .. } | Expr::KernelMintShielded { .. } => Self::kernel,
            Expr::NativeWitnessCall {
                builtin: NativeWitnessBuiltin::OwnPublicKey,
            } => Self::query_operation,
            Expr::KernelSelf { .. } => Self::query_operation,
            Expr::DegradeToTransient { .. }
            | Expr::UpgradeFromTransient { .. }
            | Expr::TransientHash { .. }
            | Expr::HashToCurve { .. }
            | Expr::TransientCommit { .. } => Self::transient_crypto,
            Expr::MerkleCheckRoot { .. } | Expr::HistoricMerkleCheckRoot { .. } => {
                Self::query_operation
            }
            Expr::FieldToBytes32 { .. } => Self::conversion_operation,
            Expr::PersistentHash { .. } | Expr::Keccak256 { .. } => Self::persistent_operation,
            Expr::PersistentCommit { .. } => Self::persistent_operation,
            Expr::MapMember { .. } | Expr::MapLookup { .. } | Expr::MapIsEmpty { .. } => {
                Self::map_expression
            }
            Expr::SetMember { .. } => Self::query_operation,
            Expr::SetSize { .. } => Self::query_operation,
            Expr::SetIsEmpty { .. } => Self::query_operation,
            Expr::ListLength { .. } | Expr::ListIsEmpty { .. } | Expr::ListHead { .. } => {
                Self::query_operation
            }
            Expr::CellRead { .. } => Self::query_operation,
            Expr::CounterLessThan { .. } => Self::query_operation,
            Expr::CounterRead { .. } => Self::query_operation,
            Expr::JubjubPointX { .. }
            | Expr::JubjubPointY { .. }
            | Expr::JubjubScalarFromNative { .. }
            | Expr::EcMulGenerator { .. }
            | Expr::EcNeg { .. }
            | Expr::ConstructJubjubPoint { .. }
            | Expr::EcMul { .. }
            | Expr::EcAdd { .. } => Self::jubjub_selected,
            Expr::Add { .. }
            | Expr::Subtract { .. }
            | Expr::Multiply { .. }
            | Expr::Compare { .. }
            | Expr::Equal { .. }
            | Expr::NotEqual { .. }
            | Expr::UnsignedAdd { .. }
            | Expr::UnsignedSubtract { .. }
            | Expr::UnsignedMultiply { .. } => Self::binary,
            Expr::UnsignedCast { .. } => Self::conversion_operation,
            Expr::Assert { .. } => Self::assertion_operation,
            Expr::WitnessCall { .. } => Self::witness_operation,
            _ => return Err(self.unsupported_expression(expression, path)),
        };
        let (operation, ty) = lower(self, expression, scope, bindings, path)
            .inspect_err(|_| refused(self.observer, RefusalSite::node(Owner::Expr(expression))))?;
        // This is an owned inferred output, not an original Type occurrence.
        self.declarations
            .types()
            .validate_at(&ty, path)
            .inspect_err(|_| refused(self.observer, RefusalSite::unavailable()))?;
        Ok(self.emit(operation, ty, bindings, path))
    }
    fn jubjub_selected(
        &mut self,
        expression: &Expr,
        scope: &Scope,
        bindings: &mut Vec<Binding>,
        path: &str,
    ) -> Result<(Operation, Type), RecordingGap> {
        let lower: OperationLowerer<'_> = if matches!(
            expression,
            Expr::JubjubPointX { .. } | Expr::JubjubPointY { .. }
        ) {
            Self::jubjub_coordinate
        } else if matches!(expression, Expr::ConstructJubjubPoint { .. }) {
            Self::jubjub_constructor
        } else {
            Self::jubjub
        };
        lower(self, expression, scope, bindings, path)
    }
}
#[cfg(test)]
mod dispatch_tests;

#[cfg(test)]
mod boundary_tests;
