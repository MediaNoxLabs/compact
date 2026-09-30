use compact_rust_counter_fixture::ledger_contract::{increment, initial_state};
use midnight_compact_runtime::context::ConstructorContext;
use midnight_compact_runtime::ledger::{ContractAddress, StateValue, read_counter};

#[test]
fn generated_counter_contract_runs_through_ledger_vm() {
    let constructor = initial_state(ConstructorContext::new(()));
    let context = constructor.into_circuit_context(ContractAddress::default());
    let result = increment(context).unwrap();
    let StateValue::Array(fields) = result.context.query.state.get_ref() else {
        panic!("expected ledger field array")
    };
    assert_eq!(read_counter(&fields.get(0).unwrap()).unwrap(), 1);
}
