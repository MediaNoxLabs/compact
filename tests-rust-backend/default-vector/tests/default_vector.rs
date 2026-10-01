use compact_rust_default_vector_fixture::ledger_contract::{initial_state, read_values};
use compact_rust_default_vector_fixture::pure_circuits::{empty_hash, empty_vector};
use midnight_compact_runtime::context::ConstructorContext;
use midnight_compact_runtime::ledger::{ContractAddress, DefaultDB, StateValue};
use midnight_compact_runtime::{Field, FixedVector};
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let operations: HashMap<EntryPointBuf, ContractOperation, DefaultDB> = HashMap::new();
    let operations = operations.insert(
        EntryPointBuf(b"read_values".to_vec()),
        ContractOperation::new(None),
    );
    let contract_state =
        ContractState::new(state, operations, ContractMaintenanceAuthority::default());
    let mut bytes = Vec::new();
    tagged_serialize(&contract_state, &mut bytes).unwrap();
    hex::encode(bytes)
}

#[test]
fn typed_default_vector_matches_typescript_state_and_hash() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/default-vector.json"
    ))
    .unwrap();
    let constructor = initial_state(ConstructorContext::new(())).unwrap();
    assert_eq!(
        state_hex(constructor.ledger_state.get_ref().clone()),
        oracle["initialHex"]
    );
    let expected = FixedVector::new([Field::from(0_u64); 2]);
    assert_eq!(empty_vector().unwrap(), expected);
    assert_eq!(oracle["vector"], serde_json::json!(["0", "0"]));
    assert_eq!(
        hex::encode(empty_hash().unwrap().into_array()),
        oracle["hashHex"]
    );
    let read = read_values(constructor.into_circuit_context(ContractAddress::default())).unwrap();
    assert_eq!(read.result, expected);
    assert_eq!(oracle["read"], oracle["vector"]);
}
