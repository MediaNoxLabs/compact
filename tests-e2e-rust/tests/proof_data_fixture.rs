// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

use compact_contract_proof_data_fixture::{Contract, Witnesses};
use midnight_compact_runtime::*;
use serde::Deserialize;

struct ProofWitnesses;

impl Witnesses<()> for ProofWitnesses {
    fn first_secret<'a>(
        &self,
        _ctx: &WitnessContext<compact_contract_proof_data_fixture::Ledger<'a>, ()>,
    ) -> ((), Fr) {
        ((), Fr::from(5u64))
    }

    fn second_secret<'a>(
        &self,
        _ctx: &WitnessContext<compact_contract_proof_data_fixture::Ledger<'a>, ()>,
    ) -> ((), Fr) {
        ((), Fr::from(7u64))
    }
}

#[derive(Deserialize)]
struct TsProofDataFixture {
    #[serde(rename = "publicTranscriptTags")]
    public_transcript_tags: Vec<String>,
    #[serde(rename = "privateTranscriptOutputs")]
    private_transcript_outputs: Vec<TsAlignedSummary>,
    input: TsAlignedSummary,
    output: TsAlignedSummary,
    #[serde(rename = "popeqValues")]
    popeq_values: Vec<TsAlignedSummary>,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
struct TsAlignedSummary {
    #[serde(rename = "valueAtoms")]
    value_atoms: Vec<String>,
    alignment: Vec<String>,
}

fn ts_fixture() -> TsProofDataFixture {
    serde_json::from_str(include_str!("../fixtures/proof-data-fixture-ts-state.json"))
        .expect("proof-data fixture JSON")
}

fn ctor_ctx() -> ConstructorContext<()> {
    ConstructorContext {
        initial_private_state: (),
        empty_zswap_local_state: ZswapLocalState::default(),
        cost_model: INITIAL_COST_MODEL.clone(),
        gas_limit: None,
    }
}

fn rust_summary(value: &AlignedValue) -> TsAlignedSummary {
    TsAlignedSummary {
        value_atoms: value
            .value
            .0
            .iter()
            .map(|atom| hex::encode(&atom.0))
            .collect(),
        alignment: value
            .alignment
            .0
            .iter()
            .map(|segment| serde_json::to_string(segment).expect("alignment JSON"))
            .collect(),
    }
}

fn op_tag(op: &Op<ResultModeVerify, DefaultDB>) -> &'static str {
    match op {
        Op::Dup { .. } => "dup",
        Op::Idx { .. } => "idx",
        Op::Popeq { .. } => "popeq",
        Op::Push { .. } => "push",
        Op::Ins { .. } => "ins",
        _ => "other",
    }
}

#[test]
fn generated_proof_data_matches_ts_reference_shape() {
    let ts = ts_fixture();
    let contract: Contract<(), ProofWitnesses> = Contract::new(ProofWitnesses);
    let init = contract
        .initial_state(ctor_ctx(), Fr::from(11u64))
        .expect("initial_state");
    let read_qctx = QueryContext::new(
        init.current_contract_state.clone(),
        midnight_compact_runtime::ContractAddress::default(),
    );
    let read_ops = OpProgramGather::<DefaultDB>::new()
        .dup(0)
        .idx_at_index(0u8, false)
        .popeq(true)
        .build();
    let direct_read = query_for_read(&read_qctx, &read_ops, None, &initial_cost_model()).unwrap();
    let mut recorded_read_proof = PartialProofData::<DefaultDB>::new(aligned_value_from_parts(&[]));
    let recorded_read = recorded_query_for_read(
        &mut recorded_read_proof,
        &read_qctx,
        &read_ops,
        None,
        &initial_cost_model(),
    )
    .unwrap();
    assert_eq!(recorded_read.gas_cost, direct_read.gas_cost);
    assert_eq!(recorded_read.events.len(), direct_read.events.len());

    let ctx = CircuitContext::new(init.current_contract_state, ());
    let result = contract
        .read_witness_write(ctx, Fr::from(11u64), Fr::from(30u64))
        .expect("read_witness_write");

    let call = result
        .context
        .call_proof_data_trace
        .as_slice()
        .last()
        .expect("call proof data");
    assert_eq!(call.circuit_id, "read_witness_write");
    assert_eq!(ts.input.value_atoms, ["0b", "1e"]);
    assert_eq!(ts.output.value_atoms, Vec::<String>::new());
    let expected_input = aligned_value_from_parts(&[
        proof_aligned_value(&Fr::from(11u64)),
        proof_aligned_value(&Fr::from(30u64)),
    ]);
    let expected_output = aligned_value_from_parts(&[]);
    assert_eq!(call.proof_data.input, expected_input);
    assert_eq!(call.proof_data.output, expected_output);
    assert_eq!(rust_summary(&call.proof_data.input), ts.input);
    assert_eq!(rust_summary(&call.proof_data.output), ts.output);
    assert_eq!(
        call.proof_data.input.value.0.len(),
        ts.input.value_atoms.len()
    );
    assert_eq!(
        call.proof_data.input.alignment.0.len(),
        ts.input.alignment.len()
    );
    assert_eq!(
        call.proof_data.output.value.0.len(),
        ts.output.value_atoms.len()
    );
    assert_eq!(
        call.proof_data.output.alignment.0.len(),
        ts.output.alignment.len()
    );

    let tags: Vec<_> = call
        .proof_data
        .public_transcript
        .iter()
        .map(op_tag)
        .collect();
    assert_eq!(tags, ts.public_transcript_tags);
    assert_eq!(tags, ["dup", "idx", "popeq", "push", "push", "ins"]);

    let popeq_values: Vec<_> = call
        .proof_data
        .public_transcript
        .iter()
        .filter_map(|op| match op {
            Op::Popeq { result, .. } => Some(result),
            _ => None,
        })
        .collect();
    assert_eq!(popeq_values.len(), ts.popeq_values.len());
    assert_eq!(
        std_lib::decode_fr(popeq_values[0]).unwrap(),
        Fr::from(11u64)
    );

    let private = call.proof_data.private_transcript_outputs().as_slice();
    assert_eq!(private.len(), ts.private_transcript_outputs.len());
    assert_eq!(ts.private_transcript_outputs[0].value_atoms, ["05"]);
    assert_eq!(ts.private_transcript_outputs[1].value_atoms, ["07"]);
    assert_eq!(private[0], proof_aligned_value(&Fr::from(5u64)));
    assert_eq!(private[1], proof_aligned_value(&Fr::from(7u64)));
    assert_eq!(rust_summary(&private[0]), ts.private_transcript_outputs[0]);
    assert_eq!(rust_summary(&private[1]), ts.private_transcript_outputs[1]);
    assert_eq!(std_lib::decode_fr(&private[0]).unwrap(), Fr::from(5u64));
    assert_eq!(std_lib::decode_fr(&private[1]).unwrap(), Fr::from(7u64));
}
