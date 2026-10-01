//! Compact native operations backed by the ledger-8 cryptography crates.

use crate::CompactError;
use crate::primitives::checked_jubjub_coordinates;
use crate::{BinaryHashRepr, Field, FieldRepr, FixedBytes, JubjubPoint};
use midnight_base_crypto::fab::{Aligned, AlignedValue, Value};
use midnight_base_crypto::hash::{self as persistent, HashOutput, PersistentHashWriter};
use midnight_transient_crypto::curve::{EmbeddedFr, EmbeddedGroupAffine, embedded};
use midnight_transient_crypto::fab::ValueReprAlignedValue;
use midnight_transient_crypto::hash;
use sha3::{Digest, Keccak256};

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

/// Hash the concatenated FAB atoms using Compact's Keccak-256 encoding.
pub fn keccak256<T: Into<Value>>(value: T) -> FixedBytes<32> {
    let mut hasher = Keccak256::new();
    for atom in value.into().0 {
        hasher.update(atom.0);
    }
    FixedBytes(hasher.finalize().into())
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
    hash::hash_to_curve(&value_repr(value)).into()
}

/// Construct a curve point after checking the affine coordinates.
pub fn construct_jubjub_point(x: Field, y: Field) -> Result<JubjubPoint, CompactError> {
    checked_jubjub_coordinates(x, y)
        .map(JubjubPoint::from)
        .ok_or(CompactError::InvalidJubjubPoint)
}

/// Read the affine X coordinate; the identity has Compact's zero coordinate.
pub fn jubjub_point_x(point: JubjubPoint) -> Field {
    point.x().unwrap_or_else(|| Field::from(0_u64))
}

/// Read the affine Y coordinate; Compact represents the identity as (0, 1).
pub fn jubjub_point_y(point: JubjubPoint) -> Field {
    point.y().unwrap_or_else(|| Field::from(1_u64))
}

/// Add two ledger embedded-curve points.
pub fn ec_add(left: JubjubPoint, right: JubjubPoint) -> JubjubPoint {
    (left.inner + right.inner).into()
}

/// Negate a ledger embedded-curve point.
pub fn ec_neg(point: JubjubPoint) -> JubjubPoint {
    (-point.inner).into()
}

fn canonical_jubjub_scalar(value: Field) -> Result<EmbeddedFr, CompactError> {
    EmbeddedFr::from_le_bytes(&value.as_le_bytes()).ok_or(CompactError::InvalidJubjubScalar)
}

/// Multiply by a canonical Jubjub scalar, matching the ledger WASM check.
pub fn ec_mul(point: JubjubPoint, scalar: Field) -> Result<JubjubPoint, CompactError> {
    Ok((point.inner * canonical_jubjub_scalar(scalar)?).into())
}

/// Multiply the embedded group generator by a canonical Jubjub scalar.
pub fn ec_mul_generator(scalar: Field) -> Result<JubjubPoint, CompactError> {
    Ok((EmbeddedGroupAffine::generator() * canonical_jubjub_scalar(scalar)?).into())
}

/// Reduce a native field value into the embedded scalar field.
pub fn jubjub_scalar_from_native(value: Field) -> Field {
    let mut wide = [0u8; 64];
    wide[..32].copy_from_slice(&value.as_le_bytes());
    let scalar = EmbeddedFr(embedded::Scalar::from_bytes_wide(&wide));
    Field::from_le_bytes(&scalar.as_le_bytes()).expect("embedded scalar fits the native field")
}
