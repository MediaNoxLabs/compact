// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Source-matched native, recorded and independent public-program controls.
use compact_rust_effect_plan_counter_source_fixture::{
    ledger_contract as contract, ledger_slots, runtime,
};
use runtime::context::{CircuitContext, ConstructorContext, ConstructorResult};
use runtime::ledger::{ChargedState, ContractAddress, DefaultDB, StateValue, constructor_cell};
use runtime::recording::{RecordedCircuitResult, RecordingFrame};
use runtime::{BoundedUint, CompactError, Field};

type Amount = BoundedUint<65535>;

fn context(round: u64, peer: u64) -> CircuitContext<u64> {
    let state = StateValue::Array(
        vec![
            constructor_cell::<_, DefaultDB>(round),
            constructor_cell::<_, DefaultDB>(peer),
            constructor_cell::<_, DefaultDB>(Field::from(42_u64)),
        ]
        .into(),
    );
    ConstructorResult::new(ConstructorContext::new(7), ChargedState::new(state))
        .into_circuit_context(ContractAddress::default())
}

/// Calls the official slot adapters in the source's prescribed query order.
fn reference(
    round: u64,
    peer: u64,
    amount: u16,
    remove: u16,
) -> Result<RecordedCircuitResult<u64, Field>, CompactError> {
    let frame =
        ledger_slots::round.record_increment(RecordingFrame::new(context(round, peer)), amount)?;
    let (frame, threshold) = ledger_slots::peer.record_read(frame)?;
    let (frame, selected) = ledger_slots::round.record_less_than(frame, threshold)?;
    let frame = if selected {
        ledger_slots::round.record_decrement(frame, remove)?
    } else {
        ledger_slots::round.record_reset(frame)?
    };
    let (frame, result) = ledger_slots::round.record_read(frame)?;
    let result = Field::from(result);
    let frame = ledger_slots::outcome.record_write(frame, result)?;
    Ok(frame.finish(result))
}

#[test]
fn counter_boundaries_preserve_results_failures_effects_and_gas() {
    let mut successes = 0;
    let mut failures = 0;
    for round in [0_u64, 1, u64::MAX] {
        for peer in [0_u64, 1, 10, u64::MAX] {
            for amount in [0_u16, 1, u16::MAX] {
                for remove in [0_u16, 1, u16::MAX] {
                    let args = (
                        Amount::new(amount.into()).unwrap(),
                        Amount::new(remove.into()).unwrap(),
                    );
                    let native = contract::adjust(context(round, peer), args.0, args.1);
                    let recorded = contract::recorded::adjust(context(round, peer), args.0, args.1);
                    let manual = reference(round, peer, amount, remove);
                    let arithmetic = round.checked_add(amount.into()).and_then(|value| {
                        if value < peer {
                            value.checked_sub(remove.into())
                        } else {
                            Some(0)
                        }
                    });
                    match manual {
                        Err(error) => {
                            failures += 1;
                            assert!(arithmetic.is_none(), "unexpected VM refusal: {error}");
                            assert_eq!(native.err().expect("native must refuse"), error);
                            assert_eq!(recorded.err().expect("recorded must refuse"), error);
                        }
                        Ok(manual) => {
                            successes += 1;
                            let expected = arithmetic.expect("arithmetic must succeed");
                            let native = native.unwrap();
                            let recorded = recorded.unwrap();
                            assert_eq!(native.result, Field::from(expected));
                            assert_eq!(recorded.execution.result, native.result);
                            assert_eq!(manual.execution.result, native.result);
                            assert_eq!(
                                native.context.query.state,
                                recorded.execution.context.query.state
                            );
                            assert_eq!(
                                native.context.query.state,
                                manual.execution.context.query.state
                            );
                            assert_eq!(
                                native.context.query.effects,
                                recorded.execution.context.query.effects
                            );
                            assert_eq!(
                                native.context.query.effects,
                                manual.execution.context.query.effects
                            );
                            assert_eq!(native.gas_cost, recorded.execution.gas_cost);
                            assert_eq!(native.gas_cost, manual.execution.gas_cost);
                            assert_eq!(native.context.private_state, 7);
                            assert_eq!(recorded.execution.context.private_state, 7);
                            assert!(native.private_transcript_outputs.is_empty());
                            assert!(recorded.execution.private_transcript_outputs.is_empty());
                            assert_eq!(
                                ledger_slots::round
                                    .inspect(native.context.query.state.get_ref())
                                    .unwrap(),
                                expected
                            );
                            assert_eq!(
                                ledger_slots::peer
                                    .inspect(native.context.query.state.get_ref())
                                    .unwrap(),
                                peer
                            );
                            assert_eq!(
                                ledger_slots::outcome
                                    .inspect(native.context.query.state.get_ref())
                                    .unwrap(),
                                Field::from(expected)
                            );
                            assert_eq!(
                                serde_json::to_value(recorded.public.verify_ops()).unwrap(),
                                serde_json::to_value(manual.public.verify_ops()).unwrap()
                            );
                            let replay = recorded
                                .public
                                .initial()
                                .query(
                                    recorded.public.verify_ops(),
                                    None,
                                    &recorded.execution.context.cost_model,
                                )
                                .unwrap();
                            let reference_replay = manual
                                .public
                                .initial()
                                .query(
                                    manual.public.verify_ops(),
                                    None,
                                    &manual.execution.context.cost_model,
                                )
                                .unwrap();
                            assert_eq!(replay.context.state, native.context.query.state);
                            assert_eq!(replay.context.effects, native.context.query.effects);
                            // Batched replay is compared with batched replay, never per-query execution gas.
                            assert_eq!(replay.gas_cost, reference_replay.gas_cost);
                        }
                    }
                }
            }
        }
    }
    assert_eq!(successes + failures, 108);
    assert!(successes > 0 && failures > 0);
}

#[test]
fn peer_query_precedes_less_than_and_reset_skips_underflow() {
    let amount = Amount::new(10).unwrap();
    let remove = Amount::new(u16::MAX.into()).unwrap();
    let recorded = contract::recorded::adjust(context(0, 10), amount, remove).unwrap();
    assert_eq!(recorded.execution.result, Field::from(0_u64));
    let expected = reference(0, 10, 10, u16::MAX).unwrap();
    assert_eq!(
        serde_json::to_value(recorded.public.verify_ops()).unwrap(),
        serde_json::to_value(expected.public.verify_ops()).unwrap()
    );
    // Both queries are observable even though moving them leaves this final state unchanged.
    let frame = ledger_slots::round
        .record_increment(RecordingFrame::new(context(0, 10)), 10)
        .unwrap();
    let (frame, _) = ledger_slots::round.record_less_than(frame, 10).unwrap();
    let (frame, _) = ledger_slots::peer.record_read(frame).unwrap();
    let frame = ledger_slots::round.record_reset(frame).unwrap();
    let (frame, result) = ledger_slots::round.record_read(frame).unwrap();
    let frame = ledger_slots::outcome
        .record_write(frame, Field::from(result))
        .unwrap();
    let reordered = frame.finish(Field::from(result));
    assert_eq!(
        recorded.execution.context.query.state,
        reordered.execution.context.query.state
    );
    assert_ne!(
        serde_json::to_value(recorded.public.verify_ops()).unwrap(),
        serde_json::to_value(reordered.public.verify_ops()).unwrap()
    );
}
