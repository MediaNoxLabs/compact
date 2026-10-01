use compact_rust_opaque_string_set_oracle_fixture::ledger_contract::{
    addName, hasName, initial_state,
};
use midnight_compact_runtime::OpaqueString;
use midnight_compact_runtime::context::ConstructorContext;
use midnight_compact_runtime::ledger::{ContractAddress, DefaultDB, StateValue};
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let mut operations: HashMap<EntryPointBuf, ContractOperation, DefaultDB> = HashMap::new();
    for name in ["addName", "hasName"] {
        operations = operations.insert(
            EntryPointBuf(name.as_bytes().to_vec()),
            ContractOperation::new(None),
        );
    }
    let state = ContractState::new(state, operations, ContractMaintenanceAuthority::default());
    let mut bytes = Vec::new();
    tagged_serialize(&state, &mut bytes).unwrap();
    hex::encode(bytes)
}

#[test]
fn variable_length_set_key_matches_typescript_state() {
    let reference: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/opaque-string-set-oracle.json"
    ))
    .unwrap();
    let initial = initial_state(ConstructorContext::new(7_u64)).unwrap();
    assert_eq!(
        state_hex(initial.ledger_state.get_ref().clone()),
        reference["initialHex"]
    );
    let context = initial.into_circuit_context(ContractAddress::default());
    let key = OpaqueString::from("registry-key");
    let before = hasName(context, key.clone()).unwrap();
    assert_eq!(before.result, reference["before"].as_bool().unwrap());
    let added = addName(before.context, key.clone()).unwrap();
    assert_eq!(
        state_hex(added.context.query.state.get_ref().clone()),
        reference["afterAddHex"]
    );
    let after = hasName(added.context, key).unwrap();
    assert_eq!(after.result, reference["after"].as_bool().unwrap());
    let other = hasName(after.context, OpaqueString::from("other")).unwrap();
    assert_eq!(other.result, reference["other"].as_bool().unwrap());
}
