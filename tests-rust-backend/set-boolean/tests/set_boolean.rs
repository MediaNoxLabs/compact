use compact_rust_set_boolean_fixture::ledger_contract::{
    add, add_field, choose, contains, contains_field, initial_state, remove, reset_fields,
    seen_is_empty, seen_size,
};
use midnight_compact_runtime::Field;
use midnight_compact_runtime::context::ConstructorContext;
use midnight_compact_runtime::ledger::{ContractAddress, DefaultDB, StateValue};
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let mut operations: HashMap<EntryPointBuf, ContractOperation, DefaultDB> = HashMap::new();
    for name in [
        "add",
        "contains",
        "add_field",
        "contains_field",
        "remove",
        "seen_size",
        "seen_is_empty",
        "reset_fields",
        "choose",
    ] {
        operations = operations.insert(
            EntryPointBuf(name.as_bytes().to_vec()),
            ContractOperation::new(None),
        );
    }
    let contract_state =
        ContractState::new(state, operations, ContractMaintenanceAuthority::default());
    let mut bytes = Vec::new();
    tagged_serialize(&contract_state, &mut bytes).unwrap();
    hex::encode(bytes)
}

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

#[test]
fn conditional_set_actions_keep_the_selected_branch_and_following_query() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/conditional-set-actions.json"
    ))
    .unwrap();
    let context = initial_state(ConstructorContext::new(()))
        .unwrap()
        .into_circuit_context(ContractAddress::default());
    let inserted = choose(context, true, true).unwrap();
    assert!(inserted.result);
    assert_eq!(
        state_hex(inserted.context.query.state.get_ref().clone()),
        oracle[0]["stateHex"]
    );
    let removed = choose(inserted.context, true, false).unwrap();
    assert!(!removed.result);
    assert_eq!(
        state_hex(removed.context.query.state.get_ref().clone()),
        oracle[1]["stateHex"]
    );
    let other = choose(removed.context, false, true).unwrap();
    assert!(other.result);
    assert_eq!(
        state_hex(other.context.query.state.get_ref().clone()),
        oracle[2]["stateHex"]
    );
    let present = contains(other.context, false).unwrap();
    assert!(present.result);
}
