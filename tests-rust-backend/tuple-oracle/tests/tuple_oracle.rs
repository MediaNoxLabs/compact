use compact_rust_tuple_oracle_fixture::ledger_contract::{initial_state, ping};
use compact_rust_tuple_oracle_fixture::pure_circuits::{
    as_tuple, as_vector, empty_tuple, hetero, one_tuple, struct_vector_return,
    struct_vector_to_tuple, tuple_coerce, tuple_to_vector, tuple_var_ref,
};
use compact_rust_tuple_oracle_fixture::types::Pair;
use midnight_compact_runtime as runtime;
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;
use runtime::context::ConstructorContext;
use runtime::ledger::{ContractAddress, DefaultDB, StateValue};

fn field(value: &serde_json::Value) -> runtime::Field {
    runtime::Field::from(value.as_str().unwrap().parse::<u64>().unwrap())
}

fn fields(value: &serde_json::Value) -> Vec<runtime::Field> {
    value.as_array().unwrap().iter().map(field).collect()
}

fn assert_pairs(actual: (Pair, Pair), expected: &serde_json::Value) {
    for (pair, oracle) in [actual.0, actual.1]
        .into_iter()
        .zip(expected.as_array().unwrap())
    {
        assert_eq!(pair.a, field(&oracle["a"]));
        assert_eq!(pair.b.value().to_string(), oracle["b"]);
    }
}

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let operations: HashMap<EntryPointBuf, ContractOperation, DefaultDB> = HashMap::new().insert(
        EntryPointBuf(b"ping".to_vec()),
        ContractOperation::new(None),
    );
    let contract_state =
        ContractState::new(state, operations, ContractMaintenanceAuthority::default());
    let mut bytes = Vec::new();
    tagged_serialize(&contract_state, &mut bytes).unwrap();
    hex::encode(bytes)
}

#[test]
fn exact_tuple_oracle_matches_typescript_values_and_state() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/tuple-oracle.json"
    ))
    .unwrap();
    let values = &oracle["results"];
    let seven = runtime::Field::from(7_u64);
    let nine = runtime::BoundedUint::<255>::new(9).unwrap();
    let eleven = runtime::Field::from(11_u64);
    assert_eq!(
        as_vector(seven).unwrap().0.to_vec(),
        fields(&values["asVector"])
    );
    let pair = as_tuple(seven).unwrap();
    assert_eq!(vec![pair.0, pair.1], fields(&values["asTuple"]));
    let pair = hetero(nine).unwrap();
    assert_eq!(pair.0, field(&values["hetero"][0]));
    assert_eq!(pair.1.value().to_string(), values["hetero"][1]);
    assert_eq!(
        vec![one_tuple(seven).unwrap().0],
        fields(&values["oneTuple"])
    );
    assert_eq!(empty_tuple().unwrap(), ());
    assert!(values["emptyTuple"].as_array().unwrap().is_empty());
    let pair = tuple_coerce(nine).unwrap();
    assert_eq!(vec![pair.0, pair.1], fields(&values["tupleCoerce"]));
    let pair = tuple_var_ref(nine).unwrap();
    assert_eq!(vec![pair.0, pair.1], fields(&values["tupleVarRef"]));
    assert_eq!(
        tuple_to_vector(seven).unwrap().0.to_vec(),
        fields(&values["tupleToVector"])
    );
    assert_pairs(
        struct_vector_return(eleven).unwrap(),
        &values["structVectorReturn"],
    );
    assert_pairs(
        struct_vector_to_tuple(eleven).unwrap(),
        &values["structVectorToTuple"],
    );

    let initial = initial_state(ConstructorContext::new(())).unwrap();
    assert_eq!(
        state_hex(initial.ledger_state.get_ref().clone()),
        oracle["afterInit"]
    );
    let after = ping(initial.into_circuit_context(ContractAddress::default())).unwrap();
    assert_eq!(
        state_hex(after.context.query.state.get_ref().clone()),
        oracle["afterPing"]
    );
    let flag =
        runtime::ledger::read_root_cell::<bool, _>(after.context.query.state.get_ref(), 0).unwrap();
    assert_eq!(flag, oracle["flagAfterPing"]);
}
