// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//  	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Private calibration weights, rounded from retained debug frames. Array order:
//! pure, native, legacy recorded, typed recorded. These are separate owners,
//! not additive execution stages. The score is not measured stack bytes.
//! Exhaustive expression/action families force review when the IR grows.
use super::traversal::Node;
// Calibration-only KiB path weights, rounded upward from retained debug frames.
// Four independent rendering owners; not an admission/profile evaluator.
pub(super) fn frame_cost(node: Node<'_>) -> [usize; 4] {
    use crate::ir::{Expr as E, ReturnPlan as R, StateAction as A};
    match node {
        Node::Type(_) => [6; 4],
        Node::ReturnPlan(R::Let { .. }) => [0, 0, 0, 11],
        Node::ReturnPlan(_) => [0, 0, 0, 8],
        Node::StateAction(a) => [
            0,
            0,
            match a {
                A::Let { .. } => 92,
                A::Sequence { .. } | A::If { .. } | A::CircuitCall { .. } => 17,
                A::Expression { .. } | A::Assert { .. } | A::PureCall { .. } => 14,
                A::NativeWitnessCall { .. }
                | A::CounterIncrement { .. }
                | A::CounterDecrement { .. }
                | A::CounterReset { .. }
                | A::CellWrite { .. }
                | A::SetInsert { .. }
                | A::CellWriteCoin { .. }
                | A::SetInsertCoin { .. }
                | A::SetRemove { .. }
                | A::SetReset { .. }
                | A::ListPushFront { .. }
                | A::ListPopFront { .. }
                | A::ListReset { .. }
                | A::MapInsert { .. }
                | A::MapInsertDefault { .. }
                | A::MapRemove { .. }
                | A::MapReset { .. }
                | A::HistoricMerkleInsertIndexDefault { .. }
                | A::HistoricMerkleInsert { .. }
                | A::HistoricMerkleInsertIndex { .. }
                | A::HistoricMerkleInsertHash { .. }
                | A::HistoricMerkleInsertHashIndex { .. }
                | A::HistoricMerkleResetHistory { .. }
                | A::HistoricMerkleResetToDefault { .. }
                | A::MerkleInsertIndexDefault { .. }
                | A::MerkleInsert { .. }
                | A::MerkleInsertIndex { .. }
                | A::MerkleInsertHash { .. }
                | A::MerkleInsertHashIndex { .. }
                | A::MerkleResetToDefault { .. } => 29,
            },
            match a {
                A::Let { .. } => 36,
                A::CircuitCall { .. } | A::PureCall { .. } => 44,
                A::Sequence { .. }
                | A::If { .. }
                | A::Expression { .. }
                | A::NativeWitnessCall { .. }
                | A::Assert { .. }
                | A::CounterIncrement { .. }
                | A::CounterDecrement { .. }
                | A::CounterReset { .. }
                | A::CellWrite { .. }
                | A::SetInsert { .. }
                | A::CellWriteCoin { .. }
                | A::SetInsertCoin { .. }
                | A::SetRemove { .. }
                | A::SetReset { .. }
                | A::ListPushFront { .. }
                | A::ListPopFront { .. }
                | A::ListReset { .. }
                | A::MapInsert { .. }
                | A::MapInsertDefault { .. }
                | A::MapRemove { .. }
                | A::MapReset { .. }
                | A::HistoricMerkleInsertIndexDefault { .. }
                | A::HistoricMerkleInsert { .. }
                | A::HistoricMerkleInsertIndex { .. }
                | A::HistoricMerkleInsertHash { .. }
                | A::HistoricMerkleInsertHashIndex { .. }
                | A::HistoricMerkleResetHistory { .. }
                | A::HistoricMerkleResetToDefault { .. }
                | A::MerkleInsertIndexDefault { .. }
                | A::MerkleInsert { .. }
                | A::MerkleInsertIndex { .. }
                | A::MerkleInsertHash { .. }
                | A::MerkleInsertHashIndex { .. }
                | A::MerkleResetToDefault { .. } => 33,
            },
        ],
        Node::ConstructorStep(_) => [0, 55, 0, 0],
        Node::Expr(e) => {
            let pure = match e {
                E::Sequence { .. } | E::Assert { .. } | E::Let { .. } | E::If { .. } => 21,
                E::StructField { .. }
                | E::TupleIndex { .. }
                | E::StructLiteral { .. }
                | E::Tuple { .. }
                | E::Vector { .. }
                | E::VectorMap { .. }
                | E::VectorFoldCall { .. }
                | E::Call { .. } => 32,
                E::FieldCast { .. }
                | E::FieldToBytes32 { .. }
                | E::Coerce { .. }
                | E::UnsignedCast { .. } => 12,
                E::Add { .. }
                | E::Subtract { .. }
                | E::Multiply { .. }
                | E::UnsignedAdd { .. }
                | E::UnsignedSubtract { .. }
                | E::UnsignedMultiply { .. }
                | E::Equal { .. }
                | E::NotEqual { .. }
                | E::Compare { .. } => 18,
                E::TransientHash { .. }
                | E::TransientCommit { .. }
                | E::PersistentHash { .. }
                | E::PersistentCommit { .. }
                | E::Keccak256 { .. }
                | E::DegradeToTransient { .. }
                | E::UpgradeFromTransient { .. }
                | E::HashToCurve { .. }
                | E::JubjubPointX { .. }
                | E::JubjubPointY { .. }
                | E::EcAdd { .. }
                | E::ConstructJubjubPoint { .. }
                | E::EcNeg { .. }
                | E::EcMul { .. }
                | E::EcMulGenerator { .. }
                | E::JubjubScalarFromNative { .. } => 39,
                E::Unit
                | E::Default { .. }
                | E::Boolean { .. }
                | E::FieldLiteral { .. }
                | E::BytesLiteral { .. }
                | E::UnsignedLiteral { .. }
                | E::Parameter { .. }
                | E::EnumVariant { .. }
                | E::SetMember { .. }
                | E::MapMember { .. }
                | E::MapLookup { .. }
                | E::ListLength { .. }
                | E::ListIsEmpty { .. }
                | E::ListHead { .. }
                | E::MerkleCheckRoot { .. }
                | E::HistoricMerkleCheckRoot { .. }
                | E::CellRead { .. }
                | E::CounterLessThan { .. }
                | E::CounterRead { .. }
                | E::KernelSelf { .. }
                | E::SetSize { .. }
                | E::SetIsEmpty { .. }
                | E::MapIsEmpty { .. }
                | E::WitnessCall { .. }
                | E::KernelClaim { .. }
                | E::KernelMintShielded { .. }
                | E::CreateZswapInput { .. }
                | E::CreateZswapOutput { .. }
                | E::NativeWitnessCall { .. } => 14,
            };
            let native = match e {
                E::CellRead { .. }
                | E::CounterRead { .. }
                | E::CounterLessThan { .. }
                | E::SetMember { .. }
                | E::SetSize { .. }
                | E::SetIsEmpty { .. }
                | E::MapMember { .. }
                | E::MapLookup { .. }
                | E::MapIsEmpty { .. }
                | E::MerkleCheckRoot { .. }
                | E::HistoricMerkleCheckRoot { .. }
                | E::ListLength { .. }
                | E::ListIsEmpty { .. }
                | E::ListHead { .. } => 51,
                E::KernelSelf { .. }
                | E::KernelClaim { .. }
                | E::KernelMintShielded { .. }
                | E::CreateZswapInput { .. }
                | E::CreateZswapOutput { .. }
                | E::NativeWitnessCall { .. } => 21,
                E::WitnessCall { .. } => 18,
                E::Add { .. }
                | E::Subtract { .. }
                | E::Multiply { .. }
                | E::UnsignedAdd { .. }
                | E::UnsignedSubtract { .. }
                | E::UnsignedMultiply { .. }
                | E::Equal { .. }
                | E::NotEqual { .. }
                | E::Compare { .. } => 22,
                E::Unit
                | E::Default { .. }
                | E::Boolean { .. }
                | E::FieldLiteral { .. }
                | E::BytesLiteral { .. }
                | E::UnsignedLiteral { .. }
                | E::UnsignedCast { .. }
                | E::FieldCast { .. }
                | E::FieldToBytes32 { .. }
                | E::Coerce { .. }
                | E::Parameter { .. }
                | E::EnumVariant { .. }
                | E::StructField { .. }
                | E::TupleIndex { .. }
                | E::StructLiteral { .. }
                | E::Tuple { .. }
                | E::Vector { .. }
                | E::VectorMap { .. }
                | E::VectorFoldCall { .. }
                | E::If { .. }
                | E::Let { .. }
                | E::Sequence { .. }
                | E::Assert { .. }
                | E::Call { .. }
                | E::TransientHash { .. }
                | E::TransientCommit { .. }
                | E::PersistentHash { .. }
                | E::Keccak256 { .. }
                | E::PersistentCommit { .. }
                | E::DegradeToTransient { .. }
                | E::UpgradeFromTransient { .. }
                | E::HashToCurve { .. }
                | E::JubjubPointX { .. }
                | E::JubjubPointY { .. }
                | E::EcAdd { .. }
                | E::ConstructJubjubPoint { .. }
                | E::EcNeg { .. }
                | E::EcMul { .. }
                | E::EcMulGenerator { .. }
                | E::JubjubScalarFromNative { .. } => pure + 1,
            }
            .max(pure + 1); // includes pure fallback on an entire subtree conservatively
            let legacy = match e {
                E::If { .. }
                | E::Equal { .. }
                | E::NotEqual { .. }
                | E::Compare { .. }
                | E::Call { .. }
                | E::Let { .. }
                | E::Sequence { .. }
                | E::Assert { .. } => 59,
                E::Unit
                | E::Default { .. }
                | E::Boolean { .. }
                | E::FieldLiteral { .. }
                | E::BytesLiteral { .. }
                | E::UnsignedLiteral { .. }
                | E::UnsignedCast { .. }
                | E::FieldCast { .. }
                | E::FieldToBytes32 { .. }
                | E::Coerce { .. }
                | E::Parameter { .. }
                | E::EnumVariant { .. }
                | E::StructField { .. }
                | E::TupleIndex { .. }
                | E::StructLiteral { .. }
                | E::SetMember { .. }
                | E::MapMember { .. }
                | E::MapLookup { .. }
                | E::ListLength { .. }
                | E::ListIsEmpty { .. }
                | E::ListHead { .. }
                | E::MerkleCheckRoot { .. }
                | E::HistoricMerkleCheckRoot { .. }
                | E::CellRead { .. }
                | E::CounterLessThan { .. }
                | E::CounterRead { .. }
                | E::KernelSelf { .. }
                | E::SetSize { .. }
                | E::SetIsEmpty { .. }
                | E::MapIsEmpty { .. }
                | E::Tuple { .. }
                | E::Vector { .. }
                | E::VectorMap { .. }
                | E::VectorFoldCall { .. }
                | E::TransientHash { .. }
                | E::TransientCommit { .. }
                | E::PersistentHash { .. }
                | E::Keccak256 { .. }
                | E::PersistentCommit { .. }
                | E::DegradeToTransient { .. }
                | E::UpgradeFromTransient { .. }
                | E::HashToCurve { .. }
                | E::JubjubPointX { .. }
                | E::JubjubPointY { .. }
                | E::EcAdd { .. }
                | E::ConstructJubjubPoint { .. }
                | E::EcNeg { .. }
                | E::EcMul { .. }
                | E::EcMulGenerator { .. }
                | E::JubjubScalarFromNative { .. }
                | E::WitnessCall { .. }
                | E::KernelClaim { .. }
                | E::KernelMintShielded { .. }
                | E::CreateZswapInput { .. }
                | E::CreateZswapOutput { .. }
                | E::NativeWitnessCall { .. }
                | E::Add { .. }
                | E::Subtract { .. }
                | E::Multiply { .. }
                | E::UnsignedAdd { .. }
                | E::UnsignedSubtract { .. }
                | E::UnsignedMultiply { .. } => 34,
            };
            let typed = match e {
                // Typed Jubjub dispatch plus ordered operand closure; the shared
                // syntax helper runs only after recursive operands have returned.
                E::EcAdd { .. }
                | E::EcMul { .. }
                | E::EcMulGenerator { .. }
                | E::JubjubScalarFromNative { .. } => 14,
                E::Sequence { .. } | E::Assert { .. } | E::Let { .. } | E::If { .. } => 14,
                E::Call { .. } => 21,
                E::StructField { .. } | E::StructLiteral { .. } | E::Tuple { .. } => 7,
                E::FieldCast { .. }
                | E::FieldToBytes32 { .. }
                | E::Coerce { .. }
                | E::UnsignedCast { .. }
                | E::UnsignedMultiply { .. }
                | E::NotEqual { .. }
                | E::JubjubPointX { .. }
                | E::JubjubPointY { .. }
                | E::UnsignedAdd { .. }
                | E::UnsignedSubtract { .. }
                | E::Equal { .. }
                | E::Add { .. } => 23,
                E::PersistentCommit { .. }
                | E::PersistentHash { .. }
                | E::TransientCommit { .. }
                | E::DegradeToTransient { .. }
                | E::TransientHash { .. }
                | E::UpgradeFromTransient { .. } => 14,
                E::Unit
                | E::Parameter { .. }
                | E::Boolean { .. }
                | E::BytesLiteral { .. }
                | E::EnumVariant { .. }
                | E::FieldLiteral { .. }
                | E::UnsignedLiteral { .. }
                | E::Default { .. } => 5,
                E::TupleIndex { .. }
                | E::SetMember { .. }
                | E::MapMember { .. }
                | E::MapLookup { .. }
                | E::ListLength { .. }
                | E::ListIsEmpty { .. }
                | E::ListHead { .. }
                | E::MerkleCheckRoot { .. }
                | E::HistoricMerkleCheckRoot { .. }
                | E::CellRead { .. }
                | E::CounterLessThan { .. }
                | E::CounterRead { .. }
                | E::KernelSelf { .. }
                | E::SetSize { .. }
                | E::SetIsEmpty { .. }
                | E::MapIsEmpty { .. }
                | E::Vector { .. }
                | E::VectorMap { .. }
                | E::VectorFoldCall { .. }
                | E::Compare { .. }
                | E::Keccak256 { .. }
                | E::HashToCurve { .. }
                | E::ConstructJubjubPoint { .. }
                | E::EcNeg { .. }
                | E::WitnessCall { .. }
                | E::KernelClaim { .. }
                | E::KernelMintShielded { .. }
                | E::CreateZswapInput { .. }
                | E::CreateZswapOutput { .. }
                | E::NativeWitnessCall { .. }
                | E::Subtract { .. }
                | E::Multiply { .. } => 11,
            };
            [pure, native, legacy, typed]
        }
        Node::SourceLocation(..)
        | Node::Contract(..)
        | Node::TypeAlias(..)
        | Node::Constructor(..)
        | Node::WitnessDeclaration(..)
        | Node::LedgerField(..)
        | Node::LedgerFieldKind(..)
        | Node::StatefulCircuit(..)
        | Node::StateReturn(..)
        | Node::NativeWitnessBuiltin(..)
        | Node::CounterAmount(..)
        | Node::PureCircuit(..)
        | Node::Parameter(..)
        | Node::LocalBinding(..)
        | Node::StructField(..)
        | Node::KernelClaimKind(..)
        | Node::ComparisonOperator(..)
        | Node::Text(..) => [0; 4],
    }
}
pub(super) fn add_cost(a: [usize; 4], b: [usize; 4]) -> [usize; 4] {
    std::array::from_fn(|i| a[i].saturating_add(b[i]))
}
pub(super) fn max_cost(a: [usize; 4], b: [usize; 4]) -> [usize; 4] {
    std::array::from_fn(|i| a[i].max(b[i]))
}
