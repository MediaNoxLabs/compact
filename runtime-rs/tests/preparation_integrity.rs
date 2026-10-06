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

//! Public preparation boundaries; these tests do not generate cryptographic proofs.
#![cfg(feature = "ledger-transaction")]

use midnight_compact_runtime::Field;
use midnight_compact_runtime::context::{CircuitContext, ConstructorContext, ConstructorResult};
use midnight_compact_runtime::fab::AlignedValue;
use midnight_compact_runtime::ledger::{
    ChargedState, ContractAddress, HashOutput, StateValue, constructor_cell,
};
use midnight_compact_runtime::recording::{RecordedCircuitResult, RecordingFrame};
use midnight_compact_runtime::transaction::{CallSpec, PrepareCallError, prepare_call};
use rand::{Rng, SeedableRng, rngs::StdRng};

fn context() -> CircuitContext<()> {
    ConstructorResult::new(
        ConstructorContext::new(()),
        ChargedState::new(StateValue::Array(vec![constructor_cell(false)].into())),
    )
    .into_circuit_context(ContractAddress::default())
}

fn recorded() -> RecordedCircuitResult<(), bool> {
    let frame = RecordingFrame::new(context())
        .write_cell(0_u8, true)
        .unwrap();
    let (frame, value) = frame.read_cell::<bool>(0_u8).unwrap();
    frame.finish(value)
}

fn spec() -> CallSpec {
    // A deterministic verifier carrier is sufficient for preparation, not proof verification.
    CallSpec::new(
        "integrity",
        StdRng::seed_from_u64(244).r#gen(),
        (),
        Field::from(0),
    )
}

fn alter_effects(recorded: &mut RecordedCircuitResult<(), bool>) {
    let effects = &mut recorded.execution.context.query.effects;
    effects.shielded_mints = effects.shielded_mints.insert(HashOutput([17; 32]), 1);
}

fn alter_state(recorded: &mut RecordedCircuitResult<(), bool>) {
    recorded.execution.context.query.state = context().query.state;
}

#[test]
fn untouched_recording_prepares_with_the_original_program_and_output() {
    let recorded = recorded();
    assert!(!recorded.public.verify_ops().is_empty());
    let expected_program = recorded.public.verify_ops().to_vec();
    let expected_effects = recorded.execution.context.query.effects.clone();
    let prepared = prepare_call(recorded, spec()).unwrap();
    let transcript = prepared
        .guaranteed_public_transcript
        .as_ref()
        .or(prepared.fallible_public_transcript.as_ref())
        .unwrap();
    assert_eq!(transcript.program, expected_program.into());
    assert_eq!(transcript.effects, expected_effects);
    assert_eq!(prepared.output, AlignedValue::from(true));
}

#[test]
fn preparation_rejects_effects_added_outside_the_recorded_program() {
    let mut recorded = recorded();
    alter_effects(&mut recorded);
    assert!(matches!(
        prepare_call(recorded, spec()),
        Err(PrepareCallError::ReplayEffectsMismatch)
    ));
}

#[test]
fn preparation_rejects_state_replaced_outside_the_recorded_program() {
    let mut recorded = recorded();
    alter_state(&mut recorded);
    assert!(matches!(
        prepare_call(recorded, spec()),
        Err(PrepareCallError::ReplayStateMismatch)
    ));
}

#[test]
fn effects_mismatch_precedes_state_mismatch() {
    let mut recorded = recorded();
    alter_effects(&mut recorded);
    alter_state(&mut recorded);
    assert!(matches!(
        prepare_call(recorded, spec()),
        Err(PrepareCallError::ReplayEffectsMismatch)
    ));
}

#[test]
fn identity_and_empty_transcript_guards_precede_replay_mismatches() {
    for empty in [false, true] {
        let mut recorded = if empty {
            RecordingFrame::new(context()).finish(true)
        } else {
            recorded()
        };
        alter_effects(&mut recorded);
        alter_state(&mut recorded);
        recorded.execution.context = recorded
            .execution
            .context
            .with_coin_public_key_bytes([9; 32]);
        let error = prepare_call(recorded, spec()).err().unwrap();
        if empty {
            assert!(matches!(error, PrepareCallError::EmptyTranscript));
        } else {
            assert!(matches!(error, PrepareCallError::CoinPublicKeyMismatch));
        }
    }
}
