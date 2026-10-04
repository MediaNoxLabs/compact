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

//! Native Compact runtime surface for the typed Rust backend.
//!
//! Generated code uses this facade rather than naming the underlying
//! ledger or proof crates. Each facade operation must preserve Compact's
//! semantics and have a direct test against the corresponding ledger API.

#![forbid(unsafe_code)]

pub mod context;
pub mod ledger;
mod natives;
mod opaque;
mod primitives;
pub mod public_state;
pub mod recording;
pub mod slots;
#[cfg(feature = "ledger-transaction")]
pub mod transaction;

pub use midnight_compact_runtime_macros::{
    CompactCellValue, CompactEnum, CompactMerklePath, CompactMerklePathEntry,
    CompactMerkleTreeDigest, compact_witness_bridge,
};
pub use natives::{
    construct_jubjub_point, degrade_to_transient, ec_add, ec_mul, ec_mul_generator, ec_neg,
    hash_to_curve, jubjub_point_x, jubjub_point_y, jubjub_scalar_from_native, keccak256,
    persistent_commit, persistent_hash, transient_commit, transient_hash, upgrade_from_transient,
};
pub use opaque::{OpaqueBytes, OpaqueString};
pub use primitives::{
    BoundedUint, FixedBytes, FixedVector, JubjubPoint, WideUint, add_unsigned, cast_unsigned,
    multiply_unsigned, narrow_wide_uint, subtract_unsigned,
};

/// Ledger FAB types used by the generated user-type derive.
pub mod fab {
    pub use midnight_base_crypto::fab::{
        Aligned, AlignedValue, Alignment, AlignmentAtom, Value, ValueSlice,
    };
}

/// Increment when generated Rust and the runtime's public contract change.
pub const RUST_RUNTIME_ABI: u32 = 33;
/// The ledger line selected by this Compact branch's `flake.nix`.
pub const LEDGER_VERSION: &str = "ledger-8.0.3";

/// Compact `Field` is the scalar field used by the ledger-8 circuit runtime.
pub use midnight_transient_crypto::curve::Fr;
pub use midnight_transient_crypto::curve::Fr as Field;

pub use midnight_base_crypto::repr::BinaryHashRepr;
/// Required by the ledger derives in their generated implementation.
pub use midnight_base_crypto::repr::MemWrite;
/// Reuse ledger-8's field representation traits and derives for user structs.
pub use midnight_transient_crypto::repr::{FieldRepr, FromFieldRepr};

/// Errors raised while evaluating a Compact circuit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompactError {
    AssertionFailed(String),
    InvalidLedgerCell(String),
    LedgerQueryRejected(String),
    InvalidUnsignedValue,
    InvalidJubjubScalar,
    InvalidJubjubPoint,
    UnsignedOutOfRange { value: u128, max: u128 },
    UnsignedOverflow,
    UnsignedUnderflow,
}

impl std::fmt::Display for CompactError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AssertionFailed(message) => write!(f, "failed assert: {message}"),
            Self::InvalidLedgerCell(message) => write!(f, "invalid Compact ledger cell: {message}"),
            Self::LedgerQueryRejected(message) => write!(f, "ledger query rejected: {message}"),
            Self::InvalidUnsignedValue => write!(f, "invalid Compact unsigned value"),
            Self::InvalidJubjubScalar => write!(f, "invalid Jubjub scalar"),
            Self::InvalidJubjubPoint => write!(f, "invalid Jubjub point"),
            Self::UnsignedOutOfRange { value, max } => {
                write!(f, "unsigned value {value} exceeds Compact maximum {max}")
            }
            Self::UnsignedOverflow => write!(f, "Compact unsigned arithmetic overflow"),
            Self::UnsignedUnderflow => write!(f, "Compact unsigned arithmetic underflow"),
        }
    }
}

impl std::error::Error for CompactError {}
