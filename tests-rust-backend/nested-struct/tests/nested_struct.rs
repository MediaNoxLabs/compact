use compact_rust_nested_struct_fixture::pure_circuits::nested_identity;
use compact_rust_nested_struct_fixture::types::{Inner, Outer};
use midnight_compact_runtime::{BinaryHashRepr, BoundedUint, FieldRepr, FixedBytes, FromFieldRepr};

#[test]
fn generated_nested_struct_round_trips_through_ledger_field_repr() {
    let value = Outer {
        tag: FixedBytes::new([1, 2, 0, 0]),
        inner: Inner {
            value: BoundedUint::<255>::new(200).unwrap(),
        },
    };
    assert_eq!(nested_identity(value.clone()).unwrap(), value);
    assert_eq!(
        Outer::from_field_repr(&value.field_vec()),
        Some(value.clone())
    );
    assert_eq!(value.binary_vec(), vec![1, 2, 0, 0, 200]);
}
