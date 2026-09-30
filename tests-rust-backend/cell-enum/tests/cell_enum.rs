use compact_rust_cell_enum_fixture::ledger_contract::initial_state;
use compact_rust_cell_enum_fixture::types::Choice;
use midnight_compact_runtime::context::ConstructorContext;
use midnight_compact_runtime::fab::{Aligned, Alignment, AlignmentAtom};
use midnight_compact_runtime::ledger::{
    ContractAddress, DefaultDB, StateValue, constructor_cell, read_cell,
};

#[test]
fn generated_enum_cell_has_ledger_alignment_and_round_trips() {
    assert_eq!(
        Choice::alignment(),
        Alignment::singleton(AlignmentAtom::Bytes { length: 1 })
    );
    let constructor = initial_state(ConstructorContext::new(()));
    let StateValue::Array(fields) = constructor.ledger_state.get_ref() else {
        panic!("expected ledger field array")
    };
    assert_eq!(
        read_cell::<Choice, _>(&fields.get(0).unwrap()).unwrap(),
        Choice::yes
    );

    let cell = constructor_cell::<Choice, DefaultDB>(Choice::no);
    assert_eq!(read_cell::<Choice, _>(&cell).unwrap(), Choice::no);

    let context = constructor.into_circuit_context(ContractAddress::default());
    let result = context.write_cell(0, Choice::no).unwrap();
    let StateValue::Array(fields) = result.context.query.state.get_ref() else {
        panic!("expected ledger field array")
    };
    assert_eq!(
        read_cell::<Choice, _>(&fields.get(0).unwrap()).unwrap(),
        Choice::no
    );
}
