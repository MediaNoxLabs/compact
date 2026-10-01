use compact_rust_witness_vector_action_fixture::ledger_contract::{
    LedgerView, Witnesses, discardResult, initial_state, keepResult, reuseResult,
};
use midnight_compact_runtime::context::{CircuitResult, ConstructorContext, WitnessContext};
use midnight_compact_runtime::ledger::{ContractAddress, DefaultDB, StateValue};
use midnight_compact_runtime::{Field, FixedVector};
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;
use std::cell::Cell;

struct SumWitness {
    calls: Cell<usize>,
}

impl Witnesses<u64> for SumWitness {
    fn sumWitness(
        &self,
        context: WitnessContext<'_, u64, LedgerView<'_>>,
        values: FixedVector<Field, 2>,
    ) -> (u64, Field) {
        self.calls.set(self.calls.get() + 1);
        assert_eq!(*context.private_state, 7);
        assert_eq!(context.ledger.stored().unwrap(), Field::from(0_u64));
        assert_eq!(
            values,
            FixedVector::new([Field::from(0_u64), Field::from(1_u64)])
        );
        (8, Field::from(8_u64))
    }
}

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let mut operations: HashMap<EntryPointBuf, ContractOperation, DefaultDB> = HashMap::new();
    for name in ["keepResult", "discardResult", "reuseResult"] {
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

fn assert_oracle(result: CircuitResult<u64, ()>, calls: usize, oracle: &serde_json::Value) {
    assert_eq!(calls, oracle["calls"].as_u64().unwrap() as usize);
    assert_eq!(result.context.private_state, oracle["privateState"]);
    assert_eq!(
        state_hex(result.context.query.state.get_ref().clone()),
        oracle["stateHex"]
    );
    let expected = oracle["privateTranscriptOutputs"].as_array().unwrap();
    assert_eq!(result.private_transcript_outputs.len(), expected.len());
    for (output, expected) in result.private_transcript_outputs.iter().zip(expected) {
        let atoms = output
            .value
            .0
            .iter()
            .map(|atom| &atom.0)
            .collect::<Vec<_>>();
        assert_eq!(serde_json::to_value(atoms).unwrap(), expected["valueAtoms"]);
        assert_eq!(
            serde_json::to_value(&output.alignment).unwrap(),
            expected["alignment"]
        );
    }
}

#[test]
fn witness_vector_actions_evaluate_once_and_preserve_transcript() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/witness-vector-action.json"
    ))
    .unwrap();

    let keep_witness = SumWitness {
        calls: Cell::new(0),
    };
    let keep_context = initial_state(ConstructorContext::new(7_u64))
        .unwrap()
        .into_circuit_context(ContractAddress::default());
    let kept = keepResult(keep_context, &keep_witness).unwrap();
    assert_oracle(kept, keep_witness.calls.get(), &oracle["keepResult"]);

    let discard_witness = SumWitness {
        calls: Cell::new(0),
    };
    let discard_context = initial_state(ConstructorContext::new(7_u64))
        .unwrap()
        .into_circuit_context(ContractAddress::default());
    let discarded = discardResult(discard_context, &discard_witness).unwrap();
    assert_oracle(
        discarded,
        discard_witness.calls.get(),
        &oracle["discardResult"],
    );

    let reuse_witness = SumWitness {
        calls: Cell::new(0),
    };
    let reuse_context = initial_state(ConstructorContext::new(7_u64))
        .unwrap()
        .into_circuit_context(ContractAddress::default());
    let reused = reuseResult(reuse_context, &reuse_witness).unwrap();
    assert_oracle(reused, reuse_witness.calls.get(), &oracle["reuseResult"]);
}
