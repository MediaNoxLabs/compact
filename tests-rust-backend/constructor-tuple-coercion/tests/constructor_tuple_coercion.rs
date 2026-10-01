use compact_rust_constructor_tuple_coercion_fixture::ledger_contract::{initial_state, read_pair};
use compact_rust_constructor_tuple_coercion_fixture::pure_circuits::constant;
use midnight_compact_runtime::Field;
use midnight_compact_runtime::context::ConstructorContext;
use midnight_compact_runtime::ledger::{ContractAddress, DefaultDB, StateValue};
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let operations: HashMap<EntryPointBuf, ContractOperation, DefaultDB> = HashMap::new();
    let operations = operations.insert(
        EntryPointBuf(b"read_pair".to_vec()),
        ContractOperation::new(None),
    );
    let contract_state =
        ContractState::new(state, operations, ContractMaintenanceAuthority::default());
    let mut bytes = Vec::new();
    tagged_serialize(&contract_state, &mut bytes).unwrap();
    hex::encode(bytes)
}

#[test]
fn untyped_tuple_literals_coerce_to_field_elements() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/constructor-tuple-coercion.json"
    ))
    .unwrap();
    let constructor = initial_state(ConstructorContext::new(())).unwrap();
    assert_eq!(
        state_hex(constructor.ledger_state.get_ref().clone()),
        oracle["initialHex"]
    );
    let read = read_pair(constructor.into_circuit_context(ContractAddress::default())).unwrap();
    assert_eq!(read.result, (Field::from(3_u64), Field::from(4_u64)));
    assert_eq!(read.result, constant().unwrap());
    assert_eq!(oracle["constant"], serde_json::json!(["3", "4"]));
    assert_eq!(oracle["read"], oracle["constant"]);
}
