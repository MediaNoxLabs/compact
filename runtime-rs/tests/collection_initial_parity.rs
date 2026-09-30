use midnight_compact_runtime::ledger::{
    DefaultDB, StateValue, constructor_cell, constructor_list, constructor_map, constructor_set,
};
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;

fn state_hex(fields: Vec<StateValue<DefaultDB>>, entry_point: &str) -> String {
    let mut operations: HashMap<EntryPointBuf, ContractOperation, DefaultDB> = HashMap::new();
    operations = operations.insert(
        EntryPointBuf(entry_point.as_bytes().to_vec()),
        ContractOperation::new(None),
    );
    let contract_state = ContractState::new(
        StateValue::Array(fields.into()),
        operations,
        ContractMaintenanceAuthority::default(),
    );
    let mut bytes = Vec::new();
    tagged_serialize(&contract_state, &mut bytes).unwrap();
    hex::encode(bytes)
}

fn reference_hex(json: &str) -> String {
    let reference: serde_json::Value = serde_json::from_str(json).unwrap();
    reference["afterInit"]["stateHex"].as_str().unwrap().into()
}

#[test]
fn set_constructor_matches_typescript_oracle_bytes() {
    let expected = reference_hex(include_str!("fixtures/set-ts-initial-state.json"));
    let actual = state_hex(vec![constructor_cell(false), constructor_set()], "check");
    assert_eq!(actual, expected);
}

#[test]
fn map_constructor_matches_typescript_oracle_bytes() {
    let expected = reference_hex(include_str!("fixtures/map-ts-initial-state.json"));
    let actual = state_hex(vec![constructor_map()], "put");
    assert_eq!(actual, expected);
}

#[test]
fn list_constructor_matches_typescript_oracle_bytes() {
    let expected = reference_hex(include_str!("fixtures/list-ts-initial-state.json"));
    let actual = state_hex(vec![constructor_cell(false), constructor_list()], "ping");
    assert_eq!(actual, expected);
}
