// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use compact_rust_effect_plan_qualified_set_source_fixture::{
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
            execution.context.circuit_zswap(),
            native.context.circuit_zswap()
        );
        assert_eq!(
            execution.context.query.call_context.com_indices,
            native.context.query.call_context.com_indices
        );
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
fn nested_helpers_branches_and_reused_operands_preserve_execution() {
    for user in [false, true] {
        for selected in [false, true] {
            let coin = coin(42);
            let recipient = recipient(user);
            let make_witness = || Witness {
                coin: witness_coin(),
                recipient: self::recipient(!user),
                ..Default::default()
            };
            let (a, b, m) = (make_witness(), make_witness(), make_witness());
            let initial = || {
                allocate(
                    allocate(context(), &coin, &recipient, 11),
                    &a.coin,
                    &a.recipient,
                    13,
                )
            };
            let original = initial();
            let native =
                c::insert_selected(initial(), &a, coin.clone(), recipient.clone(), selected)
                    .unwrap();
            let recorded = c::recorded::insert_selected(
                initial(),
                &b,
                coin.clone(),
                recipient.clone(),
                selected,
            )
            .unwrap();
            let manual = insert_selected(initial(), &coin, &recipient, selected, &m).unwrap();
            equal_execution(&native, &recorded, &manual);
            let expected_calls = if selected {
                vec![("coin", 7, 1), ("recipient", 8, 1)]
            } else {
                vec![]
            };
            for witness in [&a, &b, &m] {
                assert_eq!(*witness.calls.borrow(), expected_calls);
            }
            assert_eq!(native.context.private_state, if selected { 9 } else { 7 });
            let expected_outputs = if selected {
                vec![
                    r::fab::AlignedValue::from(a.coin.clone()),
                    r::fab::AlignedValue::from(a.recipient.clone()),
                ]
            } else {
                vec![]
            };
            assert_eq!(native.private_transcript_outputs, expected_outputs);
            let set = slots::coins
                .inspect(native.context.query.state.get_ref())
                .unwrap();
            assert_eq!(set.size().unwrap().value(), if selected { 2 } else { 1 });
            assert!(set.member(qualified(&coin, 11)));
            assert_eq!(set.member(qualified(&a.coin, 13)), selected);
            assert!(!set.member(qualified(&coin, 14)));
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
            assert_eq!(native.context.circuit_zswap(), original.circuit_zswap());
            assert_eq!(
                native.context.query.call_context.com_indices,
                original.query.call_context.com_indices
            );
            assert!(
                slots::coins
                    .inspect(original.query.state.get_ref())
                    .unwrap()
                    .is_empty()
            );
        }
    }
}

#[test]
fn callback_failure_preserves_source_prefix_and_unselected_branch_is_lazy() {
    for refuse in ["coin", "recipient"] {
        for selected in [false, true] {
            let coin = coin(42);
            let recipient = recipient(true);
            let make_witness = || Witness {
                coin: witness_coin(),
                recipient: self::recipient(false),
                refuse: Some(refuse),
                ..Default::default()
            };
            let (a, b, m) = (make_witness(), make_witness(), make_witness());
            // Only the first/root coin is allocated. Selected callbacks must
            // fail after that insertion and before the helper's allocation lookup.
            let initial = || allocate(context(), &coin, &recipient, 11);
            let native =
                c::insert_selected(initial(), &a, coin.clone(), recipient.clone(), selected);
            let recorded = c::recorded::insert_selected(
                initial(),
                &b,
                coin.clone(),
                recipient.clone(),
                selected,
            );
            let manual = insert_selected(initial(), &coin, &recipient, selected, &m);
            let expected_calls = if !selected {
                vec![]
            } else if refuse == "coin" {
                vec![("coin", 7, 1)]
            } else {
                vec![("coin", 7, 1), ("recipient", 8, 1)]
            };
            if selected {
                let expected = r::CompactError::AssertionFailed(format!("{refuse} refused"));
                assert_eq!(native.err().unwrap(), expected);
                assert_eq!(recorded.err().unwrap(), expected);
                assert_eq!(manual.err().unwrap(), expected);
            } else {
                equal_execution(&native.unwrap(), &recorded.unwrap(), &manual.unwrap());
            }
            for witness in [&a, &b, &m] {
                assert_eq!(*witness.calls.borrow(), expected_calls);
            }
        }
    }
}

#[test]
fn absent_or_wrong_recipient_allocations_fail_at_the_correct_effect() {
    // Failure before callbacks versus after both callbacks and the first insert.
    for missing_root in [false, true] {
        for wrong in [0, 1, 2] {
            let coin = coin(42);
            let recipient = recipient(true);
            let make_witness = || Witness {
                coin: witness_coin(),
                recipient: self::recipient(false),
                ..Default::default()
            };
            let (a, b, m) = (make_witness(), make_witness(), make_witness());
            let initial = || {
                let state = if missing_root {
                    context()
                } else {
                    allocate(context(), &coin, &recipient, 11)
                };
                match wrong {
                    0 => state,
                    1 => allocate(state, &a.coin, &self::recipient(true), 13),
                    _ => {
                        let mut other_contract = a.recipient.clone();
                        other_contract.right.bytes = r::FixedBytes::new([10; 32]);
                        allocate(state, &a.coin, &other_contract, 13)
                    }
                }
            };
            let (ca, cb, cm) = (initial(), initial(), initial());
            let snapshots = [
                ca.query.state.clone(),
                cb.query.state.clone(),
                cm.query.state.clone(),
            ];
            let native = c::insert_selected(ca, &a, coin.clone(), recipient.clone(), true)
                .err()
                .unwrap();
            let recorded =
                c::recorded::insert_selected(cb, &b, coin.clone(), recipient.clone(), true)
                    .err()
                    .unwrap();
            let manual = insert_selected(cm, &coin, &recipient, true, &m)
                .err()
                .unwrap();
            assert!(matches!(native, r::CompactError::InvalidLedgerCell(_)));
            assert!(native.to_string().contains("Coin commitment not found"));
            assert_eq!(native, recorded);
            assert_eq!(native, manual);
            let expected_calls = if missing_root {
                vec![]
            } else {
                vec![("coin", 7, 1), ("recipient", 8, 1)]
            };
            for witness in [&a, &b, &m] {
                assert_eq!(*witness.calls.borrow(), expected_calls);
            }
            for snapshot in snapshots {
                assert!(slots::coins.inspect(snapshot.get_ref()).unwrap().is_empty());
            }
        }
    }
}

#[test]
fn qualified_set_insert_preserves_official_serialized_concat_bound() {
    use midnight_serialize::Serializable;
    // The existing ledger8 insertCoin program uses Concat { n: 91 }. Its limit
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
            let native = c::insert_once(initial(), coin.clone(), recipient.clone());
            let recorded = c::recorded::insert_once(initial(), coin.clone(), recipient.clone());
            let manual = insert_once(initial(), &coin, &recipient);
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
        let native = c::insert_once(initial(), compact_coin.clone(), recipient.clone()).unwrap();
        let recorded =
            c::recorded::insert_once(initial(), compact_coin.clone(), recipient.clone()).unwrap();
        let manual = insert_once(initial(), &compact_coin, &recipient).unwrap();
        equal_execution(&native, &recorded, &manual);
        assert!(
            slots::coins
                .inspect(native.context.query.state.get_ref())
                .unwrap()
                .member(qualified(&compact_coin, u64::MAX))
        );
    }
}
