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

use midnight_compact_runtime::context::{ConstructorContext, ConstructorResult};
use midnight_compact_runtime::fab::AlignedValue;
use midnight_compact_runtime::ledger::{
    ChargedState, ContractAddress, DefaultDB, StateValue, constructor_cell, read_cell,
};
use midnight_compact_runtime::recording::RecordingFrame;
use midnight_ledger::construct::{PreTranscript, partition_transcripts};
use midnight_ledger::structure::INITIAL_PARAMETERS;
use midnight_onchain_vm::cost_model::INITIAL_COST_MODEL;
use midnight_onchain_vm::ops::Op;
use midnight_serialize::tagged_serialize;

#[test]
fn boolean_cell_write_then_read_replays_as_one_verifying_program() {
    let state = StateValue::Array(vec![constructor_cell::<_, DefaultDB>(false)].into());
    let context = ConstructorResult::new(ConstructorContext::new(()), ChargedState::new(state))
        .into_circuit_context(ContractAddress::default());

    let frame = RecordingFrame::new(context).write_cell(0_u8, true).unwrap();
    let (frame, observed): (_, bool) = frame.read_cell(0_u8).unwrap();
    assert!(observed);
    let recorded = frame.finish(observed);
    assert!(recorded.execution.result);
    assert_eq!(recorded.public.verify_ops().len(), 6);

    let replay = recorded
        .public
        .initial()
        .query(recorded.public.verify_ops(), None, &INITIAL_COST_MODEL)
        .unwrap();
    let StateValue::Array(executed_fields) = recorded.execution.context.query.state.get_ref()
    else {
        panic!("executed state must be an array");
    };
    let StateValue::Array(replayed_fields) = replay.context.state.get_ref() else {
        panic!("replayed state must be an array");
    };
    assert!(read_cell::<bool, _>(&executed_fields.get(0).unwrap()).unwrap());
    assert!(read_cell::<bool, _>(&replayed_fields.get(0).unwrap()).unwrap());

    let mut executed_bytes = Vec::new();
    tagged_serialize(
        recorded.execution.context.query.state.get_ref(),
        &mut executed_bytes,
    )
    .unwrap();
    let mut replayed_bytes = Vec::new();
    tagged_serialize(replay.context.state.get_ref(), &mut replayed_bytes).unwrap();
    assert_eq!(executed_bytes, replayed_bytes);
    assert_eq!(
        recorded.execution.context.query.effects,
        replay.context.effects
    );

    let mut mismatched_read = recorded.public.verify_ops().to_vec();
    *mismatched_read.last_mut().unwrap() = Op::Popeq {
        cached: false,
        result: AlignedValue::from(false),
    };
    assert!(
        recorded
            .public
            .initial()
            .query(&mismatched_read, None, &INITIAL_COST_MODEL)
            .is_err()
    );

    let (initial, program) = recorded.public.into_parts();
    let partitioned = partition_transcripts(
        &[PreTranscript {
            context: initial,
            program,
            comm_comm: None,
        }],
        &INITIAL_PARAMETERS,
    )
    .unwrap();
    assert_eq!(partitioned.len(), 1);
    let (guaranteed, fallible) = &partitioned[0];
    let transcript = guaranteed.as_ref().or(fallible.as_ref()).unwrap();
    assert_eq!(transcript.program.len(), 6);
    assert_eq!(transcript.effects, replay.context.effects);
}
