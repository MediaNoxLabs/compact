// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! One exhaustive source variant map, instantiated for two native output views.
//! The family owner checks children; projection runs only after it returns.
//! No recording profile or syntax-free checker is selected here.

macro_rules! dispatch {
    ($expr:expr, $parameters:expr, $circuits:expr; $control:ident, $value:ident) => {{
        // Keep one post-match Result slot for all ordinary value families.
        // Per-arm adapter arguments inflate unoptimized dispatcher frames.
        let result = match $expr {
            $crate::ir::Expr::Assert { .. }
            | $crate::ir::Expr::Sequence { .. }
            | $crate::ir::Expr::If { .. }
            | $crate::ir::Expr::Let { .. } => {
                return $control(
                    $expr,
                    $crate::pure_control_fragment($expr, $parameters, $circuits),
                );
            }
            $crate::ir::Expr::Unit => $crate::pure_leaf_expression($expr, $parameters, $circuits),
            $crate::ir::Expr::Default { .. } => {
                $crate::pure_leaf_expression($expr, $parameters, $circuits)
            }
            $crate::ir::Expr::Boolean { .. } => {
                $crate::pure_leaf_expression($expr, $parameters, $circuits)
            }
            $crate::ir::Expr::FieldLiteral { .. } => {
                $crate::pure_leaf_expression($expr, $parameters, $circuits)
            }
            $crate::ir::Expr::BytesLiteral { .. } => {
                $crate::pure_leaf_expression($expr, $parameters, $circuits)
            }
            $crate::ir::Expr::UnsignedLiteral { .. } => {
                $crate::pure_leaf_expression($expr, $parameters, $circuits)
            }
            $crate::ir::Expr::Parameter { .. } => {
                $crate::pure_leaf_expression($expr, $parameters, $circuits)
            }
            $crate::ir::Expr::EnumVariant { .. } => {
                $crate::pure_leaf_expression($expr, $parameters, $circuits)
            }
            $crate::ir::Expr::StructField { .. } => {
                $crate::pure_aggregate_expression($expr, $parameters, $circuits)
            }
            $crate::ir::Expr::TupleIndex { .. } => {
                $crate::pure_aggregate_expression($expr, $parameters, $circuits)
            }
            $crate::ir::Expr::StructLiteral { .. } => {
                $crate::pure_aggregate_expression($expr, $parameters, $circuits)
            }
            $crate::ir::Expr::Tuple { .. } => {
                $crate::pure_aggregate_expression($expr, $parameters, $circuits)
            }
            $crate::ir::Expr::Vector { .. } => {
                $crate::pure_aggregate_expression($expr, $parameters, $circuits)
            }
            $crate::ir::Expr::VectorMap { .. } => {
                $crate::pure_aggregate_expression($expr, $parameters, $circuits)
            }
            $crate::ir::Expr::VectorFoldCall { .. } => {
                $crate::pure_aggregate_expression($expr, $parameters, $circuits)
            }
            $crate::ir::Expr::Call { .. } => {
                $crate::pure_aggregate_expression($expr, $parameters, $circuits)
            }
            $crate::ir::Expr::TransientHash { .. } => {
                $crate::pure_crypto_expression($expr, $parameters, $circuits)
            }
            $crate::ir::Expr::TransientCommit { .. } => {
                $crate::pure_crypto_expression($expr, $parameters, $circuits)
            }
            $crate::ir::Expr::PersistentHash { .. } => {
                $crate::pure_crypto_expression($expr, $parameters, $circuits)
            }
            $crate::ir::Expr::Keccak256 { .. } => {
                $crate::pure_crypto_expression($expr, $parameters, $circuits)
            }
            $crate::ir::Expr::PersistentCommit { .. } => {
                $crate::pure_crypto_expression($expr, $parameters, $circuits)
            }
            $crate::ir::Expr::DegradeToTransient { .. } => {
                $crate::pure_crypto_expression($expr, $parameters, $circuits)
            }
            $crate::ir::Expr::UpgradeFromTransient { .. } => {
                $crate::pure_crypto_expression($expr, $parameters, $circuits)
            }
            $crate::ir::Expr::HashToCurve { .. } => {
                $crate::pure_crypto_expression($expr, $parameters, $circuits)
            }
            $crate::ir::Expr::JubjubPointX { .. } | $crate::ir::Expr::JubjubPointY { .. } => {
                $crate::pure_crypto_expression($expr, $parameters, $circuits)
            }
            $crate::ir::Expr::EcAdd { .. } => {
                $crate::pure_crypto_expression($expr, $parameters, $circuits)
            }
            $crate::ir::Expr::ConstructJubjubPoint { .. } => {
                $crate::pure_crypto_expression($expr, $parameters, $circuits)
            }
            $crate::ir::Expr::EcNeg { .. } => {
                $crate::pure_crypto_expression($expr, $parameters, $circuits)
            }
            $crate::ir::Expr::EcMul { .. } => {
                $crate::pure_crypto_expression($expr, $parameters, $circuits)
            }
            $crate::ir::Expr::EcMulGenerator { .. } => {
                $crate::pure_crypto_expression($expr, $parameters, $circuits)
            }
            $crate::ir::Expr::JubjubScalarFromNative { .. } => {
                $crate::pure_crypto_expression($expr, $parameters, $circuits)
            }
            $crate::ir::Expr::WitnessCall { .. }
            | $crate::ir::Expr::KernelClaim { .. }
            | $crate::ir::Expr::KernelMintShielded { .. }
            | $crate::ir::Expr::CreateZswapInput { .. }
            | $crate::ir::Expr::CreateZswapOutput { .. }
            | $crate::ir::Expr::NativeWitnessCall { .. }
            | $crate::ir::Expr::SetMember { .. }
            | $crate::ir::Expr::MapMember { .. }
            | $crate::ir::Expr::MapLookup { .. }
            | $crate::ir::Expr::MerkleCheckRoot { .. }
            | $crate::ir::Expr::HistoricMerkleCheckRoot { .. }
            | $crate::ir::Expr::SetIsEmpty { .. }
            | $crate::ir::Expr::SetSize { .. }
            | $crate::ir::Expr::MapIsEmpty { .. }
            | $crate::ir::Expr::ListLength { .. }
            | $crate::ir::Expr::ListIsEmpty { .. }
            | $crate::ir::Expr::ListHead { .. }
            | $crate::ir::Expr::CellRead { .. }
            | $crate::ir::Expr::CounterRead { .. }
            | $crate::ir::Expr::CounterLessThan { .. }
            | $crate::ir::Expr::KernelSelf { .. } => {
                $crate::pure_leaf_expression($expr, $parameters, $circuits)
            }
            $crate::ir::Expr::FieldCast { .. } => {
                $crate::pure_conversion_expression($expr, $parameters, $circuits)
            }
            $crate::ir::Expr::FieldToBytes32 { .. } => {
                $crate::pure_conversion_expression($expr, $parameters, $circuits)
            }
            $crate::ir::Expr::Coerce { .. } => {
                $crate::pure_conversion_expression($expr, $parameters, $circuits)
            }
            $crate::ir::Expr::UnsignedCast { .. } => {
                $crate::pure_conversion_expression($expr, $parameters, $circuits)
            }
            $crate::ir::Expr::UnsignedAdd { .. }
            | $crate::ir::Expr::UnsignedSubtract { .. }
            | $crate::ir::Expr::UnsignedMultiply { .. } => {
                $crate::pure_arithmetic_expression($expr, $parameters, $circuits)
            }
            $crate::ir::Expr::Add { .. }
            | $crate::ir::Expr::Subtract { .. }
            | $crate::ir::Expr::Multiply { .. } => {
                $crate::pure_arithmetic_expression($expr, $parameters, $circuits)
            }
            $crate::ir::Expr::Equal { .. } | $crate::ir::Expr::NotEqual { .. } => {
                $crate::pure_arithmetic_expression($expr, $parameters, $circuits)
            }
            $crate::ir::Expr::Compare { .. } => {
                $crate::pure_arithmetic_expression($expr, $parameters, $circuits)
            }
        };
        $value($expr, result)
    }};
}

pub(crate) use dispatch;
