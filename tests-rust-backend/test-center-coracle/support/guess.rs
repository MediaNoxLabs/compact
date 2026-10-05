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
//! Explicit prior-state fixture; no funded start/payout lifecycle is implied.
#![allow(
    dead_code,
    reason = "execution tests cover rejection modes; proof uses valid paths"
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
    pub board: u64,
    pub empty: bool,
    pub both_players: bool,
    pub first_blue: bool,
    pub wrong_turn: bool,
    pub dead: bool,
    pub impostor: bool,
    pub changed_secret: bool,
    pub wrong_nonce: bool,
    pub invalid_board: bool,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            red: true,
            board: 1,
            empty: false,
            both_players: false,
            first_blue: false,
            wrong_turn: false,
            dead: false,
            impostor: false,
            changed_secret: false,
            wrong_nonce: false,
            invalid_board: false,
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
    fn step(
        &self,
        name: &'static str,
        ctx: &WitnessContext<'_, Private, c::LedgerView<'_>>,
    ) -> Private {
        self.calls.borrow_mut().push(name);
        Private {
            calls: ctx.private_state.calls + 1,
        }
    }
}
pub fn secret(red: bool) -> FixedBytes<32> {
    FixedBytes::new([if red { 7 } else { 8 }; 32])
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
    let commitment = Commitment {
        value: runtime::transient_commit(
            Board {
                position: Field::from(s.board),
            },
            Field::from(11u64),
        ),
    };
    ctx = slots::red_board.write(ctx, commitment.clone())?.context;
    ctx = slots::blue_board.write(ctx, commitment)?.context;
    ctx = slots::last_guess
        .write(
            ctx,
            Maybe {
                is_some: !s.empty,
                value: Field::from(if s.empty {
                    0
                } else if s.dead {
                    s.board
                } else {
                    5
                }),
            },
        )?
        .context;
    let phase = if s.wrong_turn {
        if s.red {
            State::blue_turn
        } else {
            State::red_turn
        }
    } else if s.first_blue {
        State::blue_started
    } else if s.red {
        State::red_turn
    } else {
        State::blue_turn
    };
    Ok(slots::state.write(ctx, phase)?.context)
}
impl c::TryWitnesses<Private> for Witness {
    fn local_secret_key(
        &self,
        ctx: WitnessContext<'_, Private, c::LedgerView<'_>>,
    ) -> Result<(Private, FixedBytes<32>), CompactError> {
        let p = self.step("secret", &ctx);
        let wrong = self.settings.impostor
            || (self.settings.changed_secret && self.calls.borrow().len() > 1);
        Ok((
            p,
            if wrong {
                FixedBytes::new([9; 32])
            } else {
                secret(self.settings.red)
            },
        ))
    }
    fn local_board(
        &self,
        ctx: WitnessContext<'_, Private, c::LedgerView<'_>>,
    ) -> Result<(Private, Committable), CompactError> {
        Ok((
            self.step("board", &ctx),
            Committable {
                nonce: Field::from(if self.settings.wrong_nonce { 12u64 } else { 11 }),
                contents: Board {
                    position: Field::from(if self.settings.invalid_board {
                        0
                    } else {
                        self.settings.board
                    }),
                },
            },
        ))
    }
    fn local_set_board(
        &self,
        _: WitnessContext<'_, Private, c::LedgerView<'_>>,
        _: Committable,
    ) -> Result<(Private, ()), CompactError> {
        panic!("unselected local_set_board called")
    }
    fn fresh_nonce(
        &self,
        _: WitnessContext<'_, Private, c::LedgerView<'_>>,
    ) -> Result<(Private, Field), CompactError> {
        panic!("unselected fresh_nonce called")
    }
}
