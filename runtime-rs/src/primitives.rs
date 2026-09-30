use crate::{CompactError, FieldRepr, Fr, FromFieldRepr, MemWrite};
use midnight_base_crypto::fab::{Aligned, Alignment, AlignmentAtom, Value, ValueSlice};

/// A Compact `Uint<MAX>` value whose declared maximum is checked at creation
/// and decoding. The inner `u128` uses ledger-8's field representation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BoundedUint<const MAX: u128>(u128);

impl<const MAX: u128> BoundedUint<MAX> {
    /// Matches Compact's `ceil(bit_length(MAX) / 8)` descriptor width.
    pub const BYTE_LENGTH: u32 = (128 - MAX.leading_zeros()).div_ceil(8);

    pub fn new(value: u128) -> Result<Self, CompactError> {
        if value > MAX {
            Err(CompactError::UnsignedOutOfRange { value, max: MAX })
        } else {
            Ok(Self(value))
        }
    }

    pub const fn value(self) -> u128 {
        self.0
    }
}

impl<const MAX: u128> Aligned for BoundedUint<MAX> {
    fn alignment() -> Alignment {
        Alignment::singleton(AlignmentAtom::Bytes {
            length: Self::BYTE_LENGTH,
        })
    }
}

impl<const MAX: u128> From<BoundedUint<MAX>> for Value {
    fn from(value: BoundedUint<MAX>) -> Self {
        Value::from(value.0)
    }
}

impl<const MAX: u128> TryFrom<&ValueSlice> for BoundedUint<MAX> {
    type Error = CompactError;

    fn try_from(value: &ValueSlice) -> Result<Self, Self::Error> {
        let integer = u128::try_from(value).map_err(|_| CompactError::InvalidUnsignedValue)?;
        Self::new(integer)
    }
}

impl<const MAX: u128> TryFrom<u128> for BoundedUint<MAX> {
    type Error = CompactError;

    fn try_from(value: u128) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl<const MAX: u128> FieldRepr for BoundedUint<MAX> {
    fn field_repr<W: MemWrite<Fr>>(&self, writer: &mut W) {
        self.0.field_repr(writer);
    }

    fn field_size(&self) -> usize {
        self.0.field_size()
    }
}

impl<const MAX: u128> FromFieldRepr for BoundedUint<MAX> {
    const FIELD_SIZE: usize = <u128 as FromFieldRepr>::FIELD_SIZE;

    fn from_field_repr(repr: &[Fr]) -> Option<Self> {
        let value = <u128 as FromFieldRepr>::from_field_repr(repr)?;
        Self::new(value).ok()
    }
}
