use compact_rust_aliases_oracle_fixture::ledger_contract::initial_state;
use midnight_compact_runtime::context::ConstructorContext;
use midnight_compact_runtime::ledger::{DefaultDB, StateValue};
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;

#[test]
fn exact_aliases_oracle_initial_state_matches_typescript() {
    let _: compact_rust_aliases_oracle_fixture::Tag = Default::default();
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/aliases-oracle.json"
    ))
    .unwrap();
    assert!(oracle["operations"].as_array().unwrap().is_empty());
    let initial = initial_state(ConstructorContext::new(())).unwrap();
    let state: StateValue<DefaultDB> = initial.ledger_state.get_ref().clone();
    let operations: HashMap<EntryPointBuf, ContractOperation, DefaultDB> = HashMap::new();
    let contract_state =
        ContractState::new(state, operations, ContractMaintenanceAuthority::default());
    let mut bytes = Vec::new();
    tagged_serialize(&contract_state, &mut bytes).unwrap();
    assert_eq!(hex::encode(bytes), oracle["afterInit"]);
}
