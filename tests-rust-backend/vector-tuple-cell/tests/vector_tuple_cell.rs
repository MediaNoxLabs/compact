use compact_rust_vector_tuple_cell_fixture::ledger_contract::{
    initial_state, read_pair, read_values, set_pair, set_values,
};
use midnight_compact_runtime::context::ConstructorContext;
use midnight_compact_runtime::ledger::{ContractAddress, DefaultDB, StateValue};
use midnight_compact_runtime::{Field, FixedVector};
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let mut operations: HashMap<EntryPointBuf, ContractOperation, DefaultDB> = HashMap::new();
    for name in ["set_values", "read_values", "set_pair", "read_pair"] {
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
fn vector_and_tuple_cells_match_typescript_state_and_reads() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/vector-tuple-cell.json"
    ))
    .unwrap();
    let constructor = initial_state(ConstructorContext::new(())).unwrap();
    assert_eq!(
        state_hex(constructor.ledger_state.get_ref().clone()),
        oracle["initialHex"]
    );
    let context = constructor.into_circuit_context(ContractAddress::default());
    let before_vector = read_values(context).unwrap();
    assert_eq!(
        before_vector.result,
        FixedVector::new([Field::from(0_u64); 3])
    );
    let before_pair = read_pair(before_vector.context).unwrap();
    assert_eq!(before_pair.result, (Field::from(0_u64), false));

    let vector = FixedVector::new([Field::from(3_u64), Field::from(5_u64), Field::from(8_u64)]);
    let write = set_values(before_pair.context, vector.clone()).unwrap();
    assert_eq!(
        state_hex(write.context.query.state.get_ref().clone()),
        oracle["afterVectorHex"]
    );
    let after_vector = read_values(write.context).unwrap();
    assert_eq!(after_vector.result, vector);

    let write = set_pair(after_vector.context, (Field::from(42_u64), true)).unwrap();
    assert_eq!(
        state_hex(write.context.query.state.get_ref().clone()),
        oracle["afterPairHex"]
    );
    let after_pair = read_pair(write.context).unwrap();
    assert_eq!(after_pair.result, (Field::from(42_u64), true));
}
