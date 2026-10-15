// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use compact_rust_effect_plan_qualified_coin_source_fixture::{
    ledger_contract as c, ledger_slots as slots, runtime as r, support::*,
};
use r::context::CircuitResult;
use r::recording::RecordedCircuitResult;

fn equal_execution<T: std::fmt::Debug + PartialEq>(
    native: &CircuitResult<u64, T>,
    recorded: &RecordedCircuitResult<u64, T>,
    manual: &RecordedCircuitResult<u64, T>,
) {
    for execution in [&recorded.execution, &manual.execution] {
        assert_eq!(execution.result, native.result);
        assert_eq!(
            execution.context.private_state,
            native.context.private_state
        );
        assert_eq!(execution.context.query.state, native.context.query.state);
        assert_eq!(
            execution.context.query.effects,
            native.context.query.effects
        );
        assert_eq!(execution.gas_cost, native.gas_cost);
        assert_eq!(
            execution.private_transcript_outputs,
            native.private_transcript_outputs
        );
    }
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
    assert_eq!(replay.context.state, native.context.query.state);
    assert_eq!(replay.context.effects, native.context.query.effects);
    // Execution sums separate write/read queries. QueryContext::query charges
    // state-update overhead per invocation, so one batched replay has a
    // different compute cost. Compare like-for-like manual transcript replay;
    // native/generated/manual sequential execution costs are checked above.
    let manual_replay = manual
        .public
        .initial()
        .query(
            manual.public.verify_ops(),
            None,
            &manual.execution.context.cost_model,
        )
        .unwrap();
    assert_eq!(replay.context.state, manual_replay.context.state);
    assert_eq!(replay.context.effects, manual_replay.context.effects);
    assert_eq!(replay.gas_cost, manual_replay.gas_cost);
}

#[test]
fn nested_coin_slot_uses_allocated_index_and_both_recipient_branches() {
    use midnight_serialize::Serializable;
    let (mut accepted, mut refused) = (0, 0);
    for user in [false, true] {
        for index in [0, 7, u64::MAX] {
            for amount in [0, 42, u128::MAX] {
                let case = format!("user={user}, index={index}, amount={amount}");
                let coin = coin(amount);
                let recipient = recipient(user);
                let initial = || allocate(context(), &coin, &recipient, index);
                let (official_coin, _) = carriers(&coin, &recipient);
                let size = r::fab::AlignedValue::from(official_coin).serialized_size()
                    + r::fab::AlignedValue::from(index).serialized_size();
                let (native_context, recorded_context, manual_context) =
                    (initial(), initial(), initial());
                let snapshots = [
                    native_context.query.state.clone(),
                    recorded_context.query.state.clone(),
                    manual_context.query.state.clone(),
                ];
                let original_state = snapshots[0].get_ref().clone();
                let native = c::write_read(native_context, coin.clone(), recipient.clone());
                let recorded =
                    c::recorded::write_read(recorded_context, coin.clone(), recipient.clone());
                let manual = write_read(manual_context, &coin, &recipient);
                if size > 91 {
                    refused += 1;
                    let expected =
                        r::CompactError::LedgerQueryRejected("Execution(BoundsExceeded)".into());
                    assert_eq!(native.err().expect(&case), expected, "{case}");
                    assert_eq!(recorded.err().expect(&case), expected, "{case}");
                    assert_eq!(manual.err().expect(&case), expected, "{case}");
                    for snapshot in snapshots {
                        assert_eq!(snapshot.get_ref(), &original_state, "{case}");
                    }
                    continue;
                }
                accepted += 1;
                let native = native.unwrap_or_else(|error| panic!("native {case}: {error}"));
                let recorded = recorded.unwrap_or_else(|error| panic!("recorded {case}: {error}"));
                let manual = manual.unwrap_or_else(|error| panic!("manual {case}: {error}"));
                equal_execution(&native, &recorded, &manual);
                assert_eq!(native.result.nonce, coin.nonce);
                assert_eq!(native.result.color, coin.color);
                assert_eq!(native.result.value, coin.value);
                assert_eq!(native.result.mt_index.value(), index as u128);
                assert!(native.private_transcript_outputs.is_empty());
                assert_eq!(native.context.private_state, 7);
                assert_eq!(
                    slots::padding00
                        .inspect(native.context.query.state.get_ref())
                        .unwrap(),
                    r::Field::from(0_u64)
                );
                assert_eq!(
                    slots::padding14
                        .inspect(native.context.query.state.get_ref())
                        .unwrap(),
                    r::Field::from(0_u64)
                );
                // A second invocation replaces the existing cell without inferring
                // qualification from its previous stored index.
                let mut occupied = context();
                occupied.query.state = native.context.query.state.clone();
                let replacement = c::recorded::write_read(
                    allocate(occupied, &coin, &recipient, 19),
                    coin,
                    recipient,
                )
                .unwrap();
                assert_eq!(replacement.execution.result.mt_index.value(), 19);
            }
        }
    }
    assert_eq!((accepted, refused), (12, 6));
}

#[test]
fn witness_operands_run_once_in_source_order_before_coin_storage() {
    for user in [false, true] {
        let make_witness = || Witness {
            coin: coin(42),
            recipient: recipient(user),
            ..Default::default()
        };
        let a = make_witness();
        let b = make_witness();
        let m = make_witness();
        let initial = || allocate(context(), &a.coin, &a.recipient, 13);
        let native = c::write_once(initial(), &a).unwrap();
        let recorded = c::recorded::write_once(initial(), &b).unwrap();
        let manual = write_once(initial(), &m).unwrap();
        equal_execution(&native, &recorded, &manual);
        assert_eq!(*a.calls.borrow(), vec![("coin", 7), ("recipient", 8)]);
        assert_eq!(*a.calls.borrow(), *b.calls.borrow());
        assert_eq!(*a.calls.borrow(), *m.calls.borrow());
        assert_eq!(native.context.private_state, 9);
        assert_eq!(
            native.private_transcript_outputs,
            vec![
                r::fab::AlignedValue::from(a.coin.clone()),
                r::fab::AlignedValue::from(a.recipient.clone()),
            ]
        );
        assert_eq!(
            slots::pot
                .inspect(native.context.query.state.get_ref())
                .unwrap()
                .mt_index
                .value(),
            13
        );
    }
}

#[test]
fn callback_failure_stops_before_later_operand_or_missing_allocation_error() {
    for (refuse, expected_calls) in [
        ("coin", vec![("coin", 7)]),
        ("recipient", vec![("coin", 7), ("recipient", 8)]),
    ] {
        let make_witness = || Witness {
            coin: coin(42),
            recipient: recipient(true),
            refuse: Some(refuse),
            ..Default::default()
        };
        let a = make_witness();
        let b = make_witness();
        let m = make_witness();
        // Deliberately no allocation: a witness failure must precede the ledger
        // error and must never cause the later witness to be evaluated.
        let expected = r::CompactError::AssertionFailed(format!("{refuse} refused"));
        assert_eq!(c::write_once(context(), &a).err().unwrap(), expected);
        assert_eq!(
            c::recorded::write_once(context(), &b).err().unwrap(),
            expected
        );
        assert_eq!(write_once(context(), &m).err().unwrap(), expected);
        assert_eq!(*a.calls.borrow(), expected_calls);
        assert_eq!(*a.calls.borrow(), *b.calls.borrow());
        assert_eq!(*a.calls.borrow(), *m.calls.borrow());
    }
}

#[test]
fn absent_or_wrong_recipient_allocation_is_not_inferred_from_the_coin() {
    for wrong_recipient in [false, true] {
        let make_witness = || Witness {
            coin: coin(42),
            recipient: recipient(true),
            ..Default::default()
        };
        let a = make_witness();
        let b = make_witness();
        let m = make_witness();
        let initial = || {
            if wrong_recipient {
                allocate(context(), &a.coin, &recipient(false), 7)
            } else {
                context()
            }
        };
        let native = c::write_once(initial(), &a).err().unwrap();
        let recorded = c::recorded::write_once(initial(), &b).err().unwrap();
        let manual = write_once(initial(), &m).err().unwrap();
        assert!(matches!(native, r::CompactError::InvalidLedgerCell(_)));
        assert!(native.to_string().contains("Coin commitment not found"));
        assert_eq!(native, recorded);
        assert_eq!(native, manual);
        assert_eq!(*a.calls.borrow(), vec![("coin", 7), ("recipient", 8)]);
        assert_eq!(*a.calls.borrow(), *b.calls.borrow());
        assert_eq!(*a.calls.borrow(), *m.calls.borrow());
        let direct = c::write_read(initial(), a.coin.clone(), a.recipient.clone())
            .err()
            .unwrap();
        let recorded = c::recorded::write_read(initial(), a.coin.clone(), a.recipient.clone())
            .err()
            .unwrap();
        assert_eq!(native, direct);
        assert_eq!(native, recorded);
    }
}

#[test]
fn qualified_write_preserves_official_serialized_concat_bound() {
    use midnight_serialize::Serializable;
    // The existing ledger8 writeCoin program uses Concat { n: 91 }. Its limit
    // applies to the combined serialized coin/index, not just a numeric index.
    for user in [false, true] {
        for (amount, index, expected_size) in [
            ((1_u128 << 104) - 1, 0, 91),
            (1_u128 << 104, 0, 92),
            (u128::MAX, 0, 94),
            ((1_u128 << 40) - 1, u64::MAX, 91),
            (1_u128 << 40, u64::MAX, 92),
            (u128::MAX, u64::MAX, 102),
        ] {
            let coin = coin(amount);
            let recipient = recipient(user);
            let case = format!("user={user}, index={index}, amount={amount}");
            let (official_coin, _) = carriers(&coin, &recipient);
            let size = r::fab::AlignedValue::from(official_coin).serialized_size()
                + r::fab::AlignedValue::from(index).serialized_size();
            assert_eq!(size, expected_size, "{case}");
            let initial = || allocate(context(), &coin, &recipient, index);
            let native = c::write_read(initial(), coin.clone(), recipient.clone());
            let recorded = c::recorded::write_read(initial(), coin.clone(), recipient.clone());
            let manual = write_read(initial(), &coin, &recipient);
            if expected_size <= 91 {
                equal_execution(
                    &native.unwrap_or_else(|error| panic!("native {case}: {error}")),
                    &recorded.unwrap_or_else(|error| panic!("recorded {case}: {error}")),
                    &manual.unwrap_or_else(|error| panic!("manual {case}: {error}")),
                );
            } else {
                let expected =
                    r::CompactError::LedgerQueryRejected("Execution(BoundsExceeded)".into());
                assert_eq!(native.err().expect(&case), expected, "{case}");
                assert_eq!(recorded.err().expect(&case), expected, "{case}");
                assert_eq!(manual.err().expect(&case), expected, "{case}");
            }
        }
        // Numeric maxima themselves are supported when the other canonical FAB
        // atoms have shorter normalized payloads; no compiler narrowing applies.
        let mut compact_coin = coin(u128::MAX);
        compact_coin.nonce = r::FixedBytes::new([0; 32]);
        compact_coin.color = r::FixedBytes::new([0; 32]);
        let recipient = recipient(user);
        let initial = || allocate(context(), &compact_coin, &recipient, u64::MAX);
        let native = c::write_read(initial(), compact_coin.clone(), recipient.clone()).unwrap();
        let recorded =
            c::recorded::write_read(initial(), compact_coin.clone(), recipient.clone()).unwrap();
        let manual = write_read(initial(), &compact_coin, &recipient).unwrap();
        equal_execution(&native, &recorded, &manual);
        assert_eq!(native.result.value.value(), u128::MAX);
        assert_eq!(native.result.mt_index.value(), u64::MAX as u128);
    }
}
