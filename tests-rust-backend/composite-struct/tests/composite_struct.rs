use compact_rust_composite_struct_fixture::pure_circuits::composite_identity;
use compact_rust_composite_struct_fixture::types::Composite;
use midnight_compact_runtime::{Field, FieldRepr, FixedVector, FromFieldRepr};

#[test]
fn generated_vector_and_tuple_fields_round_trip() {
    let value = Composite {
        vector: FixedVector::new([Field::from(1_u64), Field::from(2_u64)]),
        pair: (Field::from(3_u64), true),
    };
    assert_eq!(composite_identity(value.clone()).unwrap(), value);
    assert_eq!(Composite::from_field_repr(&value.field_vec()), Some(value));
}
