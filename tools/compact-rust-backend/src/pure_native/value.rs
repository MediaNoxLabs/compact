// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Output-only projection of checked fragment operands into existing syntax facts.
//! This mapping never traverses source, checks types or reselects support policy.
use super::fragment::NativeFragment as V;
use crate::pure_semantics::{
    aggregate::CheckedAggregate as A, crypto::CheckedCrypto as C, jubjub::CheckedJubjub as J,
};

pub(crate) fn crypto(checked: C<V>) -> C<syn::Expr> {
    match checked {
        C::TransientHash { value, encoding } => C::TransientHash {
            value: value.into_expression(),
            encoding,
        },
        C::TransientCommit {
            value,
            opening,
            encoding,
        } => C::TransientCommit {
            value: value.into_expression(),
            opening: opening.into_expression(),
            encoding,
        },
        C::PersistentHash(value) => C::PersistentHash(value.into_expression()),
        C::PersistentCommit { value, opening } => C::PersistentCommit {
            value: value.into_expression(),
            opening: opening.into_expression(),
        },
        C::Keccak256(value) => C::Keccak256(value.into_expression()),
        C::DegradeToTransient(value) => C::DegradeToTransient(value.into_expression()),
        C::UpgradeFromTransient(value) => C::UpgradeFromTransient(value.into_expression()),
        C::HashToCurve { value, encoding } => C::HashToCurve {
            value: value.into_expression(),
            encoding,
        },
        C::Jubjub(checked) => C::Jubjub(jubjub(checked)),
    }
}
fn jubjub(checked: J<V>) -> J<syn::Expr> {
    match checked {
        J::Reduce(value) => J::Reduce(value.into_expression()),
        J::Generator(value) => J::Generator(value.into_expression()),
        J::Multiply(left, right) => J::Multiply(left.into_expression(), right.into_expression()),
        J::Add(left, right) => J::Add(left.into_expression(), right.into_expression()),
        J::Coordinate(axis, value) => J::Coordinate(axis, value.into_expression()),
        J::Construct(left, right) => J::Construct(left.into_expression(), right.into_expression()),
        J::Negate(value) => J::Negate(value.into_expression()),
    }
}
pub(crate) fn aggregate(
    checked: A<V, syn::Ident, syn::Type>,
) -> A<syn::Expr, syn::Ident, syn::Type> {
    match checked {
        A::StructField {
            value,
            field,
            ownership,
        } => A::StructField {
            value: value.into_expression(),
            field,
            ownership,
        },
        A::TupleIndex { value, index } => A::TupleIndex {
            value: value.into_expression(),
            index,
        },
        A::StructLiteral { name, fields } => A::StructLiteral {
            name,
            fields: fields
                .into_iter()
                .map(|(name, value)| (name, value.into_expression()))
                .collect(),
        },
        A::Tuple(values) => A::Tuple(expressions(values)),
        A::Vector(values) => A::Vector(expressions(values)),
        A::EmptyVector(carrier) => A::EmptyVector(carrier),
        A::Map {
            source,
            source_plan,
            binding,
            body,
            length,
        } => A::Map {
            source: source.into_expression(),
            source_plan,
            binding,
            body: body.into_expression(),
            length,
        },
        A::Fold {
            function,
            initial,
            initial_plan,
            source,
            source_plan,
        } => A::Fold {
            function,
            initial: initial.into_expression(),
            initial_plan,
            source: source.into_expression(),
            source_plan,
        },
        A::Call {
            function,
            arguments,
        } => A::Call {
            function,
            arguments: expressions(arguments),
        },
    }
}
fn expressions(values: Vec<V>) -> Vec<syn::Expr> {
    values.into_iter().map(V::into_expression).collect()
}
