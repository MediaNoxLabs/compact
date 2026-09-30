use midnight_base_crypto::fab::{
    Aligned, AlignedValue, Alignment, AlignmentAtom, Value, ValueAtom,
};
use midnight_compact_runtime::{BoundedUint, CompactError, Field, FieldRepr, FromFieldRepr};

#[test]
fn compact_uint_maximum_is_enforced_at_input_and_decode() {
    type Small = BoundedUint<8>;
    assert_eq!(Small::new(8).unwrap().value(), 8);
    assert_eq!(
        Small::new(9),
        Err(CompactError::UnsignedOutOfRange { value: 9, max: 8 })
    );
    assert_eq!(Small::from_field_repr(&[Field::from(9_u64)]), None);
}

#[test]
fn compact_uint_round_trips_through_the_ledger_field_representation() {
    type Large = BoundedUint<{ u128::MAX }>;
    let value = Large::new(u128::MAX).unwrap();
    assert_eq!(Large::from_field_repr(&value.field_vec()), Some(value));
}

#[test]
fn compact_uint_uses_the_declared_byte_alignment_and_ledger_value() {
    type Byte = BoundedUint<255>;
    assert_eq!(Byte::BYTE_LENGTH, 1);
    assert_eq!(
        Byte::alignment(),
        Alignment::singleton(AlignmentAtom::Bytes { length: 1 })
    );
    let value = Byte::new(254).unwrap();
    let aligned = AlignedValue::from(value);
    assert_eq!(aligned.value, Value(vec![ValueAtom(vec![254])]));
    assert_eq!(Byte::try_from(&*aligned.as_slice()), Ok(value));
    assert_eq!(
        Byte::try_from(&*Value(vec![ValueAtom(vec![0, 1])])),
        Err(CompactError::UnsignedOutOfRange {
            value: 256,
            max: 255
        })
    );
    assert_eq!(BoundedUint::<0>::BYTE_LENGTH, 0);
}
