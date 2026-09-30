//! Native Compact runtime surface for the typed Rust backend.
//!
//! Generated code uses this facade rather than naming the underlying
//! ledger or proof crates. Each facade operation must preserve Compact's
//! semantics and have a direct test against the corresponding ledger API.

#![forbid(unsafe_code)]

pub mod context;
pub mod ledger;
mod primitives;

pub use midnight_compact_runtime_macros::CompactCellValue;
pub use primitives::{
    BoundedUint, FixedBytes, FixedVector, add_unsigned, cast_unsigned, multiply_unsigned,
    subtract_unsigned,
};

/// Ledger FAB types used by the generated user-type derive.
pub mod fab {
    pub use midnight_base_crypto::fab::{Aligned, Alignment, AlignmentAtom, Value, ValueSlice};
}

/// Increment when generated Rust and the runtime's public contract change.
pub const RUST_RUNTIME_ABI: u32 = 1;
/// The ledger line selected by this Compact branch's `flake.nix`.
pub const LEDGER_VERSION: &str = "ledger-8.0.2";

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
    UnsignedOutOfRange { value: u128, max: u128 },
    UnsignedOverflow,
    UnsignedUnderflow,
}

impl std::fmt::Display for CompactError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AssertionFailed(message) => write!(f, "Compact assertion failed: {message}"),
            Self::InvalidLedgerCell(message) => write!(f, "invalid Compact ledger cell: {message}"),
            Self::LedgerQueryRejected(message) => write!(f, "ledger query rejected: {message}"),
            Self::InvalidUnsignedValue => write!(f, "invalid Compact unsigned value"),
            Self::UnsignedOutOfRange { value, max } => {
                write!(f, "unsigned value {value} exceeds Compact maximum {max}")
            }
            Self::UnsignedOverflow => write!(f, "Compact unsigned arithmetic overflow"),
            Self::UnsignedUnderflow => write!(f, "Compact unsigned arithmetic underflow"),
        }
    }
}

impl std::error::Error for CompactError {}
