use compact_rust_inline_type_scope_oracle_fixture::ledger_contract::{
    checkAggScope, checkNoCollisionScope, checkScalarScope, initial_state, setHash,
};
use midnight_compact_runtime::context::{CircuitContext, CircuitResult, ConstructorContext};
use midnight_compact_runtime::ledger::{ContractAddress, DefaultDB, StateValue};
use midnight_compact_runtime::{CompactError, Field, FixedBytes, FixedVector, persistent_hash};
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let mut operations: HashMap<EntryPointBuf, ContractOperation, DefaultDB> = HashMap::new();
    for name in [
        "setHash",
        "checkScalarScope",
        "checkAggScope",
        "checkNoCollisionScope",
    ] {
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

fn vector2(a: u64, b: u64) -> FixedVector<Field, 2> {
    FixedVector::new([Field::from(a), Field::from(b)])
}

fn execute_scenario<F>(hash: FixedBytes<32>, check: F) -> String
where
    F: FnOnce(CircuitContext<()>) -> Result<CircuitResult<(), ()>, CompactError>,
{
    let constructor = initial_state(ConstructorContext::new(())).unwrap();
    let context = constructor.into_circuit_context(ContractAddress::default());
    let seeded = setHash(context, hash).unwrap();
    let checked = check(seeded.context).unwrap();
    state_hex(checked.context.query.state.get_ref().clone())
}

#[test]
fn internal_helper_formal_scopes_match_typescript() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/inline-type-scope-oracle.json"
    ))
    .unwrap();
    let constructor = initial_state(ConstructorContext::new(())).unwrap();
    assert_eq!(
        state_hex(constructor.ledger_state.get_ref().clone()),
        oracle["initialHex"]
    );

    let scalar_hash = persistent_hash(Field::from(5_u64));
    assert_eq!(
        execute_scenario(scalar_hash, |context| checkScalarScope(
            context,
            vector2(9, 10),
            Field::from(5_u64),
        )),
        oracle["scalarStateHex"]
    );
    assert_eq!(
        execute_scenario(
            persistent_hash(FixedVector::new([
                Field::from(1_u64),
                Field::from(2_u64),
                Field::from(3_u64),
                Field::from(4_u64),
            ])),
            |context| checkAggScope(
                context,
                vector2(7, 8),
                FixedVector::new([
                    Field::from(1_u64),
                    Field::from(2_u64),
                    Field::from(3_u64),
                    Field::from(4_u64),
                ]),
            ),
        ),
        oracle["aggStateHex"]
    );
    assert_eq!(
        execute_scenario(persistent_hash(vector2(3, 4)), |context| {
            checkNoCollisionScope(context, vector2(3, 4))
        }),
        oracle["noCollisionStateHex"]
    );

    let context = initial_state(ConstructorContext::new(()))
        .unwrap()
        .into_circuit_context(ContractAddress::default());
    let seeded = setHash(context, scalar_hash).unwrap();
    let error = checkScalarScope(seeded.context, vector2(9, 10), Field::from(6_u64))
        .err()
        .unwrap();
    assert_eq!(error.to_string(), oracle["scalarMismatch"]);
}
