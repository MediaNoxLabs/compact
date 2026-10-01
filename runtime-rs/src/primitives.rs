use crate::{BinaryHashRepr, CompactError, FieldRepr, Fr, FromFieldRepr, MemWrite};
use midnight_base_crypto::fab::{
    Aligned, Alignment, AlignmentAtom, InvalidBuiltinDecode, Value, ValueSlice,
};
use midnight_transient_crypto::curve::{EmbeddedGroupAffine, FR_BYTES_STORED};
use midnight_transient_crypto::repr::bytes_from_field_repr;

/// Compact's Jubjub point, wrapping the ledger point so it can participate in
/// the ledger FAB and field-representation derives used by user types.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct JubjubPoint {
    pub(crate) inner: EmbeddedGroupAffine,
}

impl From<EmbeddedGroupAffine> for JubjubPoint {
    fn from(inner: EmbeddedGroupAffine) -> Self {
        Self { inner }
    }
}

impl JubjubPoint {
    pub fn x(&self) -> Option<Fr> {
        self.inner.x()
    }

    pub fn y(&self) -> Option<Fr> {
        self.inner.y()
    }

    pub fn identity() -> Self {
        Self::default()
    }

    pub fn is_identity(&self) -> bool {
        self.inner.is_identity()
    }
}

impl Aligned for JubjubPoint {
    fn alignment() -> Alignment {
        EmbeddedGroupAffine::alignment()
    }
}

impl From<JubjubPoint> for Value {
    fn from(point: JubjubPoint) -> Self {
        Value::from(point.inner)
    }
}

fn checked_jubjub_coordinates(x: Fr, y: Fr) -> Option<EmbeddedGroupAffine> {
    if x == Fr::from(0_u64) && y == Fr::from(0_u64) {
        return Some(EmbeddedGroupAffine::identity());
    }
    // The pinned ledger's `EmbeddedGroupAffine::new` currently panics when
    // compressed point decoding fails. Validate through the curve's checked
    // decoder first, then use the ledger constructor for subgroup handling.
    let mut bytes: [u8; 32] = y.as_le_bytes().try_into().ok()?;
    bytes[31] |= (x.as_le_bytes()[0] & 1) << 7;
    let affine = midnight_curves::JubjubAffine::from_bytes(bytes).into_option()?;
    if affine.get_u() != x.0 || affine.get_v() != y.0 {
        return None;
    }
    EmbeddedGroupAffine::new(x, y)
}

impl TryFrom<&ValueSlice> for JubjubPoint {
    type Error = InvalidBuiltinDecode;

    fn try_from(value: &ValueSlice) -> Result<Self, Self::Error> {
        if value.0.len() != 2 {
            return Err(InvalidBuiltinDecode("JubjubPoint"));
        }
        let x = Fr::try_from(&value.0[0])?;
        let y = Fr::try_from(&value.0[1])?;
        checked_jubjub_coordinates(x, y)
            .map(Self::from)
            .ok_or(InvalidBuiltinDecode("JubjubPoint"))
    }
}

impl FieldRepr for JubjubPoint {
    fn field_repr<W: MemWrite<Fr>>(&self, writer: &mut W) {
        writer.write(&[
            self.x().unwrap_or_else(|| Fr::from(0_u64)),
            self.y().unwrap_or_else(|| Fr::from(0_u64)),
        ]);
    }

    fn field_size(&self) -> usize {
        2
    }
}

impl FromFieldRepr for JubjubPoint {
    const FIELD_SIZE: usize = 2;

    fn from_field_repr(repr: &[Fr]) -> Option<Self> {
        if repr.len() != Self::FIELD_SIZE {
            return None;
        }
        checked_jubjub_coordinates(repr[0], repr[1]).map(Self::from)
    }
}

impl BinaryHashRepr for JubjubPoint {
    fn binary_repr<W: MemWrite<u8>>(&self, writer: &mut W) {
        writer.write(&self.x().unwrap_or_else(|| Fr::from(0_u64)).as_le_bytes());
        writer.write(&self.y().unwrap_or_else(|| Fr::from(0_u64)).as_le_bytes());
    }

    fn binary_len(&self) -> usize {
        64
    }
}

/// Fixed-length Compact bytes. Ledger-8 encodes every `[u8; N]` into fields,
/// but only provides `FromFieldRepr` for `[u8; 32]`; this newtype supplies the
/// matching decoder for every Compact `Bytes<N>` width.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FixedBytes<const N: usize>(pub [u8; N]);

/// Compact `Vector<N, T>` with a ledger field decoder for every element type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FixedVector<T, const N: usize>(pub [T; N]);

impl<T, const N: usize> FixedVector<T, N> {
    pub const fn new(elements: [T; N]) -> Self {
        Self(elements)
    }

    pub fn into_array(self) -> [T; N] {
        self.0
    }
}

impl<T: Default, const N: usize> Default for FixedVector<T, N> {
    fn default() -> Self {
        Self(std::array::from_fn(|_| T::default()))
    }
}

impl<T: Aligned, const N: usize> Aligned for FixedVector<T, N> {
    fn alignment() -> Alignment {
        let element = T::alignment();
        Alignment::concat(std::iter::repeat_n(&element, N))
    }
}

impl<T: Into<Value>, const N: usize> From<FixedVector<T, N>> for Value {
    fn from(value: FixedVector<T, N>) -> Self {
        Value(
            value
                .0
                .into_iter()
                .flat_map(|element| {
                    let value: Value = element.into();
                    value.0
                })
                .collect(),
        )
    }
}

impl<T: FieldRepr, const N: usize> FieldRepr for FixedVector<T, N> {
    fn field_repr<W: MemWrite<Fr>>(&self, writer: &mut W) {
        for element in &self.0 {
            element.field_repr(writer);
        }
    }

    fn field_size(&self) -> usize {
        self.0.iter().map(FieldRepr::field_size).sum()
    }
}

impl<T: BinaryHashRepr, const N: usize> BinaryHashRepr for FixedVector<T, N> {
    fn binary_repr<W: MemWrite<u8>>(&self, writer: &mut W) {
        for element in &self.0 {
            element.binary_repr(writer);
        }
    }

    fn binary_len(&self) -> usize {
        self.0.iter().map(BinaryHashRepr::binary_len).sum()
    }
}

impl<T: FromFieldRepr, const N: usize> FromFieldRepr for FixedVector<T, N> {
    const FIELD_SIZE: usize = N * T::FIELD_SIZE;

    fn from_field_repr(repr: &[Fr]) -> Option<Self> {
        if repr.len() != Self::FIELD_SIZE {
            return None;
        }
        let elements = (0..N)
            .map(|index| {
                let start = index * T::FIELD_SIZE;
                T::from_field_repr(&repr[start..start + T::FIELD_SIZE])
            })
            .collect::<Option<Vec<_>>>()?;
        Some(Self(elements.try_into().ok()?))
    }
}

impl<const N: usize> FixedBytes<N> {
    pub const fn new(bytes: [u8; N]) -> Self {
        Self(bytes)
    }

    pub const fn into_array(self) -> [u8; N] {
        self.0
    }
}

impl<const N: usize> Default for FixedBytes<N> {
    fn default() -> Self {
        Self([0; N])
    }
}

impl<const N: usize> From<[u8; N]> for FixedBytes<N> {
    fn from(value: [u8; N]) -> Self {
        Self(value)
    }
}

impl<const N: usize> From<FixedBytes<N>> for Value {
    fn from(value: FixedBytes<N>) -> Self {
        Value::from(value.0)
    }
}

impl<const N: usize> Aligned for FixedBytes<N> {
    fn alignment() -> Alignment {
        <[u8; N]>::alignment()
    }
}

impl<const N: usize> FieldRepr for FixedBytes<N> {
    fn field_repr<W: MemWrite<Fr>>(&self, writer: &mut W) {
        self.0.field_repr(writer);
    }

    fn field_size(&self) -> usize {
        self.0.field_size()
    }
}

impl<const N: usize> BinaryHashRepr for FixedBytes<N> {
    fn binary_repr<W: MemWrite<u8>>(&self, writer: &mut W) {
        self.0.binary_repr(writer);
    }

    fn binary_len(&self) -> usize {
        N
    }
}

impl<const N: usize> FromFieldRepr for FixedBytes<N> {
    const FIELD_SIZE: usize = N.div_ceil(FR_BYTES_STORED);

    fn from_field_repr(repr: &[Fr]) -> Option<Self> {
        if repr.len() != Self::FIELD_SIZE {
            return None;
        }
        let mut remaining = repr;
        let bytes = bytes_from_field_repr(&mut remaining, N)?;
        if !remaining.is_empty() {
            return None;
        }
        Some(Self(bytes.try_into().ok()?))
    }
}

/// A Compact `Uint<MAX>` value whose declared maximum is checked at creation
/// and decoding. The inner `u128` uses ledger-8's field representation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BoundedUint<const MAX: u128>(u128);

impl<const MAX: u128> Default for BoundedUint<MAX> {
    fn default() -> Self {
        Self(0)
    }
}

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

pub fn cast_unsigned<const FROM: u128, const TO: u128>(
    value: BoundedUint<FROM>,
) -> Result<BoundedUint<TO>, CompactError> {
    BoundedUint::<TO>::new(value.value())
}

pub fn add_unsigned<const LEFT: u128, const RIGHT: u128, const RESULT: u128>(
    left: BoundedUint<LEFT>,
    right: BoundedUint<RIGHT>,
) -> Result<BoundedUint<RESULT>, CompactError> {
    let sum = left
        .value()
        .checked_add(right.value())
        .ok_or(CompactError::UnsignedOverflow)?;
    BoundedUint::<RESULT>::new(sum)
}

pub fn subtract_unsigned<const LEFT: u128, const RIGHT: u128, const RESULT: u128>(
    left: BoundedUint<LEFT>,
    right: BoundedUint<RIGHT>,
) -> Result<BoundedUint<RESULT>, CompactError> {
    let difference = left
        .value()
        .checked_sub(right.value())
        .ok_or(CompactError::UnsignedUnderflow)?;
    BoundedUint::<RESULT>::new(difference)
}

pub fn multiply_unsigned<const LEFT: u128, const RIGHT: u128, const RESULT: u128>(
    left: BoundedUint<LEFT>,
    right: BoundedUint<RIGHT>,
) -> Result<BoundedUint<RESULT>, CompactError> {
    let product = left
        .value()
        .checked_mul(right.value())
        .ok_or(CompactError::UnsignedOverflow)?;
    BoundedUint::<RESULT>::new(product)
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

impl<const MAX: u128> BinaryHashRepr for BoundedUint<MAX> {
    fn binary_repr<W: MemWrite<u8>>(&self, writer: &mut W) {
        writer.write(&self.0.to_le_bytes()[..Self::BYTE_LENGTH as usize]);
    }

    fn binary_len(&self) -> usize {
        Self::BYTE_LENGTH as usize
    }
}

impl<const MAX: u128> FromFieldRepr for BoundedUint<MAX> {
    const FIELD_SIZE: usize = <u128 as FromFieldRepr>::FIELD_SIZE;

    fn from_field_repr(repr: &[Fr]) -> Option<Self> {
        let value = <u128 as FromFieldRepr>::from_field_repr(repr)?;
        Self::new(value).ok()
    }
}
