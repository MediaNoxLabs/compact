use compact_rust_tuple_oracle_fixture::ledger_contract::initial_state;
use compact_rust_tuple_oracle_fixture::pure_circuits::{
    as_tuple, as_vector, empty_tuple, hetero, one_tuple, struct_vector_return,
    struct_vector_to_tuple, tuple_coerce, tuple_first, tuple_second, tuple_to_vector,
    tuple_var_ref,
};
use compact_rust_tuple_oracle_fixture::types::Pair;
use midnight_compact_runtime::context::ConstructorContext;
use midnight_compact_runtime::ledger::{DefaultDB, StateValue};
use midnight_compact_runtime::{BoundedUint, Field, FixedVector};
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let operations: HashMap<EntryPointBuf, ContractOperation, DefaultDB> = HashMap::new();
    let operations = operations.insert(
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
fn tuple_shapes_and_coercions_match_typescript() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/tuple-oracle.json"
    ))
    .unwrap();
    let constructor = initial_state(ConstructorContext::new(())).unwrap();
    assert_eq!(
        state_hex(constructor.ledger_state.get_ref().clone()),
        oracle["initialHex"]
    );

    let field = Field::from(7_u64);
    let small = BoundedUint::<255>::new(255).unwrap();
    let pair = (field, field);
    assert_eq!(as_vector(field).unwrap(), FixedVector::new([field; 2]));
    assert_eq!(as_tuple(field).unwrap(), pair);
    assert_eq!(
        hetero(small).unwrap(),
        (
            Field::from(255_u64),
            BoundedUint::<65535>::new(255).unwrap()
        )
    );
    assert_eq!(one_tuple(field).unwrap(), (field,));
    assert_eq!(empty_tuple().unwrap(), ());
    assert_eq!(
        tuple_coerce(small).unwrap(),
        (Field::from(255_u64), Field::from(255_u64))
    );
    assert_eq!(
        tuple_var_ref(small).unwrap(),
        (Field::from(255_u64), Field::from(255_u64))
    );
    assert_eq!(
        tuple_to_vector(field).unwrap(),
        FixedVector::new([field; 2])
    );

    let expected_structs = (
        Pair {
            a: field,
            b: BoundedUint::<255>::new(1).unwrap(),
        },
        Pair {
            a: field,
            b: BoundedUint::<255>::new(2).unwrap(),
        },
    );
    assert_eq!(struct_vector_return(field).unwrap(), expected_structs);
    assert_eq!(struct_vector_to_tuple(field).unwrap(), expected_structs);
    assert_eq!(tuple_first(field, true).unwrap(), field);
    assert!(tuple_second(field, true).unwrap());
    assert_eq!(oracle["tupleFirst"], "7");
    assert_eq!(oracle["tupleSecond"], true);

    for key in ["asVector", "asTuple", "tupleToVector"] {
        assert_eq!(oracle[key], serde_json::json!(["7", "7"]));
    }
    for key in ["tupleCoerce", "tupleVarRef"] {
        assert_eq!(oracle[key], serde_json::json!(["255", "255"]));
    }
    assert_eq!(oracle["hetero"], serde_json::json!(["255", "255"]));
    assert_eq!(oracle["oneTuple"], serde_json::json!(["7"]));
    assert_eq!(oracle["emptyTuple"], serde_json::json!([]));
    for key in ["structVectorReturn", "structVectorToTuple"] {
        assert_eq!(
            oracle[key],
            serde_json::json!([
                { "a": "7", "b": "1" }, { "a": "7", "b": "2" }
            ])
        );
    }
}
