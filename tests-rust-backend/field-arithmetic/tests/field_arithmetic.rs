use compact_rust_field_arithmetic_fixture::pure_circuits::{multiply, subtract};
use midnight_compact_runtime::Field;

#[test]
fn generated_field_arithmetic_uses_ledger_field_operations() {
    assert_eq!(
        subtract(Field::from(9_u64), Field::from(4_u64)).unwrap(),
        Field::from(5_u64)
    );
    assert_eq!(
        subtract(Field::from(0_u64), Field::from(1_u64)).unwrap(),
        -Field::from(1_u64)
    );
    assert_eq!(
        multiply(Field::from(7_u64), Field::from(6_u64)).unwrap(),
        Field::from(42_u64)
    );
}
