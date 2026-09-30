use midnight_base_crypto::fab::{AlignedValue, Alignment, AlignmentAtom, Value, ValueAtom};
use midnight_compact_runtime::BoundedUint;
use midnight_compact_runtime::CompactError;
use midnight_compact_runtime::ledger::{
    DefaultDB, StateValue, constructor_cell, constructor_counter, read_cell, read_counter,
};

#[test]
fn constructor_cell_round_trips_and_checks_alignment() {
    let state = constructor_cell::<_, DefaultDB>([1_u8, 2, 0, 0]);
    assert_eq!(read_cell::<[u8; 4], _>(&state).unwrap(), [1, 2, 0, 0]);
    assert!(matches!(
        read_cell::<u64, _>(&state),
        Err(CompactError::InvalidLedgerCell(_))
    ));
    assert!(matches!(
        read_cell::<bool, _>(&StateValue::<DefaultDB>::Null),
        Err(CompactError::InvalidLedgerCell(_))
    ));
}

#[test]
fn counter_constructor_uses_ledger_uint64_cell_shape() {
    let state = constructor_counter::<DefaultDB>();
    let StateValue::Cell(cell) = &state else {
        panic!("counter must be Cell")
    };
    assert_eq!(
        **cell,
        AlignedValue::new(
            Value(vec![ValueAtom(vec![])]),
            Alignment::singleton(AlignmentAtom::Bytes { length: 8 })
        )
        .unwrap()
    );
    assert_eq!(read_counter(&state).unwrap(), 0);
}

#[test]
fn bounded_uint_cell_rejects_a_value_above_its_declared_maximum() {
    type Small = BoundedUint<8>;
    let cell = constructor_cell::<_, DefaultDB>(Small::new(8).unwrap());
    assert_eq!(read_cell::<Small, _>(&cell).unwrap().value(), 8);
    let wider = constructor_cell::<_, DefaultDB>(9_u8);
    assert!(matches!(
        read_cell::<Small, _>(&wider),
        Err(CompactError::UnsignedOutOfRange { value: 9, max: 8 })
    ));
}
