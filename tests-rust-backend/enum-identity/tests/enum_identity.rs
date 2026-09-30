use compact_rust_enum_identity_fixture::pure_circuits::choice_identity;
use compact_rust_enum_identity_fixture::types::Choice;
use midnight_compact_runtime::{BinaryHashRepr, Field, FieldRepr, FromFieldRepr};

#[test]
fn generated_enum_identity_and_ordinal_field_repr_round_trip() {
    assert_eq!(choice_identity(Choice::no).unwrap(), Choice::no);
    assert_eq!(Choice::no.field_vec(), vec![Field::from(1_u64)]);
    assert_eq!(
        Choice::from_field_repr(&Choice::yes.field_vec()),
        Some(Choice::yes)
    );
    assert_eq!(Choice::from_field_repr(&[Field::from(2_u64)]), None);
    assert_eq!(Choice::no.binary_vec(), vec![1]);
}
