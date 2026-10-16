// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Nonrecursive operation signatures stay off nested block traversal frames.
use super::*;

pub(super) fn validate(
    binding: &Binding,
    declarations: &Declarations,
    scope: &HashMap<usize, Type>,
    state: &mut Validation,
) -> Result<bool, RecordingGap> {
    let path = &binding.path;
    let ty = &binding.value.ty;
    let use_value = |value: &Value| used(value, scope, declarations, path);
    Ok(match &binding.operation {
        Operation::Literal(literal) => match literal {
            Literal::Unit => *ty == Type::Unit,
            Literal::Boolean(_) => *ty == Type::Boolean,
            Literal::Field(bytes) => {
                *ty == Type::Field
                    && midnight_transient_crypto::curve::Fr::from_le_bytes(bytes).is_some()
            }
            Literal::Unsigned(n) => bound(ty).is_some_and(|max| *n <= max),
            Literal::WideUnsigned(bytes) => wide_literal_fits(bytes, ty),
            Literal::Bytes(bytes) => {
                *ty == (Type::Bytes {
                    length: bytes.len(),
                })
            }
        },
        Operation::EnumVariant { .. }
        | Operation::Default
        | Operation::StructConstruct { .. }
        | Operation::StructProject { .. }
        | Operation::TupleProject { .. }
        | Operation::Tuple { .. }
        | Operation::Vector { .. } => aggregates::validate(binding, declarations, scope)?,
        Operation::CreateZswapInput { coin } => {
            use_value(coin)?;
            coin.ty == crate::coin_shapes::qualified_coin_type() && *ty == Type::Unit
        }
        Operation::CreateZswapOutput { coin, recipient } => {
            use_value(coin)?;
            use_value(recipient)?;
            coin.ty == crate::coin_shapes::shielded_coin_type()
                && recipient.ty == crate::coin_shapes::shielded_recipient_type()
                && *ty == Type::Unit
        }
        Operation::OwnPublicKey => {
            *ty == crate::ir::NativeWitnessBuiltin::OwnPublicKey.result_type()
        }
        Operation::KernelSelf => *ty == crate::ir::contract_address_type(),
        Operation::KernelClaim { value, .. } => {
            use_value(value)?;
            value.ty == (Type::Bytes { length: 32 }) && *ty == Type::Unit
        }
        Operation::KernelMintShielded { domain, amount } => {
            use_value(domain)?;
            use_value(amount)?;
            domain.ty == (Type::Bytes { length: 32 })
                && amount.ty == uint("18446744073709551615")
                && *ty == Type::Unit
        }
        Operation::PersistentCommit { value, opening } => {
            use_value(value)?;
            use_value(opening)?;
            fixed_aligned_fab_type(&value.ty)
                && opening.ty == (Type::Bytes { length: 32 })
                && *ty == (Type::Bytes { length: 32 })
        }
        Operation::FieldToBytes32(value) => {
            use_value(value)?;
            value.ty == Type::Field && *ty == (Type::Bytes { length: 32 })
        }
        Operation::DegradeToTransient(value) => {
            use_value(value)?;
            value.ty == (Type::Bytes { length: 32 }) && *ty == Type::Field
        }
        Operation::UpgradeFromTransient(value) => {
            use_value(value)?;
            value.ty == Type::Field && *ty == (Type::Bytes { length: 32 })
        }
        Operation::HashToCurve(value) => {
            use_value(value)?;
            aligned_fab_type(&value.ty) && *ty == Type::JubjubPoint
        }
        Operation::TransientHash(value) => {
            use_value(value)?;
            aligned_fab_type(&value.ty) && *ty == Type::Field
        }
        Operation::TransientCommit { value, opening } => {
            use_value(value)?;
            use_value(opening)?;
            aligned_fab_type(&value.ty) && opening.ty == Type::Field && *ty == Type::Field
        }
        Operation::PersistentHash(value) => {
            use_value(value)?;
            fixed_aligned_fab_type(&value.ty) && *ty == (Type::Bytes { length: 32 })
        }
        Operation::Keccak256(value) => {
            use_value(value)?;
            keccak_input_type(&value.ty) && *ty == (Type::Bytes { length: 32 })
        }
        Operation::Add(a, b) | Operation::Subtract(a, b) | Operation::Multiply(a, b) => {
            use_value(a)?;
            use_value(b)?;
            a.ty == Type::Field && b.ty == Type::Field && *ty == Type::Field
        }
        Operation::UnsignedAdd(a, b) => {
            use_value(a)?;
            use_value(b)?;
            unsigned_add_type(&a.ty, &b.ty, ty)
        }
        Operation::UnsignedSubtract(a, b) | Operation::UnsignedMultiply(a, b) => {
            use_value(a)?;
            use_value(b)?;
            bound(&a.ty).is_some() && bound(&b.ty).is_some() && bound(ty).is_some()
        }
        Operation::Compare { left, right, .. } => {
            use_value(left)?;
            use_value(right)?;
            bound(&left.ty).is_some() && bound(&right.ty).is_some() && *ty == Type::Boolean
        }
        Operation::Equal(a, b) | Operation::NotEqual(a, b) => {
            use_value(a)?;
            use_value(b)?;
            equality_type(&a.ty) && a.ty == b.ty && *ty == Type::Boolean
        }
        Operation::JubjubPointX(value) | Operation::JubjubPointY(value) => {
            use_value(value)?;
            value.ty == Type::JubjubPoint && *ty == Type::Field
        }
        Operation::JubjubScalarFromNative(value) => {
            use_value(value)?;
            value.ty == Type::Field && *ty == Type::Field
        }
        Operation::EcMulGenerator(scalar) => {
            use_value(scalar)?;
            scalar.ty == Type::Field && *ty == Type::JubjubPoint
        }
        Operation::EcNeg(value) => {
            use_value(value)?;
            value.ty == Type::JubjubPoint && *ty == Type::JubjubPoint
        }
        Operation::ConstructJubjubPoint { x, y } => {
            use_value(x)?;
            use_value(y)?;
            x.ty == Type::Field && y.ty == Type::Field && *ty == Type::JubjubPoint
        }
        Operation::EcMul { point, scalar } => {
            use_value(point)?;
            use_value(scalar)?;
            point.ty == Type::JubjubPoint && scalar.ty == Type::Field && *ty == Type::JubjubPoint
        }
        Operation::EcAdd { left, right } => {
            use_value(left)?;
            use_value(right)?;
            left.ty == Type::JubjubPoint
                && right.ty == Type::JubjubPoint
                && *ty == Type::JubjubPoint
        }
        Operation::Convert { value, kind } => {
            use_value(value)?;
            match kind {
                Conversion::Coerce => coercible(&value.ty, ty),
                Conversion::UnsignedCast => {
                    unsigned_bound(&value.ty).is_some() && unsigned_bound(ty).is_some()
                }
            }
        }
        Operation::Merkle { slot, operation } => {
            merkle::validate(*slot, operation, ty, declarations, scope, path)?
        }
        Operation::List { slot, operation } => {
            lists::validate(*slot, operation, ty, declarations, scope, path)?
        }
        Operation::CellRead(slot) => cell(declarations, *slot, path)? == ty && *ty != Type::Unit,
        Operation::CellWrite { slot, value } => {
            use_value(value)?;
            let declared = cell(declarations, *slot, path)?;
            *declared == value.ty && *declared != Type::Unit && *ty == Type::Unit
        }
        Operation::CellWriteCoin {
            slot,
            coin,
            recipient,
        } => {
            use_value(coin)?;
            use_value(recipient)?;
            *cell(declarations, *slot, path)? == crate::coin_shapes::qualified_coin_type()
                && coin.ty == crate::coin_shapes::shielded_coin_type()
                && recipient.ty == crate::coin_shapes::shielded_recipient_type()
                && *ty == Type::Unit
        }
        Operation::SetInsertCoin {
            slot,
            coin,
            recipient,
        } => {
            use_value(coin)?;
            use_value(recipient)?;
            *sets::element(declarations, *slot, path)? == crate::coin_shapes::qualified_coin_type()
                && coin.ty == crate::coin_shapes::shielded_coin_type()
                && recipient.ty == crate::coin_shapes::shielded_recipient_type()
                && *ty == Type::Unit
        }
        Operation::MapMember { .. }
        | Operation::MapLookup { .. }
        | Operation::MapInsert { .. }
        | Operation::MapRemove { .. }
        | Operation::MapInsertDefault { .. }
        | Operation::MapReset { .. }
        | Operation::MapSize { .. }
        | Operation::MapIsEmpty { .. } => {
            maps::validate(&binding.operation, ty, declarations, scope, path)?
        }
        Operation::SetMember { slot, value }
        | Operation::SetInsert { slot, value }
        | Operation::SetRemove { slot, value } => {
            use_value(value)?;
            let declared = sets::element(declarations, *slot, path)?;
            *declared == value.ty
                && *ty
                    == if matches!(binding.operation, Operation::SetMember { .. }) {
                        Type::Boolean
                    } else {
                        Type::Unit
                    }
        }
        Operation::SetReset { slot }
        | Operation::SetSize { slot }
        | Operation::SetIsEmpty { slot } => {
            sets::element(declarations, *slot, path)?;
            *ty == match binding.operation {
                Operation::SetSize { .. } => uint("18446744073709551615"),
                Operation::SetIsEmpty { .. } => Type::Boolean,
                _ => Type::Unit,
            }
        }
        Operation::CounterRead { slot } => {
            counter(declarations, *slot, path)?;
            *ty == uint("18446744073709551615")
        }
        Operation::CounterIncrement { slot, amount }
        | Operation::CounterDecrement { slot, amount } => {
            counter(declarations, *slot, path)?;
            use_value(amount)?;
            amount.ty == uint("65535") && *ty == Type::Unit
        }
        Operation::CounterReset { slot } => {
            counter(declarations, *slot, path)?;
            *ty == Type::Unit
        }
        Operation::CounterLessThan { slot, threshold } => {
            counter(declarations, *slot, path)?;
            use_value(threshold)?;
            threshold.ty == uint("18446744073709551615") && *ty == Type::Boolean
        }
        Operation::Witness {
            declaration,
            arguments,
        } => {
            let witness = declarations
                .witness(*declaration)
                .ok_or_else(|| invalid(path, "unknown witness declaration"))?;
            declarations.types().validate_at(&witness.result, path)?;
            for parameter in &witness.parameters {
                declarations.types().validate_at(&parameter.ty, path)?;
            }
            for value in arguments {
                use_value(value)?;
            }
            witness_result_type(&witness.result)
                && witness.result == *ty
                && witness.parameters.len() == arguments.len()
                && witness
                    .parameters
                    .iter()
                    .zip(arguments)
                    .all(|(p, a)| witness_parameter_type(&p.ty) && p.ty == a.ty)
        }
        Operation::PureCall { .. } => {
            pure::validate(binding, declarations, scope, state)?;
            true
        }
        Operation::Assert { condition, .. } => {
            use_value(condition)?;
            condition.ty == Type::Boolean && *ty == Type::Unit
        }
        Operation::VectorMap { .. } | Operation::Call { .. } | Operation::If { .. } => {
            unreachable!("recursive operations are validated by block traversal")
        }
    })
}
