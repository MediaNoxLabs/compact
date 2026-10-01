use compact_rust_opaque_string_map_query_oracle_fixture::ledger_contract::{
    ensure, initial_state, put,
};
use midnight_compact_runtime::context::ConstructorContext;
use midnight_compact_runtime::ledger::{ContractAddress, DefaultDB, StateValue};
use midnight_compact_runtime::{Field, OpaqueString};
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let mut operations: HashMap<EntryPointBuf, ContractOperation, DefaultDB> = HashMap::new();
    for name in ["ensure", "put"] {
        operations = operations.insert(
            EntryPointBuf(name.as_bytes().to_vec()),
            ContractOperation::new(None),
        );
    }
    let state = ContractState::new(state, operations, ContractMaintenanceAuthority::default());
    let mut bytes = Vec::new();
    tagged_serialize(&state, &mut bytes).unwrap();
    hex::encode(bytes)
}

#[test]
fn nested_map_queries_match_typescript_with_string_key() {
    let reference: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/opaque-string-map-query-oracle.json"
    ))
    .unwrap();
    let initial = initial_state(ConstructorContext::new(7_u64)).unwrap();
    assert_eq!(
        state_hex(initial.ledger_state.get_ref().clone()),
        reference["initialHex"]
    );
    let context = initial.into_circuit_context(ContractAddress::default());
    let key = OpaqueString::from("asset-1");
    let error = ensure(context, key.clone()).err().unwrap();
    assert_eq!(
        error.to_string(),
        reference["beforeError"].as_str().unwrap()
    );
    let context = initial_state(ConstructorContext::new(7_u64))
        .unwrap()
        .into_circuit_context(ContractAddress::default());
    let written = put(context, key.clone(), Field::from(42_u64)).unwrap();
    assert_eq!(
        state_hex(written.context.query.state.get_ref().clone()),
        reference["afterPutHex"]
    );
    let value = ensure(written.context, key).unwrap();
    assert_eq!(value.result, Field::from(42_u64));
    assert_eq!(reference["result"], "42");
}
