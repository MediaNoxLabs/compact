//! Compact native operations backed by the ledger-8 cryptography crates.

use crate::{Field, FieldRepr};
use midnight_base_crypto::fab::{Aligned, AlignedValue, Value};
use midnight_transient_crypto::fab::ValueReprAlignedValue;
use midnight_transient_crypto::hash;

/// Hash a Compact value using the same value-only FAB field encoding as the
/// ledger's onchain-runtime-wasm `transientHash` entry point.
pub fn transient_hash<T: Aligned + Into<Value>>(value: T) -> Field {
    let aligned = AlignedValue::new(value.into(), T::alignment())
        .expect("a typed Compact value must fit its ledger alignment");
    hash::transient_hash(&ValueReprAlignedValue(aligned).field_vec())
}

/// Commit to a Compact value using the ledger's transient commitment primitive.
pub fn transient_commit<T: Aligned + Into<Value>>(value: T, opening: Field) -> Field {
    let aligned = AlignedValue::new(value.into(), T::alignment())
        .expect("a typed Compact value must fit its ledger alignment");
    hash::transient_commit(&ValueReprAlignedValue(aligned), opening)
}
