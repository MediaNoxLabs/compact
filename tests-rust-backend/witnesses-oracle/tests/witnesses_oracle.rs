use compact_rust_witnesses_oracle_fixture::ledger_contract::{
    LedgerView, Witnesses, initial_state, pull,
};
use midnight_compact_runtime as runtime;
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;
use runtime::context::{ConstructorContext, WitnessContext};
use runtime::ledger::{ContractAddress, DefaultDB, StateValue};

struct OracleWitness;

impl Witnesses<u64> for OracleWitness {
    fn fetch_field(
        &self,
        context: WitnessContext<'_, u64, LedgerView<'_>>,
    ) -> (u64, runtime::Field) {
        assert_eq!(*context.private_state, 7);
        assert_eq!(*context.contract_address, ContractAddress::default());
        assert_eq!(context.ledger.v().unwrap(), runtime::Field::default());
        (8, runtime::Field::from(42_u64))
    }
}

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let operations: HashMap<EntryPointBuf, ContractOperation, DefaultDB> = HashMap::new().insert(
        EntryPointBuf(b"pull".to_vec()),
        ContractOperation::new(None),
    );
    let contract_state =
        ContractState::new(state, operations, ContractMaintenanceAuthority::default());
    let mut bytes = Vec::new();
    tagged_serialize(&contract_state, &mut bytes).unwrap();
    hex::encode(bytes)
}

#[test]
fn exact_witness_oracle_matches_state_private_state_and_transcript() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/witnesses-oracle.json"
    ))
    .unwrap();
    let initial = initial_state(ConstructorContext::new(7_u64)).unwrap();
    assert_eq!(
        state_hex(initial.ledger_state.get_ref().clone()),
        oracle["afterInit"]
    );
    let after = pull(
        initial.into_circuit_context(ContractAddress::default()),
        &OracleWitness,
    )
    .unwrap();
    assert_eq!(
        state_hex(after.context.query.state.get_ref().clone()),
        oracle["afterPull"]
    );
    let value = runtime::ledger::read_root_cell::<runtime::Field, _>(
        after.context.query.state.get_ref(),
        0,
    )
    .unwrap();
    let oracle_value = oracle["valueAfterPull"]
        .as_str()
        .unwrap()
        .parse::<u64>()
        .unwrap();
    assert_eq!(value, runtime::Field::from(oracle_value));
    assert_eq!(after.context.private_state, oracle["privateStateAfterPull"]);
    assert_eq!(after.private_transcript_outputs.len(), 1);
    let transcript = &after.private_transcript_outputs[0];
    let atoms = transcript
        .value
        .0
        .iter()
        .map(|atom| &atom.0)
        .collect::<Vec<_>>();
    assert_eq!(
        serde_json::to_value(atoms).unwrap(),
        oracle["privateTranscriptOutputs"][0]["valueAtoms"]
    );
    assert_eq!(
        serde_json::to_value(&transcript.alignment).unwrap(),
        oracle["privateTranscriptOutputs"][0]["alignment"]
    );
}
