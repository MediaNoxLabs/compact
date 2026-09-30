use compact_rust_field_add_fixture::pure_circuits::field_add;
use midnight_compact_runtime::Field;

#[test]
fn field_addition_uses_the_ledger_field_implementation() {
    assert_eq!(
        field_add(Field::from(3_u64), Field::from(5_u64)).unwrap(),
        Field::from(8_u64)
    );
}
