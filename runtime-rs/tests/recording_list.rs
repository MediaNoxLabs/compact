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

use midnight_compact_runtime::Field;
use midnight_compact_runtime::context::{ConstructorContext, ConstructorResult};
use midnight_compact_runtime::fab::AlignedValue;
use midnight_compact_runtime::ledger::{
    ChargedState, ContractAddress, DefaultDB, StateValue, constructor_list,
};
use midnight_compact_runtime::recording::RecordingFrame;
use midnight_compact_runtime::slots::ListSlot;
use midnight_ledger::construct::{PreTranscript, partition_transcripts};
use midnight_ledger::structure::INITIAL_PARAMETERS;
use midnight_onchain_vm::cost_model::INITIAL_COST_MODEL;
use midnight_onchain_vm::ops::Op;
use midnight_serialize::tagged_serialize;

const ITEMS: ListSlot<Field> = ListSlot::new(0);

fn context() -> midnight_compact_runtime::context::CircuitContext<()> {
    let state = StateValue::Array(vec![constructor_list::<DefaultDB>()].into());
    ConstructorResult::new(ConstructorContext::new(()), ChargedState::new(state))
        .into_circuit_context(ContractAddress::default())
}

#[test]
fn list_trace_matches_native_state_gas_and_effects() {
    let (frame, empty) = ITEMS
        .record_is_empty(RecordingFrame::new(context()))
        .unwrap();
    assert!(empty);
    let (frame, length) = ITEMS.record_length(frame).unwrap();
    assert_eq!(length, 0);
    let (frame, head): (_, (bool, Field)) = ITEMS.record_head(frame).unwrap();
    assert_eq!(head, (false, Field::from(0_u64)));
    let frame = ITEMS.record_push_front(frame, Field::from(7_u64)).unwrap();
    let (frame, length) = ITEMS.record_length(frame).unwrap();
    assert_eq!(length, 1);
    let (frame, head): (_, (bool, Field)) = ITEMS.record_head(frame).unwrap();
    assert_eq!(head, (true, Field::from(7_u64)));
    let frame = ITEMS.record_pop_front(frame).unwrap();
    let frame = ITEMS.record_push_front(frame, Field::from(8_u64)).unwrap();
    let recorded = ITEMS.record_reset(frame).unwrap().finish(());
    assert!(recorded.public.verify_ops().iter().any(|op| {
        matches!(
            op,
            Op::Concat {
                cached: false,
                n: 38
            }
        )
    }));

    let native = ITEMS.is_empty(context()).unwrap();
    let mut gas = native.gas_cost;
    assert!(native.result);
    let native = ITEMS.length(native.context).unwrap();
    gas += native.gas_cost;
    assert_eq!(native.result, 0);
    let native = ITEMS.head::<(bool, Field), _, _>(native.context).unwrap();
    gas += native.gas_cost;
    assert_eq!(native.result, (false, Field::from(0_u64)));
    let native = ITEMS
        .push_front(native.context, Field::from(7_u64))
        .unwrap();
    gas += native.gas_cost;
    let native = ITEMS.length(native.context).unwrap();
    gas += native.gas_cost;
    assert_eq!(native.result, 1);
    let native = ITEMS.head::<(bool, Field), _, _>(native.context).unwrap();
    gas += native.gas_cost;
    assert_eq!(native.result, (true, Field::from(7_u64)));
    let native = ITEMS.pop_front(native.context).unwrap();
    gas += native.gas_cost;
    let native = ITEMS
        .push_front(native.context, Field::from(8_u64))
        .unwrap();
    gas += native.gas_cost;
    let native = ITEMS.reset(native.context).unwrap();
    gas += native.gas_cost;
    assert_eq!(recorded.execution.gas_cost, gas);

    let replay = recorded
        .public
        .initial()
        .query(recorded.public.verify_ops(), None, &INITIAL_COST_MODEL)
        .unwrap();
    let mut native_bytes = Vec::new();
    tagged_serialize(native.context.query.state.get_ref(), &mut native_bytes).unwrap();
    let mut recorded_bytes = Vec::new();
    tagged_serialize(
        recorded.execution.context.query.state.get_ref(),
        &mut recorded_bytes,
    )
    .unwrap();
    let mut replay_bytes = Vec::new();
    tagged_serialize(replay.context.state.get_ref(), &mut replay_bytes).unwrap();
    assert_eq!(recorded_bytes, native_bytes);
    assert_eq!(replay_bytes, native_bytes);
    assert_eq!(
        recorded.execution.context.query.effects,
        native.context.query.effects
    );
    assert_eq!(replay.context.effects, native.context.query.effects);

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
    let (guaranteed, fallible) = &partitioned[0];
    let transcript = guaranteed.as_ref().or(fallible.as_ref()).unwrap();
    assert_eq!(transcript.effects, replay.context.effects);
}

#[test]
fn tampered_list_head_read_is_rejected() {
    let frame = ITEMS
        .record_push_front(RecordingFrame::new(context()), Field::from(7_u64))
        .unwrap();
    let (frame, head): (_, (bool, Field)) = ITEMS.record_head(frame).unwrap();
    assert_eq!(head, (true, Field::from(7_u64)));
    let recorded = frame.finish(head);
    let mut program = recorded.public.verify_ops().to_vec();
    *program.last_mut().unwrap() = Op::Popeq {
        cached: true,
        result: AlignedValue::from((true, Field::from(8_u64))),
    };
    assert!(
        recorded
            .public
            .initial()
            .query(&program, None, &INITIAL_COST_MODEL)
            .is_err()
    );
}
