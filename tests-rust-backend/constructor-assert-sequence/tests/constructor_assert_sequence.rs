use compact_rust_constructor_assert_sequence_fixture::ledger_contract::{initial_state, read};
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
    operations = operations.insert(
        EntryPointBuf(b"read".to_vec()),
        ContractOperation::new(None),
    );
    let contract_state =
        ContractState::new(state, operations, ContractMaintenanceAuthority::default());
    let mut bytes = Vec::new();
    tagged_serialize(&contract_state, &mut bytes).unwrap();
    hex::encode(bytes)
}

#[test]
fn constructor_sequence_assertion_matches_typescript() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/constructor-assert-sequence.json"
    ))
    .unwrap();
    let initial = initial_state(ConstructorContext::new(()), true).unwrap();
    assert_eq!(
        state_hex(initial.ledger_state.get_ref().clone()),
        oracle["stateHex"]
    );
    let context = initial.into_circuit_context(ContractAddress::default());
    assert_eq!(
        read(context).unwrap().result,
        Field::from(oracle["read"].as_str().unwrap().parse::<u64>().unwrap())
    );
    let error = initial_state(ConstructorContext::new(()), false)
        .err()
        .unwrap();
    assert_eq!(error.to_string(), oracle["failure"]);
}
