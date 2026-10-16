// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Source oracle using official ledger operations and a separately evaluated witness.
use crate::{ledger_contract as c, ledger_slots, runtime as r, types};
use r::context::{CircuitContext, WitnessContext};
use r::recording::{RecordedCircuitResult, RecordingFrame};
use r::{BoundedUint, CompactError};
use std::cell::RefCell;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WitnessInput {
    pub private: u64,
    pub value: u128,
    pub accepted: bool,
}
#[derive(Default)]
pub struct Witness {
    pub calls: RefCell<Vec<WitnessInput>>,
    pub selected: bool,
    pub refuse: bool,
}
impl Witness {
    fn evaluate(&self, input: WitnessInput) -> Result<(u64, bool), CompactError> {
        self.calls.borrow_mut().push(input.clone());
        if self.refuse {
            return Err(CompactError::AssertionFailed(
                "receive choice refused".into(),
            ));
        }
        Ok((input.private + 1, self.selected))
    }
}
impl c::TryWitnesses<u64> for Witness {
    fn choose(
        &self,
        ctx: WitnessContext<'_, u64, c::LedgerView<'_>>,
        value: BoundedUint<{ u128::MAX }>,
    ) -> Result<(u64, bool), CompactError> {
        self.evaluate(WitnessInput {
            private: *ctx.private_state,
            value: value.value(),
            accepted: ctx.ledger.accepted()?,
        })
    }
}
pub fn reference(
    ctx: CircuitContext<u64>,
    witness: &Witness,
    coin: types::ShieldedCoinInfo,
) -> Result<RecordedCircuitResult<u64, ()>, CompactError> {
    let frame = ledger_slots::accepted.record_write(RecordingFrame::new(ctx), false)?;
    let (mut frame, selected) = frame.try_witness_metered(|ctx, meter| {
        witness.evaluate(WitnessInput {
            private: ctx.private_state,
            value: coin.value.value(),
            accepted: ledger_slots::accepted.witness_read(meter)?,
        })
    })?;
    if selected {
        let (next, address) = frame.kernel_self()?;
        let native_coin =
            r::ledger::coin_info_from_compact(coin.nonce, coin.color, coin.value.value());
        let recipient = r::ledger::CoinRecipient::Contract(address);
        let commitment = native_coin.commitment(&recipient);
        frame = next.create_zswap_output(native_coin, recipient)?;
        frame = frame.kernel_claim(r::ledger::KernelClaim::CoinReceive(commitment))?;
        frame = ledger_slots::accepted.record_write(frame, true)?;
    }
    Ok(frame.finish(()))
}
