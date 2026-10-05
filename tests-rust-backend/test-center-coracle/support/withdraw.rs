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
//! Seeded prior game and qualified coins, without claiming a funded game lifecycle.
#![allow(
    dead_code,
    reason = "proof harness and execution cases use different controls"
)]
use compact_rust_test_center_coracle_fixture::{
    ledger_contract as c, ledger_slots as slots, pure_circuits, types::*,
};
use midnight_compact_runtime as runtime;
use runtime::context::{CircuitContext, ConstructorContext, WitnessContext};
use runtime::{CompactError, Field, FixedBytes};
use std::cell::RefCell;
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Private {
    pub calls: u64,
}
#[derive(Clone, Debug)]
pub struct Settings {
    pub red: bool,
    pub both_players: bool,
    pub impostor: bool,
    pub changed_secret: bool,
    pub witness_failure: bool,
    pub phase: State,
    pub key: Option<[u8; 32]>,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            red: true,
            both_players: false,
            impostor: false,
            changed_secret: false,
            witness_failure: false,
            phase: State::red_wins,
            key: Some([11; 32]),
        }
    }
}
pub struct Witness {
    pub calls: RefCell<Vec<&'static str>>,
    pub settings: Settings,
}
impl Witness {
    pub fn new(settings: Settings) -> Self {
        Self {
            calls: RefCell::new(vec![]),
            settings,
        }
    }
}
pub fn secret(red: bool) -> FixedBytes<32> {
    FixedBytes::new([if red { 7 } else { 8 }; 32])
}
pub fn coin(nonce: u8, value: u128, mt_index: u64) -> QualifiedShieldedCoinInfo {
    QualifiedShieldedCoinInfo {
        nonce: FixedBytes::new([nonce; 32]),
        color: FixedBytes::new([2; 32]),
        value: runtime::BoundedUint::new(value).unwrap(),
        mt_index: runtime::BoundedUint::new(u128::from(mt_index)).unwrap(),
    }
}
pub fn seeded(s: &Settings) -> Result<CircuitContext<Private>, CompactError> {
    let mut ctx = c::initial_state(ConstructorContext::new(Private::default()))?
        .into_circuit_context(runtime::ledger::ContractAddress::default());
    ctx = slots::red
        .write(ctx, pure_circuits::red_pk(secret(true))?)?
        .context;
    ctx = slots::blue
        .write(ctx, pure_circuits::blue_pk(secret(s.both_players))?)?
        .context;
    ctx = slots::state.write(ctx, s.phase)?.context;
    ctx = slots::pot.write(ctx, coin(3, 42, 0))?.context;
    ctx = slots::red_deposit.write(ctx, coin(4, 17, 1))?.context;
    ctx = slots::blue_deposit.write(ctx, coin(5, 19, 2))?.context;
    if let Some(key) = s.key {
        ctx = ctx.with_coin_public_key_bytes(key);
    }
    ctx.set_zswap_output_start(3)?;
    Ok(ctx)
}
impl c::TryWitnesses<Private> for Witness {
    fn local_secret_key(
        &self,
        ctx: WitnessContext<'_, Private, c::LedgerView<'_>>,
    ) -> Result<(Private, FixedBytes<32>), CompactError> {
        self.calls.borrow_mut().push("secret");
        if self.settings.witness_failure && self.calls.borrow().len() == 2 {
            return Err(CompactError::InvalidLedgerCell(
                "selected secret witness failed".into(),
            ));
        }
        let wrong = self.settings.impostor
            || (self.settings.changed_secret && self.calls.borrow().len() > 1);
        Ok((
            Private {
                calls: ctx.private_state.calls + 1,
            },
            if wrong {
                FixedBytes::new([9; 32])
            } else {
                secret(self.settings.red)
            },
        ))
    }
    fn local_board(
        &self,
        _: WitnessContext<'_, Private, c::LedgerView<'_>>,
    ) -> Result<(Private, Committable), CompactError> {
        panic!("unselected local_board")
    }
    fn local_set_board(
        &self,
        _: WitnessContext<'_, Private, c::LedgerView<'_>>,
        _: Committable,
    ) -> Result<(Private, ()), CompactError> {
        panic!("unselected local_set_board")
    }
    fn fresh_nonce(
        &self,
        _: WitnessContext<'_, Private, c::LedgerView<'_>>,
    ) -> Result<(Private, Field), CompactError> {
        panic!("unselected fresh_nonce")
    }
}
