//! Compact's variable length opaque string value.

use crate::ledger::CellValue;
use crate::{BinaryHashRepr, CompactError, FieldRepr, Fr, FromFieldRepr, MemWrite};
use midnight_base_crypto::fab::{Aligned, Alignment, Value, ValueSlice};

/// UTF-8 text encoded as one ledger Compress atom.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct OpaqueString(pub String);

impl From<&str> for OpaqueString {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

impl From<String> for OpaqueString {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl Aligned for OpaqueString {
    fn alignment() -> Alignment {
        <Vec<u8> as Aligned>::alignment()
    }
}

impl From<OpaqueString> for Value {
    fn from(value: OpaqueString) -> Self {
        Value::from(value.0.into_bytes())
    }
}

impl CellValue for OpaqueString {
    fn decode_cell_value(value: &ValueSlice) -> Result<Self, CompactError> {
        if value.0.len() != 1 {
            return Err(CompactError::InvalidLedgerCell(
                "opaque string requires one Compress atom".into(),
            ));
        }
        String::from_utf8(value.0[0].0.clone())
            .map(Self)
            .map_err(|error| CompactError::InvalidLedgerCell(error.to_string()))
    }
}

impl FieldRepr for OpaqueString {
    fn field_repr<W: MemWrite<Fr>>(&self, writer: &mut W) {
        self.0.as_bytes().field_repr(writer);
    }

    fn field_size(&self) -> usize {
        self.0.len().div_ceil(31)
    }
}

impl FromFieldRepr for OpaqueString {
    // No fixed field width can encode an arbitrary length. Empty values are
    // representable; ledger reads use CellValue and retain the exact atom.
    const FIELD_SIZE: usize = 0;

    fn from_field_repr(repr: &[Fr]) -> Option<Self> {
        repr.is_empty().then(Self::default)
    }
}

impl BinaryHashRepr for OpaqueString {
    fn binary_repr<W: MemWrite<u8>>(&self, writer: &mut W) {
        self.0.as_bytes().binary_repr(writer);
    }

    fn binary_len(&self) -> usize {
        self.0.len()
    }
}
