use midnight_base_crypto::fab::{
    Aligned, AlignedValue, Alignment, AlignmentAtom, Value, ValueAtom,
};
use midnight_compact_runtime::{
    BinaryHashRepr, Field, FieldRepr, FixedBytes, FixedVector, FromFieldRepr,
};

#[test]
fn boolean_uses_ledger_single_byte_alignment_and_normalized_values() {
    assert_eq!(
        bool::alignment(),
        Alignment::singleton(AlignmentAtom::Bytes { length: 1 })
    );
    assert_eq!(
        AlignedValue::from(false).value,
        Value(vec![ValueAtom(vec![])])
    );
    assert_eq!(
        AlignedValue::from(true).value,
        Value(vec![ValueAtom(vec![1])])
    );
}

#[test]
fn field_uses_ledger_field_alignment_and_field_value() {
    let value = Field::from(42_u64);
    assert_eq!(
        Field::alignment(),
        Alignment::singleton(AlignmentAtom::Field)
    );
    assert_eq!(
        AlignedValue::from(value).value,
        Value(vec![ValueAtom(vec![42])])
    );
}

#[test]
fn fixed_bytes_use_upstream_alignment_value_and_field_encoding() {
    let bytes = FixedBytes::<4>::new([1, 2, 0, 0]);
    assert_eq!(FixedBytes::<4>::alignment(), <[u8; 4]>::alignment());
    assert_eq!(
        AlignedValue::from(bytes).value,
        AlignedValue::from([1, 2, 0, 0]).value
    );
    assert_eq!(bytes.field_vec(), [1, 2, 0, 0].field_vec());
    assert_eq!(
        FixedBytes::<4>::from_field_repr(&bytes.field_vec()),
        Some(bytes)
    );
    assert_eq!(bytes.binary_vec(), vec![1, 2, 0, 0]);

    let wide = FixedBytes::<32>::new([7; 32]);
    assert_eq!(wide.field_vec(), [7; 32].field_vec());
    assert_eq!(
        FixedBytes::<32>::from_field_repr(&wide.field_vec()),
        Some(wide)
    );
}

#[test]
fn fixed_vector_concatenates_upstream_element_representations() {
    let vector = FixedVector::<Field, 2>::new([Field::from(3_u64), Field::from(5_u64)]);
    assert_eq!(
        vector.field_vec(),
        vec![Field::from(3_u64), Field::from(5_u64)]
    );
    assert_eq!(
        FixedVector::<Field, 2>::from_field_repr(&vector.field_vec()),
        Some(vector.clone())
    );
    assert_eq!(vector.0.len(), 2);
    assert_eq!(vector.binary_len(), 2 * Field::from(0_u64).binary_len());
    assert_eq!(AlignedValue::from(vector).alignment.0.len(), 2);
}

#[test]
fn nested_vectors_and_tuple_elements_have_recursive_field_representations() {
    let nested = FixedVector::new([
        FixedVector::new([Field::from(1_u64), Field::from(2_u64)]),
        FixedVector::new([Field::from(3_u64), Field::from(4_u64)]),
    ]);
    assert_eq!(nested.field_vec().len(), 4);
    assert_eq!(
        FixedVector::<FixedVector<Field, 2>, 2>::from_field_repr(&nested.field_vec()),
        Some(nested)
    );

    let tuples = FixedVector::new([(Field::from(5_u64), true), (Field::from(6_u64), false)]);
    assert_eq!(
        FixedVector::<(Field, bool), 2>::from_field_repr(&tuples.field_vec()),
        Some(tuples)
    );
}
