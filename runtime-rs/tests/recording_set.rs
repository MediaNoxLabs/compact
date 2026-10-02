// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");

use midnight_compact_runtime::context::{ConstructorContext, ConstructorResult};
use midnight_compact_runtime::fab::AlignedValue;
use midnight_compact_runtime::ledger::{
    ChargedState, ContractAddress, DefaultDB, StateValue, constructor_set,
};
use midnight_compact_runtime::recording::RecordingFrame;
use midnight_compact_runtime::slots::SetSlot;
use midnight_ledger::construct::{PreTranscript, partition_transcripts};
use midnight_ledger::structure::INITIAL_PARAMETERS;
use midnight_onchain_vm::cost_model::INITIAL_COST_MODEL;
use midnight_onchain_vm::ops::Op;
use midnight_serialize::tagged_serialize;

const SEEN: SetSlot<bool> = SetSlot::new(&[0]);

fn context() -> midnight_compact_runtime::context::CircuitContext<()> {
    let state = StateValue::Array(vec![constructor_set::<DefaultDB>()].into());
    ConstructorResult::new(ConstructorContext::new(()), ChargedState::new(state))
        .into_circuit_context(ContractAddress::default())
}

#[test]
fn set_mutations_and_observations_replay_with_native_effects() {
    let frame = SEEN
        .record_insert(RecordingFrame::new(context()), true)
        .unwrap();
    let (frame, member) = SEEN.record_member(frame, true).unwrap();
    assert!(member);
    let (frame, size) = SEEN.record_size(frame).unwrap();
    assert_eq!(size, 1);
    let (frame, empty) = SEEN.record_is_empty(frame).unwrap();
    assert!(!empty);
    let frame = SEEN.record_remove(frame, true).unwrap();
    let frame = SEEN.record_insert(frame, false).unwrap();
    let frame = SEEN.record_reset(frame).unwrap();
    let (frame, empty) = SEEN.record_is_empty(frame).unwrap();
    assert!(empty);
    let recorded = frame.finish(());

    let native = SEEN.insert(context(), true).unwrap();
    let mut native_gas = native.gas_cost;
    let native = SEEN.member(native.context, true).unwrap();
    native_gas += native.gas_cost;
    assert!(native.result);
    let native = SEEN.size(native.context).unwrap();
    native_gas += native.gas_cost;
    assert_eq!(native.result, 1);
    let native = SEEN.is_empty(native.context).unwrap();
    native_gas += native.gas_cost;
    assert!(!native.result);
    let native = SEEN.remove(native.context, true).unwrap();
    native_gas += native.gas_cost;
    let native = SEEN.insert(native.context, false).unwrap();
    native_gas += native.gas_cost;
    let native = SEEN.reset(native.context).unwrap();
    native_gas += native.gas_cost;
    let native = SEEN.is_empty(native.context).unwrap();
    native_gas += native.gas_cost;
    assert!(native.result);
    assert_eq!(recorded.execution.gas_cost, native_gas);

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

    let mut wrong_read = recorded.public.verify_ops().to_vec();
    *wrong_read.last_mut().unwrap() = Op::Popeq {
        cached: true,
        result: AlignedValue::from(false),
    };
    assert!(
        recorded
            .public
            .initial()
            .query(&wrong_read, None, &INITIAL_COST_MODEL)
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
    let (guaranteed, fallible) = &partitioned[0];
    let transcript = guaranteed.as_ref().or(fallible.as_ref()).unwrap();
    assert_eq!(transcript.effects, replay.context.effects);
}

#[test]
fn removing_an_absent_element_replays_as_an_unchanged_set() {
    let recorded = SEEN
        .record_remove(RecordingFrame::new(context()), true)
        .unwrap()
        .finish(());
    let replay = recorded
        .public
        .initial()
        .query(recorded.public.verify_ops(), None, &INITIAL_COST_MODEL)
        .unwrap();
    assert_eq!(
        replay.context.effects,
        recorded.execution.context.query.effects
    );
    assert_eq!(
        replay.context.state.get_ref(),
        recorded.execution.context.query.state.get_ref()
    );
}
