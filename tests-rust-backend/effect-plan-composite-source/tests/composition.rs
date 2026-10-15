// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use compact_rust_effect_plan_composite_source_fixture::{
    ledger_contract as c, runtime as r, types,
};
use r::context::{CircuitContext, ConstructorContext, WitnessContext};
use r::recording::{RecordedCircuitResult, RecordingFrame};
use r::{BoundedUint, CompactError, FixedBytes};
use std::cell::RefCell;
fn context() -> CircuitContext<u64> {
    c::initial_state(ConstructorContext::new(10))
        .unwrap()
        .into_circuit_context(Default::default())
}
fn coin(value: u128) -> types::ShieldedCoinInfo {
    types::ShieldedCoinInfo {
        nonce: FixedBytes::new([1; 32]),
        color: FixedBytes::new([2; 32]),
        value: BoundedUint::new(value).unwrap(),
    }
}
fn input(value: u128) -> types::QualifiedShieldedCoinInfo {
    let coin = coin(value);
    types::QualifiedShieldedCoinInfo {
        nonce: coin.nonce,
        color: coin.color,
        value: coin.value,
        mt_index: BoundedUint::new(17).unwrap(),
    }
}
fn recipient(user: bool) -> types::Either {
    types::Either {
        is_left: user,
        left: types::ZswapCoinPublicKey {
            bytes: FixedBytes::new([3; 32]),
        },
        right: types::ContractAddress {
            bytes: FixedBytes::new([4; 32]),
        },
    }
}
fn native_coin(coin: &types::ShieldedCoinInfo) -> r::ledger::CoinInfo {
    r::ledger::coin_info_from_compact(coin.nonce, coin.color, coin.value.value())
}
fn native_input(coin: &types::QualifiedShieldedCoinInfo) -> r::ledger::QualifiedCoinInfo {
    r::ledger::coin_info_from_compact(coin.nonce, coin.color, coin.value.value())
        .qualify(coin.mt_index.value() as u64)
}
fn native_recipient(recipient: &types::Either) -> r::ledger::CoinRecipient {
    r::ledger::coin_recipient_from_compact(
        recipient.is_left,
        recipient.left.bytes,
        recipient.right.bytes,
    )
}
#[derive(Default)]
struct Witness {
    calls: RefCell<Vec<(&'static str, u64)>>,
    fail_at: Option<&'static str>,
}
impl Witness {
    fn coin(
        &self,
        private: u64,
        coin: types::ShieldedCoinInfo,
    ) -> Result<(u64, types::ShieldedCoinInfo), CompactError> {
        self.calls.borrow_mut().push(("coin", private));
        if self.fail_at == Some("coin") {
            return Err(CompactError::AssertionFailed("coin refused".into()));
        }
        Ok((private + 1, coin))
    }
    fn recipient(
        &self,
        private: u64,
        recipient: types::Either,
    ) -> Result<(u64, types::Either), CompactError> {
        self.calls.borrow_mut().push(("recipient", private));
        if self.fail_at == Some("recipient") {
            return Err(CompactError::AssertionFailed("recipient refused".into()));
        }
        Ok((private + 1, recipient))
    }
}
impl c::TryWitnesses<u64> for Witness {
    fn choose_coin(
        &self,
        ctx: WitnessContext<'_, u64, c::LedgerView<'_>>,
        coin: types::ShieldedCoinInfo,
    ) -> Result<(u64, types::ShieldedCoinInfo), CompactError> {
        self.coin(*ctx.private_state, coin)
    }
    fn choose_recipient(
        &self,
        ctx: WitnessContext<'_, u64, c::LedgerView<'_>>,
        recipient: types::Either,
    ) -> Result<(u64, types::Either), CompactError> {
        self.recipient(*ctx.private_state, recipient)
    }
}
fn reference(
    ctx: CircuitContext<u64>,
    w: &Witness,
    selected: bool,
    value: u128,
    user: bool,
) -> Result<RecordedCircuitResult<u64, types::Receipt>, CompactError> {
    let coin = coin(value);
    let recipient = recipient(user);
    let input = input(value);
    let frame = RecordingFrame::new(ctx).create_zswap_input(native_input(&input));
    let frame = if selected {
        frame.create_zswap_output(native_coin(&coin), native_recipient(&recipient))?
    } else {
        frame
    };
    let frame = frame.create_zswap_input(native_input(&input));
    let (frame, chosen_coin) =
        frame.try_witness_metered(|ctx, _| w.coin(ctx.private_state, coin.clone()))?;
    let (frame, chosen_recipient) =
        frame.try_witness_metered(|ctx, _| w.recipient(ctx.private_state, recipient.clone()))?;
    let frame = frame.create_zswap_output(
        native_coin(&chosen_coin),
        native_recipient(&chosen_recipient),
    )?;
    Ok(frame.finish(types::Receipt {
        consumed: (),
        emitted: (),
        kept: coin,
    }))
}
#[test]
fn composed_intents_helpers_and_owned_reuse_preserve_private_and_event_order() {
    for selected in [false, true] {
        for value in [0, 42] {
            for user in [false, true] {
                let a = Witness::default();
                let b = Witness::default();
                let m = Witness::default();
                let native = c::compose(
                    context(),
                    &a,
                    input(value),
                    coin(value),
                    recipient(user),
                    selected,
                )
                .unwrap();
                let recorded = c::recorded::compose(
                    context(),
                    &b,
                    input(value),
                    coin(value),
                    recipient(user),
                    selected,
                )
                .unwrap();
                let manual = reference(context(), &m, selected, value, user).unwrap();
                assert_eq!(*a.calls.borrow(), [("coin", 10), ("recipient", 11)]);
                assert_eq!(*b.calls.borrow(), *a.calls.borrow());
                assert_eq!(*m.calls.borrow(), *a.calls.borrow());
                for execution in [&native, &recorded.execution] {
                    assert_eq!(execution.result, manual.execution.result);
                    assert_eq!(
                        execution.context.query.state,
                        manual.execution.context.query.state
                    );
                    assert_eq!(
                        execution.context.query.effects,
                        manual.execution.context.query.effects
                    );
                    assert_eq!(
                        execution.context.query.call_context.com_indices,
                        manual.execution.context.query.call_context.com_indices
                    );
                    assert_eq!(
                        execution.context.circuit_zswap(),
                        manual.execution.context.circuit_zswap()
                    );
                    assert_eq!(
                        execution.private_transcript_outputs,
                        manual.execution.private_transcript_outputs
                    );
                    assert_eq!(execution.gas_cost, manual.execution.gas_cost);
                    assert_eq!(execution.context.private_state, 12);
                }
                assert_eq!(native.context.circuit_zswap().inputs().len(), 2);
                assert_eq!(
                    native.context.circuit_zswap().outputs().len(),
                    if selected { 2 } else { 1 }
                );
                assert_eq!(
                    native.private_transcript_outputs.len(),
                    if selected { 6 } else { 5 }
                );
                assert!(recorded.public.verify_ops().is_empty());
                assert!(manual.public.verify_ops().is_empty());
            }
        }
    }
}
#[test]
fn witness_failures_stop_before_later_operands_without_claiming_partial_frames() {
    for fail_at in ["coin", "recipient"] {
        let a = Witness {
            fail_at: Some(fail_at),
            ..Default::default()
        };
        let b = Witness {
            fail_at: Some(fail_at),
            ..Default::default()
        };
        let m = Witness {
            fail_at: Some(fail_at),
            ..Default::default()
        };
        let expected = CompactError::AssertionFailed(format!("{fail_at} refused"));
        assert_eq!(
            c::compose(context(), &a, input(42), coin(42), recipient(true), false)
                .err()
                .unwrap(),
            expected
        );
        assert_eq!(
            c::recorded::compose(context(), &b, input(42), coin(42), recipient(true), false)
                .err()
                .unwrap(),
            expected
        );
        assert_eq!(
            reference(context(), &m, false, 42, true).err().unwrap(),
            expected
        );
        let calls = if fail_at == "coin" {
            vec![("coin", 10)]
        } else {
            vec![("coin", 10), ("recipient", 11)]
        };
        assert_eq!(*a.calls.borrow(), calls);
        assert_eq!(*b.calls.borrow(), calls);
        assert_eq!(*m.calls.borrow(), calls);
    }
}
#[test]
fn official_output_cursor_overflow_keeps_operand_evaluation_order() {
    for selected in [false, true] {
        let overflowing = || {
            let mut ctx = context();
            ctx.set_zswap_output_start(u64::MAX).unwrap();
            ctx
        };
        let a = Witness::default();
        let b = Witness::default();
        let m = Witness::default();
        assert_eq!(
            c::compose(
                overflowing(),
                &a,
                input(0),
                coin(0),
                recipient(true),
                selected
            )
            .err()
            .unwrap(),
            CompactError::ZswapCursorOverflow
        );
        assert_eq!(
            c::recorded::compose(
                overflowing(),
                &b,
                input(0),
                coin(0),
                recipient(true),
                selected
            )
            .err()
            .unwrap(),
            CompactError::ZswapCursorOverflow
        );
        assert_eq!(
            reference(overflowing(), &m, selected, 0, true)
                .err()
                .unwrap(),
            CompactError::ZswapCursorOverflow
        );
        let calls = if selected {
            vec![]
        } else {
            vec![("coin", 10), ("recipient", 11)]
        };
        assert_eq!(*a.calls.borrow(), calls);
        assert_eq!(*b.calls.borrow(), calls);
        assert_eq!(*m.calls.borrow(), calls);
    }
}
#[cfg(feature = "ledger-transaction")]
#[test]
fn ordinary_observation_refuses_output_allocation_before_later_witnesses() {
    use r::transaction::{Observation, ObservedContractState};
    let initial = c::initial_state(ConstructorContext::new(10)).unwrap();
    let observed = ObservedContractState::new(
        Default::default(),
        r::ledger::ContractState::new(
            initial.ledger_state.get_ref().clone(),
            Default::default(),
            Default::default(),
        ),
        Observation {
            transaction_hash: [0; 32],
            block_hash: [0; 32],
            block_height: 0,
        },
    );
    for selected in [false, true] {
        let a = Witness::default();
        let b = Witness::default();
        assert_eq!(
            c::compose(
                observed.circuit_context(10),
                &a,
                input(42),
                coin(42),
                recipient(true),
                selected
            )
            .err()
            .unwrap(),
            CompactError::ZswapAllocationLocked
        );
        assert_eq!(
            c::recorded::compose(
                observed.circuit_context(10),
                &b,
                input(42),
                coin(42),
                recipient(true),
                selected
            )
            .err()
            .unwrap(),
            CompactError::ZswapAllocationLocked
        );
        let calls = if selected {
            vec![]
        } else {
            vec![("coin", 10), ("recipient", 11)]
        };
        assert_eq!(*a.calls.borrow(), calls);
        assert_eq!(*b.calls.borrow(), calls);
    }
}
#[cfg(feature = "ledger-transaction")]
#[test]
fn intent_only_recording_retains_empty_transcript_preparation_refusal() {
    use midnight_serialize::{Deserializable, Serializable};
    use r::transaction::{CallSpec, PrepareCallError, VerifierKey, prepare_call};
    let recorded = c::recorded::compose(
        context(),
        &Witness::default(),
        input(0),
        coin(0),
        recipient(true),
        false,
    )
    .unwrap();
    let mut bytes = vec![];
    Vec::<u8>::new().serialize(&mut bytes).unwrap();
    // Invalid-key carrier tests only the earlier empty-program boundary.
    let carrier = VerifierKey::deserialize(&mut bytes.as_slice(), 0).unwrap();
    assert!(matches!(
        prepare_call(
            recorded,
            CallSpec::new(
                "compose",
                carrier,
                (input(0), coin(0), recipient(true), false),
                r::Field::from(0_u64)
            )
        ),
        Err(PrepareCallError::EmptyTranscript)
    ));
}
