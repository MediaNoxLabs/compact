use midnight_compact_runtime::{Field, FieldRepr, Fr, FromFieldRepr, MemWrite};

#[derive(Debug, PartialEq, Eq, FieldRepr, FromFieldRepr)]
struct Pair {
    left: Field,
    right: Field,
}

#[test]
fn ledger_derive_round_trips_a_compact_struct() {
    let value = Pair {
        left: Field::from(3_u64),
        right: Field::from(5_u64),
    };
    let encoded = value.field_vec();
    assert_eq!(encoded.len(), 2);
    assert_eq!(Pair::from_field_repr(&encoded), Some(value));
}
