// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Exhaustive source classification. Every family retains its single checked owner.
use super::super::{
    aggregate::AggregateSource, arithmetic::ArithmeticSource, control::ControlSource,
    conversion::ConversionSource, crypto::CryptoSource, leaf::LeafSource, numeric::ArithmeticKind,
};
use crate::ir::Expr;
pub(super) enum Source<'a> {
    Control(ControlSource<'a>),
    Leaf(LeafSource<'a>),
    Conversion(ConversionSource<'a>),
    Arithmetic(ArithmeticSource<'a>),
    Crypto(CryptoSource<'a>),
    Aggregate(AggregateSource<'a>),
}
pub(super) fn classify(expr: &Expr) -> Source<'_> {
    match expr {
        Expr::Assert { condition, message } => {
            Source::Control(ControlSource::Assert { condition, message })
        }
        Expr::Sequence { steps, value } => {
            Source::Control(ControlSource::Sequence { steps, value })
        }
        Expr::If {
            condition,
            then,
            otherwise,
        } => Source::Control(ControlSource::If {
            condition,
            then,
            otherwise,
        }),
        Expr::Let { bindings, body } => Source::Control(ControlSource::Let { bindings, body }),
        Expr::Unit => Source::Leaf(LeafSource::Unit),
        Expr::Default { ty } => Source::Leaf(LeafSource::Default(ty)),
        Expr::Boolean { value } => Source::Leaf(LeafSource::Boolean(*value)),
        Expr::FieldLiteral { value } => Source::Leaf(LeafSource::FieldLiteral(value)),
        Expr::BytesLiteral { bytes } => Source::Leaf(LeafSource::BytesLiteral(bytes)),
        Expr::UnsignedLiteral { value, max } => {
            Source::Leaf(LeafSource::UnsignedLiteral { value, max })
        }
        Expr::Parameter { name } => Source::Leaf(LeafSource::Parameter(name)),
        Expr::EnumVariant { ty, variant } => Source::Leaf(LeafSource::EnumVariant { ty, variant }),
        Expr::WitnessCall { .. }
        | Expr::KernelClaim { .. }
        | Expr::KernelMintShielded { .. }
        | Expr::CreateZswapInput { .. }
        | Expr::CreateZswapOutput { .. }
        | Expr::NativeWitnessCall { .. }
        | Expr::SetMember { .. }
        | Expr::MapMember { .. }
        | Expr::MapLookup { .. }
        | Expr::MerkleCheckRoot { .. }
        | Expr::HistoricMerkleCheckRoot { .. }
        | Expr::SetIsEmpty { .. }
        | Expr::SetSize { .. }
        | Expr::MapIsEmpty { .. }
        | Expr::ListLength { .. }
        | Expr::ListIsEmpty { .. }
        | Expr::ListHead { .. }
        | Expr::CellRead { .. }
        | Expr::CounterRead { .. }
        | Expr::CounterLessThan { .. }
        | Expr::KernelSelf { .. } => Source::Leaf(LeafSource::Effectful),
        Expr::FieldCast { value } => Source::Conversion(ConversionSource::FieldCast { value }),
        Expr::FieldToBytes32 { value } => {
            Source::Conversion(ConversionSource::FieldToBytes32 { value })
        }
        Expr::Coerce { value, ty } => Source::Conversion(ConversionSource::Coerce { value, ty }),
        Expr::UnsignedCast { max, value } => {
            Source::Conversion(ConversionSource::UnsignedCast { max, value })
        }
        Expr::UnsignedAdd { max, left, right } => Source::Arithmetic(ArithmeticSource::Unsigned {
            kind: ArithmeticKind::Add,
            max,
            left,
            right,
        }),
        Expr::UnsignedSubtract { max, left, right } => {
            Source::Arithmetic(ArithmeticSource::Unsigned {
                kind: ArithmeticKind::Subtract,
                max,
                left,
                right,
            })
        }
        Expr::UnsignedMultiply { max, left, right } => {
            Source::Arithmetic(ArithmeticSource::Unsigned {
                kind: ArithmeticKind::Multiply,
                max,
                left,
                right,
            })
        }
        Expr::Add { left, right } => Source::Arithmetic(ArithmeticSource::Field {
            kind: ArithmeticKind::Add,
            left,
            right,
        }),
        Expr::Subtract { left, right } => Source::Arithmetic(ArithmeticSource::Field {
            kind: ArithmeticKind::Subtract,
            left,
            right,
        }),
        Expr::Multiply { left, right } => Source::Arithmetic(ArithmeticSource::Field {
            kind: ArithmeticKind::Multiply,
            left,
            right,
        }),
        Expr::Equal { left, right } => Source::Arithmetic(ArithmeticSource::Equality {
            negated: false,
            left,
            right,
        }),
        Expr::NotEqual { left, right } => Source::Arithmetic(ArithmeticSource::Equality {
            negated: true,
            left,
            right,
        }),
        Expr::Compare {
            operator,
            left,
            right,
        } => Source::Arithmetic(ArithmeticSource::Ordered {
            operator: *operator,
            left,
            right,
        }),
        Expr::TransientHash { value } => Source::Crypto(CryptoSource::TransientHash(value)),
        Expr::TransientCommit { value, opening } => {
            Source::Crypto(CryptoSource::TransientCommit(value, opening))
        }
        Expr::PersistentHash { value } => Source::Crypto(CryptoSource::PersistentHash(value)),
        Expr::PersistentCommit { value, opening } => {
            Source::Crypto(CryptoSource::PersistentCommit(value, opening))
        }
        Expr::Keccak256 { value } => Source::Crypto(CryptoSource::Keccak256(value)),
        Expr::DegradeToTransient { value } => {
            Source::Crypto(CryptoSource::DegradeToTransient(value))
        }
        Expr::UpgradeFromTransient { value } => {
            Source::Crypto(CryptoSource::UpgradeFromTransient(value))
        }
        Expr::HashToCurve { value } => Source::Crypto(CryptoSource::HashToCurve(value)),
        Expr::JubjubPointX { value } => Source::Crypto(CryptoSource::JubjubPointX(value)),
        Expr::JubjubPointY { value } => Source::Crypto(CryptoSource::JubjubPointY(value)),
        Expr::ConstructJubjubPoint { x, y } => {
            Source::Crypto(CryptoSource::ConstructJubjubPoint(x, y))
        }
        Expr::EcAdd { left, right } => Source::Crypto(CryptoSource::EcAdd(left, right)),
        Expr::EcNeg { value } => Source::Crypto(CryptoSource::EcNeg(value)),
        Expr::EcMul { point, scalar } => Source::Crypto(CryptoSource::EcMul(point, scalar)),
        Expr::EcMulGenerator { scalar } => Source::Crypto(CryptoSource::EcMulGenerator(scalar)),
        Expr::JubjubScalarFromNative { value } => {
            Source::Crypto(CryptoSource::JubjubScalarFromNative(value))
        }
        Expr::StructField {
            value,
            field,
            index,
        } => Source::Aggregate(AggregateSource::StructField {
            value,
            field,
            index: *index,
        }),
        Expr::TupleIndex { value, index } => Source::Aggregate(AggregateSource::TupleIndex {
            value,
            index: *index,
        }),
        Expr::StructLiteral { ty, fields } => {
            Source::Aggregate(AggregateSource::StructLiteral { ty, fields })
        }
        Expr::Tuple { elements } => Source::Aggregate(AggregateSource::Tuple { elements }),
        Expr::Vector { element, elements } => {
            Source::Aggregate(AggregateSource::Vector { element, elements })
        }
        Expr::VectorMap {
            parameter,
            source,
            body,
            result,
            length,
        } => Source::Aggregate(AggregateSource::VectorMap {
            parameter,
            source,
            body,
            result,
            length: *length,
        }),
        Expr::VectorFoldCall {
            name,
            initial,
            source,
            accumulator,
            element,
            length,
        } => Source::Aggregate(AggregateSource::VectorFoldCall {
            name,
            initial,
            source,
            accumulator,
            element,
            length: *length,
        }),
        Expr::Call { name, arguments } => {
            Source::Aggregate(AggregateSource::Call { name, arguments })
        }
    }
}
