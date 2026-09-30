use midnight_base_crypto::fab::{
    Aligned, AlignedValue, Alignment, AlignmentAtom, Value, ValueAtom,
};
use midnight_compact_runtime::Field;

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
