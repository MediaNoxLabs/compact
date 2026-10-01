use compact_rust_bounded_uint_oracle_fixture::ledger_contract::{initial_state, set_small};
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
    operations = operations.insert(
        EntryPointBuf(b"set_small".to_vec()),
        ContractOperation::new(None),
    );
    let contract_state =
        ContractState::new(state, operations, ContractMaintenanceAuthority::default());
    let mut bytes = Vec::new();
    tagged_serialize(&contract_state, &mut bytes).unwrap();
    hex::encode(bytes)
}

#[test]
fn exact_bounded_uint_oracle_matches_typescript_state_and_range() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/bounded-uint-oracle.json"
    ))
    .unwrap();
    let initial = initial_state(ConstructorContext::new(())).unwrap();
    assert_eq!(
        state_hex(initial.ledger_state.get_ref().clone()),
        oracle["afterInit"]
    );
    let set99 = set_small(
        initial.into_circuit_context(ContractAddress::default()),
        runtime::BoundedUint::<99>::new(99).unwrap(),
    )
    .unwrap();
    assert_eq!(
        state_hex(set99.context.query.state.get_ref().clone()),
        oracle["afterSet99"]
    );
    let value = runtime::ledger::read_root_cell::<runtime::BoundedUint<99>, _>(
        set99.context.query.state.get_ref(),
        0,
    )
    .unwrap();
    assert_eq!(value.value().to_string(), oracle["smallAfterSet99"]);
    assert!(runtime::BoundedUint::<99>::new(100).is_err());
    let set0 = set_small(set99.context, runtime::BoundedUint::<99>::new(0).unwrap()).unwrap();
    assert_eq!(
        state_hex(set0.context.query.state.get_ref().clone()),
        oracle["afterSet0"]
    );
}
