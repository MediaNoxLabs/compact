//! Compact native operations backed by the ledger-8 cryptography crates.

use crate::{BinaryHashRepr, Field, FieldRepr, FixedBytes, JubjubPoint};
use midnight_base_crypto::fab::{Aligned, AlignedValue, Value};
use midnight_base_crypto::hash::{self as persistent, HashOutput, PersistentHashWriter};
use midnight_transient_crypto::fab::ValueReprAlignedValue;
use midnight_transient_crypto::hash;

fn value_repr<T: Aligned + Into<Value>>(value: T) -> ValueReprAlignedValue {
    ValueReprAlignedValue(
        AlignedValue::new(value.into(), T::alignment())
            .expect("a typed Compact value must fit its ledger alignment"),
    )
}

/// Hash a Compact value using the same value-only FAB field encoding as the
/// ledger's onchain-runtime-wasm `transientHash` entry point.
pub fn transient_hash<T: Aligned + Into<Value>>(value: T) -> Field {
    hash::transient_hash(&value_repr(value).field_vec())
}

/// Commit to a Compact value using the ledger's transient commitment primitive.
pub fn transient_commit<T: Aligned + Into<Value>>(value: T, opening: Field) -> Field {
    hash::transient_commit(&value_repr(value), opening)
}

/// Hash the ledger FAB binary value with ledger-8's persistent SHA-256 writer.
pub fn persistent_hash<T: Aligned + Into<Value>>(value: T) -> FixedBytes<32> {
    let mut writer = PersistentHashWriter::default();
    value_repr(value).binary_repr(&mut writer);
    FixedBytes(writer.finalize().0)
}

/// Commit to the FAB binary value using ledger-8's persistent commitment.
pub fn persistent_commit<T: Aligned + Into<Value>>(
    value: T,
    opening: FixedBytes<32>,
) -> FixedBytes<32> {
    FixedBytes(persistent::persistent_commit(&value_repr(value), HashOutput(opening.0)).0)
}

/// Convert a persistent digest into the transient field representation.
pub fn degrade_to_transient(value: FixedBytes<32>) -> Field {
    hash::degrade_to_transient(HashOutput(value.0))
}

/// Convert a transient field output into a 32-byte persistent digest shape.
pub fn upgrade_from_transient(value: Field) -> FixedBytes<32> {
    FixedBytes(hash::upgrade_from_transient(value).0)
}

/// Map Compact FAB fields to the ledger's embedded curve.
pub fn hash_to_curve<T: Aligned + Into<Value>>(value: T) -> JubjubPoint {
    hash::hash_to_curve(&value_repr(value))
}

/// Read the affine X coordinate; the identity has Compact's zero coordinate.
pub fn jubjub_point_x(point: JubjubPoint) -> Field {
    point.x().unwrap_or_else(|| Field::from(0_u64))
}

/// Read the affine Y coordinate; the identity has Compact's zero coordinate.
pub fn jubjub_point_y(point: JubjubPoint) -> Field {
    point.y().unwrap_or_else(|| Field::from(0_u64))
}
