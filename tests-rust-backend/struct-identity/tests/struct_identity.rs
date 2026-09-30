use compact_rust_struct_identity_fixture::pure_circuits::pair_identity;
use compact_rust_struct_identity_fixture::types::Pair;
use midnight_compact_runtime::{Field, FieldRepr, FromFieldRepr};

#[test]
fn generated_struct_identity_and_ledger_field_repr_round_trip() {
    let value = Pair {
        amount: Field::from(42_u64),
        active: true,
    };
    assert_eq!(pair_identity(value.clone()).unwrap(), value);
    assert_eq!(Pair::from_field_repr(&value.field_vec()), Some(value));
}
