// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Manual reference uses official recording and slot APIs, independent of emission.
use crate::{ledger_contract as c, ledger_slots as slots, runtime as r, types};
use r::CompactError;
use r::context::{CircuitContext, ConstructorContext, WitnessContext};
use r::recording::{RecordedCircuitResult, RecordingFrame};
use std::cell::RefCell;

#[derive(Default)]
pub struct Witness {
    pub calls: RefCell<Vec<(&'static str, u64, u128)>>,
    pub coin: types::ShieldedCoinInfo,
    pub recipient: types::Either,
    pub refuse: Option<&'static str>,
}
impl Witness {
    pub fn observe(
        &self,
        tag: &'static str,
        private: u64,
        size: u128,
    ) -> Result<u64, CompactError> {
        self.calls.borrow_mut().push((tag, private, size));
        if self.refuse == Some(tag) {
            return Err(CompactError::AssertionFailed(format!("{tag} refused")));
        }
        Ok(private + 1)
    }
}
impl c::TryWitnesses<u64> for Witness {
    fn nextCoin(
        &self,
        ctx: WitnessContext<'_, u64, c::LedgerView<'_>>,
    ) -> Result<(u64, types::ShieldedCoinInfo), CompactError> {
        Ok((
            self.observe(
                "coin",
                *ctx.private_state,
                ctx.ledger.coins()?.size()?.value(),
            )?,
            self.coin.clone(),
        ))
    }
    fn nextRecipient(
        &self,
        ctx: WitnessContext<'_, u64, c::LedgerView<'_>>,
    ) -> Result<(u64, types::Either), CompactError> {
        Ok((
            self.observe(
                "recipient",
                *ctx.private_state,
                ctx.ledger.coins()?.size()?.value(),
            )?,
            self.recipient.clone(),
        ))
    }
}
pub fn coin(value: u128) -> types::ShieldedCoinInfo {
    types::ShieldedCoinInfo {
        nonce: r::FixedBytes::new([3; 32]),
        color: r::FixedBytes::new([5; 32]),
        value: r::BoundedUint::new(value).unwrap(),
    }
}
pub fn recipient(user: bool) -> types::Either {
    // Both inactive arms are deliberately nonzero: canonical types impose no
    // value-level zeroing requirement on the inactive recipient arm.
    types::Either {
        is_left: user,
        left: types::ZswapCoinPublicKey {
            bytes: r::FixedBytes::new([7; 32]),
        },
        right: types::ContractAddress {
            bytes: r::FixedBytes::new([9; 32]),
        },
    }
}
pub fn carriers(
    coin: &types::ShieldedCoinInfo,
    recipient: &types::Either,
) -> (r::ledger::CoinInfo, r::ledger::CoinRecipient) {
    (
        r::ledger::coin_info_from_compact(coin.nonce, coin.color, coin.value.value()),
        r::ledger::coin_recipient_from_compact(
            recipient.is_left,
            recipient.left.bytes,
            recipient.right.bytes,
        ),
    )
}
pub fn context() -> CircuitContext<u64> {
    c::initial_state(ConstructorContext::new(7))
        .unwrap()
        .into_circuit_context(Default::default())
}
pub fn allocate(
    mut context: CircuitContext<u64>,
    coin: &types::ShieldedCoinInfo,
    recipient: &types::Either,
    index: u64,
) -> CircuitContext<u64> {
    let (coin, recipient) = carriers(coin, recipient);
    context.query.call_context.com_indices = context
        .query
        .call_context
        .com_indices
        .insert(coin.commitment(&recipient), index);
    context
}
pub fn qualified(coin: &types::ShieldedCoinInfo, index: u64) -> types::QualifiedShieldedCoinInfo {
    types::QualifiedShieldedCoinInfo {
        nonce: coin.nonce,
        color: coin.color,
        value: coin.value,
        mt_index: r::BoundedUint::new(index.into()).unwrap(),
    }
}
pub fn witness_coin() -> types::ShieldedCoinInfo {
    let mut value = coin(43);
    value.nonce = r::FixedBytes::new([4; 32]);
    value
}
fn insert(
    frame: RecordingFrame<u64>,
    coin: &types::ShieldedCoinInfo,
    recipient: &types::Either,
) -> Result<RecordingFrame<u64>, CompactError> {
    let (coin, recipient) = carriers(coin, recipient);
    slots::coins.record_insert_coin(frame, coin, recipient)
}
pub fn insert_once(
    context: CircuitContext<u64>,
    coin: &types::ShieldedCoinInfo,
    recipient: &types::Either,
) -> Result<RecordedCircuitResult<u64, ()>, CompactError> {
    Ok(insert(RecordingFrame::new(context), coin, recipient)?.finish(()))
}
pub fn insert_selected(
    context: CircuitContext<u64>,
    coin: &types::ShieldedCoinInfo,
    recipient: &types::Either,
    selected: bool,
    witness: &Witness,
) -> Result<RecordedCircuitResult<u64, ()>, CompactError> {
    let mut frame = insert(RecordingFrame::new(context), coin, recipient)?;
    if selected {
        let (next, observed_coin) = frame.try_witness_metered(|ctx, meter| {
            let size = slots::coins.witness_view(meter)?.size()?.value();
            Ok((
                witness.observe("coin", ctx.private_state, size)?,
                witness.coin.clone(),
            ))
        })?;
        let (next, observed_recipient) = next.try_witness_metered(|ctx, meter| {
            let size = slots::coins.witness_view(meter)?.size()?.value();
            Ok((
                witness.observe("recipient", ctx.private_state, size)?,
                witness.recipient.clone(),
            ))
        })?;
        frame = insert(next, &observed_coin, &observed_recipient)?;
        frame = insert(frame, &observed_coin, &observed_recipient)?;
    }
    Ok(insert(frame, coin, recipient)?.finish(()))
}
