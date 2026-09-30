//! Native Compact runtime surface for the typed Rust backend.
//!
//! Generated code uses this facade rather than naming the underlying
//! ledger or proof crates. Each facade operation must preserve Compact's
//! semantics and have a direct test against the corresponding ledger API.

#![forbid(unsafe_code)]

/// Compact `Field` is the scalar field used by the ledger-8 circuit runtime.
pub use midnight_transient_crypto::curve::Fr;
pub use midnight_transient_crypto::curve::Fr as Field;

/// Required by the ledger derives in their generated implementation.
pub use midnight_base_crypto::repr::MemWrite;
/// Reuse ledger-8's field representation traits and derives for user structs.
pub use midnight_transient_crypto::repr::{FieldRepr, FromFieldRepr};

/// Errors raised while evaluating a Compact circuit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompactError {
    AssertionFailed(String),
}

impl std::fmt::Display for CompactError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AssertionFailed(message) => write!(f, "Compact assertion failed: {message}"),
        }
    }
}

impl std::error::Error for CompactError {}
