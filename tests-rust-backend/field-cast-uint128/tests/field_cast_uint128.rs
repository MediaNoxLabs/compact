use compact_rust_field_cast_uint128_fixture::ledger_contract::{initial_state, save};
use compact_rust_field_cast_uint128_fixture::pure_circuits::as_field;
use midnight_compact_runtime::context::ConstructorContext;
use midnight_compact_runtime::ledger::{ContractAddress, DefaultDB, StateValue};
use midnight_compact_runtime::{BoundedUint, Field};
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
fn uint128_to_field_cast_preserves_high_bits_and_matches_typescript() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/field-cast-uint128.json"
    ))
    .unwrap();
    let value = oracle["input"].as_str().unwrap().parse::<u128>().unwrap();
    let input = BoundedUint::<{ u128::MAX }>::new(value).unwrap();
    let expected = Field::from(value);
    assert_eq!(as_field(input).unwrap(), expected);
    assert_eq!(oracle["pure"], value.to_string());
    let context = initial_state(ConstructorContext::new(()))
        .unwrap()
        .into_circuit_context(ContractAddress::default());
    let saved = save(context, input).unwrap();
    assert_eq!(saved.result, expected);
    assert_eq!(oracle["returned"], value.to_string());
    assert_eq!(
        state_hex(saved.context.query.state.get_ref().clone()),
        oracle["afterHex"]
    );
}
