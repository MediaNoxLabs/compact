use compact_rust_stateful_circuit_call_fixture::ledger_contract::{
    add_twice, bump_twice, initial_state,
};
use midnight_compact_runtime::BoundedUint;
use midnight_compact_runtime::context::ConstructorContext;
use midnight_compact_runtime::ledger::{ContractAddress, DefaultDB, StateValue, read_counter};
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let mut operations: HashMap<EntryPointBuf, ContractOperation, DefaultDB> = HashMap::new();
    for name in ["bump", "bump_twice", "add", "add_twice"] {
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
fn stateful_calls_execute_twice_and_match_typescript_state() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/stateful-circuit-call.json"
    ))
    .unwrap();
    let context = initial_state(ConstructorContext::new(()))
        .unwrap()
        .into_circuit_context(ContractAddress::default());
    let called = bump_twice(context).unwrap();
    let state = called.context.query.state.get_ref();
    let StateValue::Array(fields) = state else {
        panic!("expected ledger field array")
    };
    assert_eq!(read_counter(&fields.get(0).unwrap()).unwrap(), 2);
    assert_eq!(state_hex(state.clone()), oracle["afterBumpHex"]);
    let added = add_twice(called.context, BoundedUint::<65535>::new(3).unwrap()).unwrap();
    let state = added.context.query.state.get_ref();
    let StateValue::Array(fields) = state else {
        panic!("expected ledger field array")
    };
    assert_eq!(read_counter(&fields.get(0).unwrap()).unwrap(), 8);
    assert_eq!(state_hex(state.clone()), oracle["afterHex"]);
}
