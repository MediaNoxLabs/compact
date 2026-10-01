use compact_rust_kernel_self_oracle_fixture::ledger_contract::{initial_state, readAddress};
use midnight_compact_runtime::context::ConstructorContext;
use midnight_compact_runtime::ledger::{ContractAddress, DefaultDB, StateValue};
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let operations: HashMap<EntryPointBuf, ContractOperation, DefaultDB> = HashMap::new().insert(
        EntryPointBuf(b"readAddress".to_vec()),
        ContractOperation::new(None),
    );
    let state = ContractState::new(state, operations, ContractMaintenanceAuthority::default());
    let mut bytes = Vec::new();
    tagged_serialize(&state, &mut bytes).unwrap();
    hex::encode(bytes)
}

#[test]
fn constructor_kernel_self_matches_typescript_address_cell() {
    let reference: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/kernel-self-oracle.json"
    ))
    .unwrap();
    let initial = initial_state(ConstructorContext::new(7_u64)).unwrap();
    assert_eq!(
        initial.private_state,
        reference["privateState"].as_u64().unwrap()
    );
    assert_eq!(
        state_hex(initial.ledger_state.get_ref().clone()),
        reference["initialHex"]
    );
    let read = readAddress(initial.into_circuit_context(ContractAddress::default())).unwrap();
    assert_eq!(
        serde_json::to_value(read.result.bytes.0).unwrap(),
        reference["addressBytes"]
    );
}
