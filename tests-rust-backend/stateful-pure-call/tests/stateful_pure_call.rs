use compact_rust_stateful_pure_call_fixture::ledger_contract::{initial_state, read_stored, save};
use compact_rust_stateful_pure_call_fixture::pure_circuits::square;
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
    for name in ["save", "read_stored"] {
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
fn stateful_pure_helper_calls_match_typescript_and_ledger_bytes() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/stateful-pure-call.json"
    ))
    .unwrap();
    let input = Field::from(7_u64);
    let expected = Field::from(oracle["pure"].as_str().unwrap().parse::<u64>().unwrap());
    assert_eq!(square(input).unwrap(), expected);
    let context =
        initial_state(ConstructorContext::new(())).into_circuit_context(ContractAddress::default());
    let saved = save(context, input).unwrap();
    assert_eq!(saved.result, expected);
    let read = read_stored(saved.context).unwrap();
    assert_eq!(read.result, expected);
    assert_eq!(
        state_hex(read.context.query.state.get_ref().clone()),
        oracle["stateHex"]
    );
}
