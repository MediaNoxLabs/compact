use compact_rust_pure_call_action_fixture::ledger_contract::{initial_state, save};
use midnight_compact_runtime::Field;
use midnight_compact_runtime::context::ConstructorContext;
use midnight_compact_runtime::ledger::{ContractAddress, DefaultDB, StateValue};
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let operations: HashMap<EntryPointBuf, ContractOperation, DefaultDB> = HashMap::new().insert(
        EntryPointBuf(b"save".to_vec()),
        ContractOperation::new(None),
    );
    let contract_state =
        ContractState::new(state, operations, ContractMaintenanceAuthority::default());
    let mut bytes = Vec::new();
    tagged_serialize(&contract_state, &mut bytes).unwrap();
    hex::encode(bytes)
}

#[test]
fn pure_call_action_propagates_error_before_write_and_matches_success_state() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/pure-call-action.json"
    ))
    .unwrap();
    let constructor = initial_state(ConstructorContext::new(()));
    let context = constructor.into_circuit_context(ContractAddress::default());
    let error = match save(context, Field::from(0_u64)) {
        Err(error) => error,
        Ok(_) => panic!("zero input unexpectedly passed"),
    };
    assert_eq!(error.to_string(), oracle["zeroError"]);

    let constructor = initial_state(ConstructorContext::new(()));
    let context = constructor.into_circuit_context(ContractAddress::default());
    let saved = save(context, Field::from(7_u64)).unwrap();
    assert_eq!(
        state_hex(saved.context.query.state.get_ref().clone()),
        oracle["afterHex"]
    );
}
