use compact_rust_counter_fixture::ledger_contract::{increment, initial_state, read_round};
use midnight_compact_runtime::context::ConstructorContext;
use midnight_compact_runtime::ledger::{ContractAddress, StateValue, read_counter};

#[test]
fn generated_counter_contract_runs_through_ledger_vm() {
    let constructor = initial_state(ConstructorContext::new(())).unwrap();
    let context = constructor.into_circuit_context(ContractAddress::default());
    let read = read_round(context).unwrap();
    assert_eq!(read.result.value(), 0);
    let result = increment(read.context).unwrap();
    let StateValue::Array(fields) = result.context.query.state.get_ref() else {
        panic!("expected ledger field array")
    };
    assert_eq!(read_counter(&fields.get(0).unwrap()).unwrap(), 1);
    let read = read_round(result.context).unwrap();
    assert_eq!(read.result.value(), 1);
}
