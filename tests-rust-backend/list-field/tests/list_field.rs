use compact_rust_list_field_fixture::ledger_contract::{
    clear_items, drop_first, initial_state, item_count, items_empty, prepend,
};
use midnight_compact_runtime::Field;
use midnight_compact_runtime::context::ConstructorContext;
use midnight_compact_runtime::ledger::{ContractAddress, StateValue};

#[test]
fn generated_empty_list_has_upstream_shape_and_zero_length() {
    let constructor = initial_state(ConstructorContext::new(()));
    let StateValue::Array(fields) = constructor.ledger_state.get_ref() else {
        panic!("expected ledger root array")
    };
    let StateValue::Array(list) = fields.get(0).unwrap() else {
        panic!("expected List array")
    };
    assert!(matches!(list.get(0), Some(StateValue::Null)));
    assert!(matches!(list.get(1), Some(StateValue::Null)));
    assert_eq!(list.len(), 3);
    let context = constructor.into_circuit_context(ContractAddress::default());
    let result = item_count(context).unwrap();
    assert_eq!(result.result.value(), 0);
    let result = items_empty(result.context).unwrap();
    assert!(result.result);
    let result = prepend(result.context, Field::from(11_u64)).unwrap();
    let result = item_count(result.context).unwrap();
    assert_eq!(result.result.value(), 1);
    let result = items_empty(result.context).unwrap();
    assert!(!result.result);
    let result = prepend(result.context, Field::from(22_u64)).unwrap();
    let result = item_count(result.context).unwrap();
    assert_eq!(result.result.value(), 2);
    let result = drop_first(result.context).unwrap();
    let result = item_count(result.context).unwrap();
    assert_eq!(result.result.value(), 1);
    let result = clear_items(result.context).unwrap();
    let result = item_count(result.context).unwrap();
    assert_eq!(result.result.value(), 0);
    let result = items_empty(result.context).unwrap();
    assert!(result.result);
    let result = prepend(result.context, Field::from(33_u64)).unwrap();
    let result = item_count(result.context).unwrap();
    assert_eq!(result.result.value(), 1);
}
