use compact_rust_cross_circuit_oracle_fixture::ledger_contract::{
    initial_state, reset, reset_and_set,
};
use midnight_compact_runtime as runtime;
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;
use runtime::context::ConstructorContext;
use runtime::ledger::{ContractAddress, DefaultDB, StateValue};

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let mut operations: HashMap<EntryPointBuf, ContractOperation, DefaultDB> = HashMap::new();
    for name in ["reset", "reset_and_set"] {
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

fn value(state: &StateValue<DefaultDB>) -> u128 {
    runtime::ledger::read_root_cell::<runtime::BoundedUint<{ u64::MAX as u128 }>, _>(state, 0)
        .unwrap()
        .value()
}

#[test]
fn exported_stateful_circuit_call_matches_typescript_state() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/cross-circuit-oracle.json"
    ))
    .unwrap();
    let initial = initial_state(ConstructorContext::new(())).unwrap();
    let state = initial.ledger_state.get_ref();
    assert_eq!(state_hex(state.clone()), oracle["afterInit"]);
    assert_eq!(value(state).to_string(), oracle["initialValue"]);
    let context = initial.into_circuit_context(ContractAddress::default());
    let set7 = reset_and_set(context, runtime::BoundedUint::new(7).unwrap()).unwrap();
    assert_eq!(
        state_hex(set7.context.query.state.get_ref().clone()),
        oracle["afterSet7"]
    );
    assert_eq!(value(set7.context.query.state.get_ref()), 7);
    let set13 = reset_and_set(set7.context, runtime::BoundedUint::new(13).unwrap()).unwrap();
    assert_eq!(
        state_hex(set13.context.query.state.get_ref().clone()),
        oracle["afterSet13"]
    );
    assert_eq!(value(set13.context.query.state.get_ref()), 13);
    let cleared = reset(set13.context).unwrap();
    assert_eq!(
        state_hex(cleared.context.query.state.get_ref().clone()),
        oracle["afterReset"]
    );
    assert_eq!(value(cleared.context.query.state.get_ref()), 0);
}
