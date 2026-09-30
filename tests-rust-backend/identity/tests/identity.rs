use compact_rust_identity_fixture::pure_circuits::identity;
use midnight_compact_runtime::Field;

#[test]
fn pure_field_identity_uses_the_ledger_field_type() {
    let value = Field::from(42_u64);
    assert_eq!(identity(value).unwrap(), value);
}
