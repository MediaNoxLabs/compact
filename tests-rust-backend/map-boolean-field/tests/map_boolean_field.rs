use compact_rust_map_boolean_field_fixture::ledger_contract::{
    get, has, initial_state, put, put_default, remove_key, reset_table, table_is_empty, table_size,
};
use midnight_compact_runtime::Field;
use midnight_compact_runtime::context::ConstructorContext;
use midnight_compact_runtime::ledger::{ContractAddress, StateValue};

#[test]
fn generated_map_contract_inserts_and_looks_up_field_values() {
    let constructor = initial_state(ConstructorContext::new(()));
    let StateValue::Array(fields) = constructor.ledger_state.get_ref() else {
        panic!("expected ledger field array")
    };
    let StateValue::Map(map) = fields.get(0).unwrap() else {
        panic!("expected Map")
    };
    assert_eq!(map.size(), 0);

    let context = constructor.into_circuit_context(ContractAddress::default());
    let result = table_is_empty(context).unwrap();
    assert!(result.result);
    let result = table_size(result.context).unwrap();
    assert_eq!(result.result.value(), 0);
    let result = has(result.context, true).unwrap();
    assert!(!result.result);
    let result = put_default(result.context, true).unwrap();
    let result = get(result.context, true).unwrap();
    assert_eq!(result.result, Field::from(0_u64));
    let result = put(result.context, true, Field::from(42_u64)).unwrap();
    let result = table_size(result.context).unwrap();
    assert_eq!(result.result.value(), 1);
    let result = table_is_empty(result.context).unwrap();
    assert!(!result.result);
    let result = has(result.context, true).unwrap();
    assert!(result.result);
    let result = get(result.context, true).unwrap();
    assert_eq!(result.result, Field::from(42_u64));
    let result = put(result.context, true, Field::from(7_u64)).unwrap();
    let result = get(result.context, true).unwrap();
    assert_eq!(result.result, Field::from(7_u64));
    let result = put_default(result.context, true).unwrap();
    let result = get(result.context, true).unwrap();
    assert_eq!(result.result, Field::from(0_u64));
    let result = has(result.context, false).unwrap();
    assert!(!result.result);
    let StateValue::Array(fields) = result.context.query.state.get_ref() else {
        panic!("expected ledger field array")
    };
    let StateValue::Map(map) = fields.get(0).unwrap() else {
        panic!("expected Map")
    };
    assert_eq!(map.size(), 1);

    let result = remove_key(result.context, true).unwrap();
    let result = table_size(result.context).unwrap();
    assert_eq!(result.result.value(), 0);
    let result = table_is_empty(result.context).unwrap();
    assert!(result.result);
    let result = put(result.context, false, Field::from(11_u64)).unwrap();
    let result = reset_table(result.context).unwrap();
    let result = table_size(result.context).unwrap();
    assert_eq!(result.result.value(), 0);
    let result = table_is_empty(result.context).unwrap();
    assert!(result.result);
}
