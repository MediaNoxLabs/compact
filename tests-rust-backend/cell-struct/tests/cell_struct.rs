use compact_rust_cell_struct_fixture::ledger_contract::{initial_state, read_record, set_record};
use compact_rust_cell_struct_fixture::types::Pair;
use midnight_compact_runtime::Field;
use midnight_compact_runtime::context::ConstructorContext;
use midnight_compact_runtime::fab::{Aligned, Alignment, AlignmentAtom};
use midnight_compact_runtime::ledger::{
    ContractAddress, DefaultDB, StateValue, constructor_cell, read_cell,
};

#[test]
fn generated_struct_cell_has_ledger_alignment_and_round_trips() {
    assert_eq!(
        Pair::alignment(),
        Alignment::concat(
            [
                Alignment::singleton(AlignmentAtom::Field),
                Alignment::singleton(AlignmentAtom::Bytes { length: 1 }),
            ]
            .iter()
        ),
    );
    let constructor = initial_state(ConstructorContext::new(())).unwrap();
    let StateValue::Array(fields) = constructor.ledger_state.get_ref() else {
        panic!("expected ledger field array")
    };
    assert_eq!(
        read_cell::<Pair, _>(&fields.get(0).unwrap()).unwrap(),
        Pair::default()
    );

    let value = Pair {
        amount: Field::from(42_u64),
        active: true,
    };
    let cell = constructor_cell::<Pair, DefaultDB>(value.clone());
    assert_eq!(read_cell::<Pair, _>(&cell).unwrap(), value);

    let context = constructor.into_circuit_context(ContractAddress::default());
    let result = set_record(context, value.clone()).unwrap();
    let StateValue::Array(fields) = result.context.query.state.get_ref() else {
        panic!("expected ledger field array")
    };
    assert_eq!(
        read_cell::<Pair, _>(&fields.get(0).unwrap()).unwrap(),
        value
    );
    let read = read_record(result.context).unwrap();
    assert_eq!(read.result, value);
}
