// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Descriptor selection only; actual semantics belong to the real owners.
use crate::{
    ir::Expr,
    pure_semantics::{
        aggregate::AggregateSource, arithmetic::ArithmeticSource, conversion::ConversionSource,
        crypto::CryptoSource, numeric::ArithmeticKind,
    },
};

pub(super) fn aggregate(expr: &Expr) -> Option<AggregateSource<'_>> {
    Some(match expr {
        Expr::StructField {
            value,
            field,
            index,
        } => AggregateSource::StructField {
            value,
            field,
            index: *index,
        },
        Expr::TupleIndex { value, index } => AggregateSource::TupleIndex {
            value,
            index: *index,
        },
        Expr::StructLiteral { ty, fields } => AggregateSource::StructLiteral { ty, fields },
        Expr::Tuple { elements } => AggregateSource::Tuple { elements },
        Expr::Vector { element, elements } => AggregateSource::Vector { element, elements },
        Expr::VectorMap {
            parameter,
            source,
            body,
            result,
            length,
        } => AggregateSource::VectorMap {
            parameter,
            source,
            body,
            result,
            length: *length,
        },
        Expr::VectorFoldCall {
            name,
            initial,
            source,
            accumulator,
            element,
            length,
        } => AggregateSource::VectorFoldCall {
            name,
            initial,
            source,
            accumulator,
            element,
            length: *length,
        },
        Expr::Call { name, arguments } => AggregateSource::Call { name, arguments },
        _ => return None,
    })
}
pub(super) fn arithmetic(expr: &Expr) -> Option<ArithmeticSource<'_>> {
    Some(match expr {
        Expr::UnsignedAdd { max, left, right } => ArithmeticSource::Unsigned {
            kind: ArithmeticKind::Add,
            max,
            left,
            right,
        },
        Expr::UnsignedSubtract { max, left, right } => ArithmeticSource::Unsigned {
            kind: ArithmeticKind::Subtract,
            max,
            left,
            right,
        },
        Expr::UnsignedMultiply { max, left, right } => ArithmeticSource::Unsigned {
            kind: ArithmeticKind::Multiply,
            max,
            left,
            right,
        },
        Expr::Add { left, right } => ArithmeticSource::Field {
            kind: ArithmeticKind::Add,
            left,
            right,
        },
        Expr::Subtract { left, right } => ArithmeticSource::Field {
            kind: ArithmeticKind::Subtract,
            left,
            right,
        },
        Expr::Multiply { left, right } => ArithmeticSource::Field {
            kind: ArithmeticKind::Multiply,
            left,
            right,
        },
        Expr::Equal { left, right } => ArithmeticSource::Equality {
            negated: false,
            left,
            right,
        },
        Expr::NotEqual { left, right } => ArithmeticSource::Equality {
            negated: true,
            left,
            right,
        },
        Expr::Compare {
            operator,
            left,
            right,
        } => ArithmeticSource::Ordered {
            operator: *operator,
            left,
            right,
        },
        _ => return None,
    })
}
pub(super) fn conversion(expr: &Expr) -> Option<ConversionSource<'_>> {
    Some(match expr {
        Expr::FieldCast { value } => ConversionSource::FieldCast { value },
        Expr::FieldToBytes32 { value } => ConversionSource::FieldToBytes32 { value },
        Expr::UnsignedCast { value, max } => ConversionSource::UnsignedCast { value, max },
        Expr::Coerce { value, ty } => ConversionSource::Coerce { value, ty },
        _ => return None,
    })
}
pub(super) fn crypto(expr: &Expr) -> Option<CryptoSource<'_>> {
    Some(match expr {
        Expr::TransientHash { value } => CryptoSource::TransientHash(value),
        Expr::TransientCommit { value, opening } => CryptoSource::TransientCommit(value, opening),
        Expr::PersistentHash { value } => CryptoSource::PersistentHash(value),
        Expr::PersistentCommit { value, opening } => CryptoSource::PersistentCommit(value, opening),
        Expr::Keccak256 { value } => CryptoSource::Keccak256(value),
        Expr::DegradeToTransient { value } => CryptoSource::DegradeToTransient(value),
        Expr::UpgradeFromTransient { value } => CryptoSource::UpgradeFromTransient(value),
        Expr::HashToCurve { value } => CryptoSource::HashToCurve(value),
        Expr::JubjubPointX { value } => CryptoSource::JubjubPointX(value),
        Expr::JubjubPointY { value } => CryptoSource::JubjubPointY(value),
        Expr::ConstructJubjubPoint { x, y } => CryptoSource::ConstructJubjubPoint(x, y),
        Expr::EcAdd { left, right } => CryptoSource::EcAdd(left, right),
        Expr::EcNeg { value } => CryptoSource::EcNeg(value),
        Expr::EcMul { point, scalar } => CryptoSource::EcMul(point, scalar),
        Expr::EcMulGenerator { scalar } => CryptoSource::EcMulGenerator(scalar),
        Expr::JubjubScalarFromNative { value } => CryptoSource::JubjubScalarFromNative(value),
        _ => return None,
    })
}
