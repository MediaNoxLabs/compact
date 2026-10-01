use compact_rust_cell_boolean_fixture::ledger_contract::{initial_state, set_flag};
use midnight_compact_runtime::context::ConstructorContext;
use midnight_compact_runtime::ledger::{ContractAddress, StateValue, read_cell};

#[test]
fn generated_cell_contract_writes_through_ledger_vm() {
    let constructor = initial_state(ConstructorContext::new(())).unwrap();
    let StateValue::Array(initial_fields) = constructor.ledger_state.get_ref() else {
        panic!("expected ledger field array")
    };
    assert!(!read_cell::<bool, _>(&initial_fields.get(0).unwrap()).unwrap());

    let context = constructor.into_circuit_context(ContractAddress::default());
    let result = set_flag(context).unwrap();
    let StateValue::Array(fields) = result.context.query.state.get_ref() else {
        panic!("expected ledger field array")
    };
    assert!(read_cell::<bool, _>(&fields.get(0).unwrap()).unwrap());
}
