use compact_rust_module_boolean_constructor_fixture::ledger_contract::{bump_inner, initial_state};
use midnight_compact_runtime::context::ConstructorContext;
use midnight_compact_runtime::ledger::{ContractAddress, DefaultDB, StateValue, read_counter};
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let operations: HashMap<EntryPointBuf, ContractOperation, DefaultDB> = HashMap::new().insert(
        EntryPointBuf(b"bump_inner".to_vec()),
        ContractOperation::new(None),
    );
    let contract_state =
        ContractState::new(state, operations, ContractMaintenanceAuthority::default());
    let mut bytes = Vec::new();
    tagged_serialize(&contract_state, &mut bytes).unwrap();
    hex::encode(bytes)
}

#[test]
fn module_constructor_and_counter_match_typescript_state() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/module-boolean-constructor.json"
    ))
    .unwrap();
    let constructor = initial_state(ConstructorContext::new(()));
    assert_eq!(
        state_hex(constructor.ledger_state.get_ref().clone()),
        oracle["initialHex"]
    );
    let context = constructor.into_circuit_context(ContractAddress::default());
    let bumped = bump_inner(context).unwrap();
    let state = bumped.context.query.state.get_ref();
    let StateValue::Array(fields) = state else {
        panic!("expected ledger field array")
    };
    assert_eq!(read_counter(&fields.get(0).unwrap()).unwrap(), 1);
    assert_eq!(state_hex(state.clone()), oracle["afterHex"]);
}
