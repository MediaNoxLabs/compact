use compact_rust_uints_oracle_fixture::ledger_contract::{initial_state, set_byte};
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
        EntryPointBuf(b"set_byte".to_vec()),
        ContractOperation::new(None),
    );
    let contract_state =
        ContractState::new(state, operations, ContractMaintenanceAuthority::default());
    let mut bytes = Vec::new();
    tagged_serialize(&contract_state, &mut bytes).unwrap();
    hex::encode(bytes)
}

#[test]
fn exact_uints_oracle_matches_typescript_state_and_width() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/uints-oracle.json"
    ))
    .unwrap();
    let initial = initial_state(ConstructorContext::new(())).unwrap();
    assert_eq!(
        state_hex(initial.ledger_state.get_ref().clone()),
        oracle["afterInit"]
    );
    let set255 = set_byte(
        initial.into_circuit_context(ContractAddress::default()),
        runtime::BoundedUint::<255>::new(255).unwrap(),
    )
    .unwrap();
    assert_eq!(
        state_hex(set255.context.query.state.get_ref().clone()),
        oracle["afterSet255"]
    );
    let value = runtime::ledger::read_root_cell::<runtime::BoundedUint<255>, _>(
        set255.context.query.state.get_ref(),
        0,
    )
    .unwrap();
    assert_eq!(value.value().to_string(), oracle["byteAfterSet255"]);
    assert!(runtime::BoundedUint::<255>::new(256).is_err());
    let set0 = set_byte(set255.context, runtime::BoundedUint::<255>::new(0).unwrap()).unwrap();
    assert_eq!(
        state_hex(set0.context.query.state.get_ref().clone()),
        oracle["afterSet0"]
    );
}
