use compact_rust_cell_parameter_fixture::ledger_contract::{initial_state, set_flag};
use midnight_compact_runtime::context::ConstructorContext;
use midnight_compact_runtime::ledger::{ContractAddress, StateValue, read_cell};

#[test]
fn generated_parameterized_cell_write_uses_supplied_value() {
    let constructor = initial_state(ConstructorContext::new(())).unwrap();
    let context = constructor.into_circuit_context(ContractAddress::default());
    let result = set_flag(context, true).unwrap();
    let StateValue::Array(fields) = result.context.query.state.get_ref() else {
        panic!("expected ledger field array")
    };
    assert!(read_cell::<bool, _>(&fields.get(0).unwrap()).unwrap());

    let result = set_flag(result.context, false).unwrap();
    let StateValue::Array(fields) = result.context.query.state.get_ref() else {
        panic!("expected ledger field array")
    };
    assert!(!read_cell::<bool, _>(&fields.get(0).unwrap()).unwrap());
}
