use compact_rust_set_boolean_fixture::ledger_contract::{
    add, add_field, contains, contains_field, initial_state, remove, reset_fields, seen_is_empty,
    seen_size,
};
use midnight_compact_runtime::Field;
use midnight_compact_runtime::context::ConstructorContext;
use midnight_compact_runtime::ledger::{ContractAddress, StateValue};

#[test]
fn generated_set_contract_inserts_and_checks_membership() {
    let constructor = initial_state(ConstructorContext::new(())).unwrap();
    let StateValue::Array(fields) = constructor.ledger_state.get_ref() else {
        panic!("expected ledger field array")
    };
    let StateValue::Map(set) = fields.get(0).unwrap() else {
        panic!("expected Set map")
    };
    assert_eq!(set.size(), 0);
    let StateValue::Map(set) = fields.get(1).unwrap() else {
        panic!("expected Field Set map")
    };
    assert_eq!(set.size(), 0);

    let context = constructor.into_circuit_context(ContractAddress::default());
    let result = seen_is_empty(context).unwrap();
    assert!(result.result);
    let result = seen_size(result.context).unwrap();
    assert_eq!(result.result.value(), 0);
    let result = contains(result.context, true).unwrap();
    assert!(!result.result);
    let result = add(result.context, true).unwrap();
    let result = contains(result.context, true).unwrap();
    assert!(result.result);
    let result = contains(result.context, false).unwrap();
    assert!(!result.result);
    let result = seen_size(result.context).unwrap();
    assert_eq!(result.result.value(), 1);
    let result = seen_is_empty(result.context).unwrap();
    assert!(!result.result);
    let result = add(result.context, true).unwrap();
    let result = contains_field(result.context, Field::from(42_u64)).unwrap();
    assert!(!result.result);
    let result = add_field(result.context, Field::from(42_u64)).unwrap();
    let result = contains_field(result.context, Field::from(42_u64)).unwrap();
    assert!(result.result);
    let result = contains_field(result.context, Field::from(43_u64)).unwrap();
    assert!(!result.result);
    let StateValue::Array(fields) = result.context.query.state.get_ref() else {
        panic!("expected ledger field array")
    };
    let StateValue::Map(set) = fields.get(0).unwrap() else {
        panic!("expected Set map")
    };
    assert_eq!(set.size(), 1);
    let StateValue::Map(set) = fields.get(1).unwrap() else {
        panic!("expected Field Set map")
    };
    assert_eq!(set.size(), 1);

    let result = remove(result.context, true).unwrap();
    let result = seen_is_empty(result.context).unwrap();
    assert!(result.result);
    let result = seen_size(result.context).unwrap();
    assert_eq!(result.result.value(), 0);
    let StateValue::Array(fields) = result.context.query.state.get_ref() else {
        panic!("expected ledger field array")
    };
    let StateValue::Map(set) = fields.get(1).unwrap() else {
        panic!("expected Field Set map")
    };
    assert_eq!(set.size(), 1);

    let result = reset_fields(result.context).unwrap();
    let StateValue::Array(fields) = result.context.query.state.get_ref() else {
        panic!("expected ledger field array")
    };
    let StateValue::Map(set) = fields.get(1).unwrap() else {
        panic!("expected Field Set map")
    };
    assert_eq!(set.size(), 0);
}
