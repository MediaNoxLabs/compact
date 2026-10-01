use compact_rust_hash_to_curve_vector_constructor_fixture::ledger_contract::{initial_state, ping};
use midnight_compact_runtime::context::ConstructorContext;
use midnight_compact_runtime::ledger::{ContractAddress, DefaultDB, StateValue};
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let operations: HashMap<EntryPointBuf, ContractOperation, DefaultDB> = HashMap::new().insert(
        EntryPointBuf(b"ping".to_vec()),
        ContractOperation::new(None),
    );
    let contract_state =
        ContractState::new(state, operations, ContractMaintenanceAuthority::default());
    let mut bytes = Vec::new();
    tagged_serialize(&contract_state, &mut bytes).unwrap();
    hex::encode(bytes)
}

#[test]
fn curve_coordinate_vector_constructor_matches_typescript_state() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/hash-to-curve-vector-constructor.json"
    ))
    .unwrap();
    let constructor = initial_state(ConstructorContext::new(()));
    assert_eq!(
        state_hex(constructor.ledger_state.get_ref().clone()),
        oracle["initialHex"]
    );
    let result = ping(constructor.into_circuit_context(ContractAddress::default())).unwrap();
    assert_eq!(
        state_hex(result.context.query.state.get_ref().clone()),
        oracle["afterHex"]
    );
}
