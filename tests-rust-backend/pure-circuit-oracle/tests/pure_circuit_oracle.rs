use compact_rust_pure_circuit_oracle_fixture::ledger_contract::{initial_state, ping};
use compact_rust_pure_circuit_oracle_fixture::pure_circuits::{and_b, which_u32};
use midnight_compact_runtime as runtime;
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;
use runtime::context::ConstructorContext;
use runtime::ledger::{ContractAddress, DefaultDB, StateValue};

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
fn exact_pure_circuit_oracle_matches_typescript_results_and_state() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/pure-circuit-oracle.json"
    ))
    .unwrap();
    for (index, (a, b)) in [(false, false), (false, true), (true, false), (true, true)]
        .into_iter()
        .enumerate()
    {
        assert_eq!(and_b(a, b).unwrap(), oracle["and"][index]);
    }
    for (index, b) in [false, true].into_iter().enumerate() {
        assert_eq!(
            which_u32(b).unwrap().value().to_string(),
            oracle["whichU32"][index]
        );
    }
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
