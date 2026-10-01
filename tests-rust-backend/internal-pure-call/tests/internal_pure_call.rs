use compact_rust_internal_pure_call_fixture::ledger_contract::{initial_state, read_stored, save};
use compact_rust_internal_pure_call_fixture::pure_circuits::double_increment;
use midnight_compact_runtime::Field;
use midnight_compact_runtime::context::ConstructorContext;
use midnight_compact_runtime::ledger::{ContractAddress, DefaultDB, StateValue};
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let mut operations: HashMap<EntryPointBuf, ContractOperation, DefaultDB> = HashMap::new();
    for name in ["save", "read_stored"] {
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
fn local_pure_helper_is_callable_without_becoming_an_export() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/internal-pure-call.json"
    ))
    .unwrap();
    assert_eq!(
        double_increment(Field::from(7_u64)).unwrap(),
        Field::from(9_u64)
    );
    assert_eq!(oracle["exportedPure"], "9");
    let context =
        initial_state(ConstructorContext::new(())).into_circuit_context(ContractAddress::default());
    let write = save(context, Field::from(7_u64)).unwrap();
    let read = read_stored(write.context).unwrap();
    assert_eq!(read.result, Field::from(8_u64));
    assert_eq!(oracle["stored"], "8");
    assert_eq!(
        state_hex(read.context.query.state.get_ref().clone()),
        oracle["stateHex"]
    );
}
