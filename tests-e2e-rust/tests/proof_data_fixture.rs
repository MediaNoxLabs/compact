// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//  	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use compact_contract_proof_data_fixture::{ledger, Contract, Witnesses};
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
    constructor: TsConstructorFixture,
    circuit: TsCircuitFixture,
    #[serde(rename = "nestedCircuit")]
    nested_circuit: TsCircuitFixture,
}

#[derive(Deserialize)]
struct TsConstructorFixture {
    #[serde(rename = "constructorId")]
    constructor_id: String,
    #[serde(rename = "publicTranscriptTags")]
    public_transcript_tags: Vec<String>,
    #[serde(rename = "privateTranscriptOutputs")]
    private_transcript_outputs: Vec<TsAlignedSummary>,
    input: TsAlignedSummary,
    output: TsAlignedSummary,
}

#[derive(Deserialize)]
struct TsCircuitFixture {
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

    let ctor = &init.constructor_proof_data;
    assert_eq!(ctor.constructor_id, ts.constructor.constructor_id);
    assert_eq!(ctor.constructor_id, "constructor");
    assert_eq!(ctor.initial_query_context.address, ctor.contract_address);
    assert_eq!(ctor.final_query_context.state, init.current_contract_state);
    assert_eq!(ts.constructor.input.value_atoms, ["0b"]);
    assert_eq!(ts.constructor.output.value_atoms, Vec::<String>::new());
    assert_eq!(ctor.proof_data.input, proof_aligned_value(&Fr::from(11u64)));
    assert_eq!(ctor.proof_data.output, aligned_value_from_parts(&[]));
    assert_eq!(rust_summary(&ctor.proof_data.input), ts.constructor.input);
    assert_eq!(rust_summary(&ctor.proof_data.output), ts.constructor.output);
    let ctor_tags: Vec<_> = ctor
        .proof_data
        .public_transcript
        .iter()
        .map(op_tag)
        .collect();
    assert_eq!(ctor_tags, ts.constructor.public_transcript_tags);
    assert_eq!(ctor_tags, ["push", "push", "ins"]);
    let ctor_private = ctor.proof_data.private_transcript_outputs().as_slice();
    assert_eq!(
        ctor_private.len(),
        ts.constructor.private_transcript_outputs.len()
    );
    assert_eq!(ctor_private[0], proof_aligned_value(&Fr::from(5u64)));
    assert_eq!(
        rust_summary(&ctor_private[0]),
        ts.constructor.private_transcript_outputs[0]
    );

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
        .read_witness_write(ctx, Fr::from(16u64), Fr::from(30u64))
        .expect("read_witness_write");

    let call = result
        .context
        .call_proof_data_trace
        .as_slice()
        .last()
        .expect("call proof data");
    assert_eq!(call.circuit_id, "read_witness_write");
    assert_eq!(ts.circuit.input.value_atoms, ["10", "1e"]);
    assert_eq!(ts.circuit.output.value_atoms, Vec::<String>::new());
    let expected_input = aligned_value_from_parts(&[
        proof_aligned_value(&Fr::from(16u64)),
        proof_aligned_value(&Fr::from(30u64)),
    ]);
    let expected_output = aligned_value_from_parts(&[]);
    assert_eq!(call.proof_data.input, expected_input);
    assert_eq!(call.proof_data.output, expected_output);
    assert_eq!(rust_summary(&call.proof_data.input), ts.circuit.input);
    assert_eq!(rust_summary(&call.proof_data.output), ts.circuit.output);

    let tags: Vec<_> = call
        .proof_data
        .public_transcript
        .iter()
        .map(op_tag)
        .collect();
    assert_eq!(tags, ts.circuit.public_transcript_tags);
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
    assert_eq!(popeq_values.len(), ts.circuit.popeq_values.len());
    assert_eq!(
        std_lib::decode_fr(popeq_values[0]).unwrap(),
        Fr::from(16u64)
    );

    let private = call.proof_data.private_transcript_outputs().as_slice();
    assert_eq!(private.len(), ts.circuit.private_transcript_outputs.len());
    assert_eq!(ts.circuit.private_transcript_outputs[0].value_atoms, ["05"]);
    assert_eq!(ts.circuit.private_transcript_outputs[1].value_atoms, ["07"]);
    assert_eq!(private[0], proof_aligned_value(&Fr::from(5u64)));
    assert_eq!(private[1], proof_aligned_value(&Fr::from(7u64)));
    assert_eq!(
        rust_summary(&private[0]),
        ts.circuit.private_transcript_outputs[0]
    );
    assert_eq!(
        rust_summary(&private[1]),
        ts.circuit.private_transcript_outputs[1]
    );
    assert_eq!(std_lib::decode_fr(&private[0]).unwrap(), Fr::from(5u64));
    assert_eq!(std_lib::decode_fr(&private[1]).unwrap(), Fr::from(7u64));
}

#[test]
fn multi_level_conditional_call_transcripts_fold_into_one_ts_shaped_root() {
    let ts = ts_fixture();
    let contract: Contract<(), ProofWitnesses> = Contract::new(ProofWitnesses);
    let init = contract
        .initial_state(ctor_ctx(), Fr::from(11u64))
        .expect("initial_state");
    let ctx = CircuitContext::new(init.current_contract_state, ());
    let initial_query_context = ctx.current_query_context.clone();
    let result = contract
        .nested_read_witness_write(
            ctx,
            Fr::from(16u64),
            Fr::from(30u64),
            true,
            Fr::from(37u64),
            Fr::from(40u64),
        )
        .expect("nested_read_witness_write");

    let root = result
        .context
        .call_proof_data_trace
        .single_contract_call()
        .expect("one exported root proof record");
    assert_eq!(root.circuit_id, "nested_read_witness_write");
    assert_eq!(root.contract_address, &initial_query_context.address);
    assert_eq!(
        root.initial_query_context.address,
        initial_query_context.address
    );
    assert_eq!(
        root.initial_query_context.state,
        initial_query_context.state
    );
    assert_eq!(
        root.final_query_context.address,
        result.context.current_query_context.address
    );
    assert_eq!(
        root.final_query_context.state,
        result.context.current_query_context.state
    );
    assert_eq!(
        root.input,
        &aligned_value_from_parts(&[
            proof_aligned_value(&Fr::from(16u64)),
            proof_aligned_value(&Fr::from(30u64)),
            proof_aligned_value(&true),
            proof_aligned_value(&Fr::from(37u64)),
            proof_aligned_value(&Fr::from(40u64)),
        ])
    );
    assert_eq!(root.output, &aligned_value_from_parts(&[]));
    assert_eq!(rust_summary(root.input), ts.nested_circuit.input);
    assert_eq!(rust_summary(root.output), ts.nested_circuit.output);

    let tags: Vec<_> = root.public_transcript.iter().map(op_tag).collect();
    assert_eq!(tags, ts.nested_circuit.public_transcript_tags);
    assert_eq!(
        tags,
        [
            "dup", "idx", "popeq", // outer read before the local call
            "dup", "idx", "popeq", // conditional nested leaf read
            "push", "push", "ins", // conditional nested leaf write
            "dup", "idx", "popeq", // outer read after the local call
            "push", "push", "ins", // outer write
        ]
    );
    let popeq: Vec<_> = root
        .public_transcript
        .iter()
        .filter_map(|op| match op {
            Op::Popeq { result, .. } => Some(std_lib::decode_fr(result).unwrap()),
            _ => None,
        })
        .collect();
    assert_eq!(popeq, [Fr::from(16u64), Fr::from(16u64), Fr::from(37u64)]);
    assert_eq!(
        root.private_transcript_outputs.as_slice(),
        &[
            proof_aligned_value(&Fr::from(5u64)),
            proof_aligned_value(&Fr::from(7u64)),
            proof_aligned_value(&Fr::from(5u64)),
        ]
    );
    for (rust, expected) in root
        .private_transcript_outputs
        .as_slice()
        .iter()
        .zip(&ts.nested_circuit.private_transcript_outputs)
    {
        assert_eq!(rust_summary(rust), *expected);
    }
    assert_eq!(
        ledger(&result.context.current_query_context.state)
            .value()
            .expect("value"),
        Fr::from(50u64)
    );
}

#[test]
fn conditional_nested_call_only_folds_the_executed_branch() {
    let contract: Contract<(), ProofWitnesses> = Contract::new(ProofWitnesses);
    let init = contract
        .initial_state(ctor_ctx(), Fr::from(11u64))
        .expect("initial_state");
    let result = contract
        .nested_read_witness_write(
            CircuitContext::new(init.current_contract_state, ()),
            Fr::from(16u64),
            Fr::from(30u64),
            false,
            Fr::from(16u64),
            Fr::from(40u64),
        )
        .expect("nested_read_witness_write false branch");

    let root = result
        .context
        .call_proof_data_trace
        .single_contract_call()
        .expect("one exported root proof record");
    assert_eq!(
        root.public_transcript
            .iter()
            .map(op_tag)
            .collect::<Vec<_>>(),
        [
            "dup", "idx", "popeq", // outer read before the skipped call
            "dup", "idx", "popeq", // outer read after the skipped call
            "push", "push", "ins", // outer write
        ]
    );
    assert_eq!(
        root.private_transcript_outputs.as_slice(),
        &[
            proof_aligned_value(&Fr::from(5u64)),
            proof_aligned_value(&Fr::from(5u64)),
        ]
    );
    assert_eq!(
        ledger(&result.context.current_query_context.state)
            .value()
            .expect("value"),
        Fr::from(50u64)
    );
}

#[test]
fn sequential_exported_calls_remain_separate_and_fail_single_call_extraction() {
    let contract: Contract<(), ProofWitnesses> = Contract::new(ProofWitnesses);
    let init = contract
        .initial_state(ctor_ctx(), Fr::from(11u64))
        .expect("initial_state");
    let first = contract
        .read_witness_write(
            CircuitContext::new(init.current_contract_state, ()),
            Fr::from(16u64),
            Fr::from(30u64),
        )
        .expect("first top-level call");
    let second = contract
        .read_witness_write(first.context, Fr::from(42u64), Fr::from(50u64))
        .expect("second top-level call");

    let trace = &second.context.call_proof_data_trace;
    assert_eq!(trace.len(), 2);
    assert_eq!(
        trace
            .as_slice()
            .iter()
            .map(|call| call.circuit_id.as_str())
            .collect::<Vec<_>>(),
        ["read_witness_write", "read_witness_write"]
    );
    assert!(matches!(
        trace.single_contract_call(),
        Err(CompactError::ProofData(_))
    ));
}
