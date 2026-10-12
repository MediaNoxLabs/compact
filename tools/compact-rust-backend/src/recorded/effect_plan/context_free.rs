// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Semantic purity of typed blocks; independent of public transcript presence.
use super::model::{Binding, Operation};

pub(super) fn bindings(bindings: &[Binding]) -> bool {
    let mut pending = vec![bindings];
    while let Some(bindings) = pending.pop() {
        for binding in bindings {
            match &binding.operation {
                Operation::VectorMap { body, .. } => pending.push(&body.bindings),
                Operation::If {
                    then, otherwise, ..
                } => {
                    pending.push(&then.bindings);
                    pending.push(&otherwise.bindings);
                }
                // The complete body of a PureCall has already been certified context-free.
                Operation::PureCall { .. }
                | Operation::Assert { .. }
                | Operation::Literal(_)
                | Operation::EnumVariant { .. }
                | Operation::Default
                | Operation::StructConstruct { .. }
                | Operation::StructProject { .. }
                | Operation::TupleProject { .. }
                | Operation::Tuple { .. }
                | Operation::Vector { .. }
                | Operation::PersistentCommit { .. }
                | Operation::FieldToBytes32(_)
                | Operation::PersistentHash(_)
                | Operation::Keccak256(_)
                | Operation::HashToCurve(_)
                | Operation::DegradeToTransient(_)
                | Operation::UpgradeFromTransient(_)
                | Operation::TransientHash(_)
                | Operation::TransientCommit { .. }
                | Operation::Add(_, _)
                | Operation::Subtract(_, _)
                | Operation::Multiply(_, _)
                | Operation::UnsignedAdd(_, _)
                | Operation::UnsignedSubtract(_, _)
                | Operation::UnsignedMultiply(_, _)
                | Operation::Compare { .. }
                | Operation::Equal(_, _)
                | Operation::NotEqual(_, _)
                | Operation::JubjubPointX(_)
                | Operation::JubjubPointY(_)
                | Operation::JubjubScalarFromNative(_)
                | Operation::EcMulGenerator(_)
                | Operation::EcNeg(_)
                | Operation::ConstructJubjubPoint { .. }
                | Operation::EcMul { .. }
                | Operation::EcAdd { .. }
                | Operation::Convert { .. } => {}
                Operation::Call { .. }
                | Operation::Witness { .. }
                | Operation::OwnPublicKey
                | Operation::CreateZswapInput { .. }
                | Operation::CreateZswapOutput { .. }
                | Operation::KernelSelf
                | Operation::KernelClaim { .. }
                | Operation::KernelMintShielded { .. }
                | Operation::List { .. }
                | Operation::Merkle { .. }
                | Operation::CellRead(_)
                | Operation::CellWrite { .. }
                | Operation::CellWriteCoin { .. }
                | Operation::MapMember { .. }
                | Operation::MapLookup { .. }
                | Operation::MapInsert { .. }
                | Operation::MapRemove { .. }
                | Operation::MapInsertDefault { .. }
                | Operation::MapReset { .. }
                | Operation::MapSize { .. }
                | Operation::MapIsEmpty { .. }
                | Operation::SetMember { .. }
                | Operation::SetInsert { .. }
                | Operation::SetInsertCoin { .. }
                | Operation::SetRemove { .. }
                | Operation::SetReset { .. }
                | Operation::SetSize { .. }
                | Operation::SetIsEmpty { .. }
                | Operation::CounterRead { .. }
                | Operation::CounterIncrement { .. }
                | Operation::CounterDecrement { .. }
                | Operation::CounterReset { .. }
                | Operation::CounterLessThan { .. } => return false,
            }
        }
    }
    true
}
