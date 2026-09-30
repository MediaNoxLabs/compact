use midnight_compact_runtime::Field;
use midnight_compact_runtime::ledger::{
    ChargedState, ContractAddress, DefaultDB, QueryContext, StateValue, constructor_list,
    constructor_map, constructor_set, insert_map, insert_set, push_front_list,
};
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_onchain_vm::cost_model::INITIAL_COST_MODEL;
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;

fn expected_bytes(json: &str) -> Vec<u8> {
    let reference: serde_json::Value = serde_json::from_str(json).unwrap();
    hex::decode(reference["stateHex"].as_str().unwrap()).unwrap()
}

fn state_bytes(state: StateValue<DefaultDB>, entry_points: &[&str]) -> Vec<u8> {
    let mut operations: HashMap<EntryPointBuf, ContractOperation, DefaultDB> = HashMap::new();
    for name in entry_points {
        operations = operations.insert(
            EntryPointBuf(name.as_bytes().to_vec()),
            ContractOperation::new(None),
        );
    }
    let contract_state =
        ContractState::new(state, operations, ContractMaintenanceAuthority::default());
    let mut actual = Vec::new();
    tagged_serialize(&contract_state, &mut actual).unwrap();
    actual
}

fn context(fields: Vec<StateValue<DefaultDB>>) -> QueryContext<DefaultDB> {
    QueryContext::new(
        ChargedState::new(StateValue::Array(fields.into())),
        ContractAddress::default(),
    )
}

#[test]
fn set_insert_state_matches_typescript_transition_bytes() {
    let expected = expected_bytes(include_str!("fixtures/set-ts-after-add.json"));
    let result = insert_set(
        &context(vec![constructor_set(), constructor_set()]),
        0,
        true,
        None,
        &INITIAL_COST_MODEL,
    )
    .unwrap();
    let actual = state_bytes(
        result.context.state.get_ref().clone(),
        &[
            "add",
            "contains",
            "add_field",
            "contains_field",
            "remove",
            "seen_size",
            "seen_is_empty",
            "reset_fields",
        ],
    );
    assert_eq!(actual, expected);
}

#[test]
fn map_insert_state_matches_typescript_transition_bytes() {
    let expected = expected_bytes(include_str!("fixtures/map-ts-after-put.json"));
    let result = insert_map(
        &context(vec![constructor_map()]),
        0,
        true,
        Field::from(42_u64),
        None,
        &INITIAL_COST_MODEL,
    )
    .unwrap();
    let actual = state_bytes(
        result.context.state.get_ref().clone(),
        &[
            "put",
            "put_default",
            "has",
            "get",
            "remove_key",
            "table_size",
            "table_is_empty",
            "reset_table",
        ],
    );
    assert_eq!(actual, expected);
}

#[test]
fn list_prepend_state_matches_typescript_transition_bytes() {
    let expected = expected_bytes(include_str!("fixtures/list-ts-after-prepend.json"));
    let result = push_front_list(
        &context(vec![constructor_list()]),
        0,
        Field::from(42_u64),
        None,
        &INITIAL_COST_MODEL,
    )
    .unwrap();
    let actual = state_bytes(
        result.context.state.get_ref().clone(),
        &[
            "item_count",
            "items_empty",
            "first_item",
            "prepend",
            "drop_first",
            "clear_items",
        ],
    );
    assert_eq!(actual, expected);
}
