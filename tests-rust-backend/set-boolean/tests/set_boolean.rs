use compact_rust_set_boolean_fixture::ledger_contract::{add, contains, initial_state};
use midnight_compact_runtime::context::ConstructorContext;
use midnight_compact_runtime::ledger::{ContractAddress, StateValue};

#[test]
fn generated_set_contract_inserts_and_checks_membership() {
    let constructor = initial_state(ConstructorContext::new(()));
    let StateValue::Array(fields) = constructor.ledger_state.get_ref() else {
        panic!("expected ledger field array")
    };
    let StateValue::Map(set) = fields.get(0).unwrap() else {
        panic!("expected Set map")
    };
    assert_eq!(set.size(), 0);

    let context = constructor.into_circuit_context(ContractAddress::default());
    let result = contains(context, true).unwrap();
    assert!(!result.result);
    let result = add(result.context, true).unwrap();
    let result = contains(result.context, true).unwrap();
    assert!(result.result);
    let result = contains(result.context, false).unwrap();
    assert!(!result.result);
    let result = add(result.context, true).unwrap();
    let StateValue::Array(fields) = result.context.query.state.get_ref() else {
        panic!("expected ledger field array")
    };
    let StateValue::Map(set) = fields.get(0).unwrap() else {
        panic!("expected Set map")
    };
    assert_eq!(set.size(), 1);
}
