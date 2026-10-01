use compact_rust_constructor_persistent_hash_fixture::ledger_contract::{
    initial_state, read_digest,
};
use midnight_compact_runtime::context::ConstructorContext;
use midnight_compact_runtime::ledger::{ContractAddress, DefaultDB, StateValue};
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let operations: HashMap<EntryPointBuf, ContractOperation, DefaultDB> = HashMap::new();
    let operations = operations.insert(
        EntryPointBuf(b"read_digest".to_vec()),
        ContractOperation::new(None),
    );
    let contract_state =
        ContractState::new(state, operations, ContractMaintenanceAuthority::default());
    let mut bytes = Vec::new();
    tagged_serialize(&contract_state, &mut bytes).unwrap();
    hex::encode(bytes)
}

#[test]
fn constructor_persistent_hash_matches_typescript_state() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/constructor-persistent-hash.json"
    ))
    .unwrap();
    let constructor = initial_state(ConstructorContext::new(())).unwrap();
    assert_eq!(
        state_hex(constructor.ledger_state.get_ref().clone()),
        oracle["initialHex"]
    );
    let read = read_digest(constructor.into_circuit_context(ContractAddress::default())).unwrap();
    assert_eq!(hex::encode(read.result.into_array()), oracle["digestHex"]);
}
