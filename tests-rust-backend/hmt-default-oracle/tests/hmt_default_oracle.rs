use compact_rust_hmt_default_oracle_fixture::ledger_contract::{add_default, initial_state};
use midnight_compact_runtime as runtime;
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;
use runtime::context::ConstructorContext;
use runtime::ledger::{ContractAddress, DefaultDB, StateValue};

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let operations: HashMap<EntryPointBuf, ContractOperation, DefaultDB> = HashMap::new();
    let operations = operations.insert(
        EntryPointBuf(b"add_default".to_vec()),
        ContractOperation::new(None),
    );
    let state = ContractState::new(state, operations, ContractMaintenanceAuthority::default());
    let mut bytes = Vec::new();
    tagged_serialize(&state, &mut bytes).unwrap();
    hex::encode(bytes)
}

#[test]
fn historic_merkle_tree_matches_typescript_after_each_insert() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/hmt-default-oracle.json"
    ))
    .unwrap();
    let initial = initial_state(ConstructorContext::new(())).unwrap();
    let state = initial.ledger_state.get_ref();
    assert_eq!(state_hex(state.clone()), oracle["afterInit"]);
    let tree = runtime::ledger::historic_merkle_tree_view_at_path(state, &[0]).unwrap();
    assert_eq!(tree.first_free().unwrap().value(), 0);
    assert!(tree.root().is_some());

    let context = initial.into_circuit_context(ContractAddress::default());
    let after_zero = add_default(context, runtime::BoundedUint::new(0).unwrap()).unwrap();
    let state = after_zero.context.query.state.get_ref();
    assert_eq!(state_hex(state.clone()), oracle["afterAddDefault0"]);
    assert_eq!(
        runtime::ledger::historic_merkle_tree_view_at_path(state, &[0])
            .unwrap()
            .first_free()
            .unwrap()
            .value(),
        1
    );

    let after_two = add_default(after_zero.context, runtime::BoundedUint::new(2).unwrap()).unwrap();
    let state = after_two.context.query.state.get_ref();
    assert_eq!(state_hex(state.clone()), oracle["afterAddDefault2"]);
    assert_eq!(
        runtime::ledger::historic_merkle_tree_view_at_path(state, &[0])
            .unwrap()
            .first_free()
            .unwrap()
            .value(),
        3
    );

    // A lower index follows the VM's swap/pop path and preserves first_free.
    let repeated = add_default(after_two.context, runtime::BoundedUint::new(0).unwrap()).unwrap();
    let state = repeated.context.query.state.get_ref();
    assert_eq!(state_hex(state.clone()), oracle["afterRepeat0"]);
    assert_eq!(
        runtime::ledger::historic_merkle_tree_view_at_path(state, &[0])
            .unwrap()
            .first_free()
            .unwrap()
            .value(),
        3
    );
}
