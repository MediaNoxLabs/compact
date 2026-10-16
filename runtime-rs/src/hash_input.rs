// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Exact source-value encoding for hashing, separate from ledger normal form.
use crate::{
    BoundedUint, Field, FixedBytes, FixedVector, JubjubPoint, OpaqueBytes, OpaqueString, WideUint,
};
use midnight_base_crypto::fab::{Aligned, Alignment, Value, ValueAtom};

/// Encode a Compact hash input without discarding opaque trailing zero bytes.
///
/// Implementations preserve the declared alignment and field order. Fixed-size
/// leaves retain ordinary ledger encoding; opaque leaves retain every byte.
/// Generated named types use [`crate::CompactHashInput`]. This is deliberately
/// separate from ledger storage conversion and validation.
pub trait HashInputValue: Aligned + Sized {
    fn into_hash_value(self) -> Value;
}

/// A value adapted for the existing official aligned hashing interfaces.
///
/// This wrapper does not implement ledger cell storage or decoding. Serializing
/// noncanonical atoms is a separate boundary; constructing a hash input does
/// not establish that it can be transported as a public or private proof value.
#[derive(Clone, Debug)]
pub struct ExactHashInput<T>(T);

impl<T> ExactHashInput<T> {
    pub(crate) fn into_inner(self) -> T {
        self.0
    }
}

/// Preserve exact opaque bytes when calling a generic Compact hash primitive.
///
/// Generated code uses this for opaque-bearing curve/transient hash inputs.
/// Ordinary `Into<Value>` keeps its existing canonical ledger behavior.
pub fn exact_hash_input<T: HashInputValue>(value: T) -> ExactHashInput<T> {
    ExactHashInput(value)
}

/// Preserve the source witness value in memory before proof-transport checks.
/// Noncanonical private atoms are rejected by recorded-call preparation.
pub fn exact_private_output<T: HashInputValue>(value: T) -> crate::fab::AlignedValue {
    exact_hash_input(value).into()
}

impl<T: HashInputValue> Aligned for ExactHashInput<T> {
    fn alignment() -> Alignment {
        T::alignment()
    }
}
impl<T: HashInputValue> From<ExactHashInput<T>> for Value {
    fn from(value: ExactHashInput<T>) -> Self {
        value.0.into_hash_value()
    }
}

macro_rules! fixed_leaf {
    ($($ty:ty),+ $(,)?) => {$(
        impl HashInputValue for $ty {
            fn into_hash_value(self) -> Value { self.into() }
        }
    )+};
}
fixed_leaf!((), bool, u8, u16, u32, u64, u128, Field, JubjubPoint);
impl<const N: usize> HashInputValue for FixedBytes<N> {
    fn into_hash_value(self) -> Value {
        self.into()
    }
}
impl<const MAX: u128> HashInputValue for BoundedUint<MAX> {
    fn into_hash_value(self) -> Value {
        self.into()
    }
}
impl<const HIGH: u128, const LOW: u128> HashInputValue for WideUint<HIGH, LOW> {
    fn into_hash_value(self) -> Value {
        self.into()
    }
}
impl HashInputValue for OpaqueString {
    fn into_hash_value(self) -> Value {
        Value(vec![ValueAtom(self.0.into_bytes())])
    }
}
impl HashInputValue for OpaqueBytes {
    fn into_hash_value(self) -> Value {
        Value(vec![ValueAtom(self.0)])
    }
}
impl<T: HashInputValue, const N: usize> HashInputValue for FixedVector<T, N> {
    fn into_hash_value(self) -> Value {
        Value(
            self.0
                .into_iter()
                .flat_map(|v| v.into_hash_value().0)
                .collect(),
        )
    }
}

macro_rules! tuple {
    () => {};
    ($first:ident $(, $rest:ident)*) => {
        #[allow(non_snake_case)]
        impl<$first: HashInputValue $(, $rest: HashInputValue)*> HashInputValue for ($first, $($rest,)*) {
            fn into_hash_value(self) -> Value {
                let ($first, $($rest,)*) = self;
                Value(
                    [$first.into_hash_value(), $($rest.into_hash_value(),)*]
                        .into_iter()
                        .flat_map(|value| value.0)
                        .collect()
                )
            }
        }
        tuple!($($rest),*);
    };
}
// Match the pinned official FAB tuple alignment/conversion range.
tuple!(A, B, C, D, E, F, G, H, I, J, K);
