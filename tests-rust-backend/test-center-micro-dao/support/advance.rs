// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Explicit prior states; this is not a funded DAO lifecycle.
#![allow(
    dead_code,
    reason = "negative witness modes are used by execution tests, not the proof smoke"
)]
use compact_rust_test_center_micro_dao_fixture::{
    ledger_contract as c, ledger_slots as slots, types::*,
};
use midnight_compact_runtime as runtime;
use runtime::context::{CircuitContext, ConstructorContext, WitnessContext};
use runtime::{CompactError, FixedBytes};
use std::cell::RefCell;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Private {
    pub calls: u64,
}
#[derive(Clone, Copy, Default)]
pub enum Mode {
    #[default]
    Normal,
    Wrong,
    Failure,
    Malformed,
}
pub struct Witness {
    pub calls: RefCell<Vec<&'static str>>,
    pub mode: Mode,
}
impl Witness {
    pub fn new(mode: Mode) -> Self {
        Self {
            calls: RefCell::new(vec![]),
            mode,
        }
    }
}
pub fn pot() -> QualifiedShieldedCoinInfo {
    QualifiedShieldedCoinInfo {
        nonce: FixedBytes::new([41; 32]),
        color: FixedBytes::new([42; 32]),
        value: runtime::BoundedUint::new(99).unwrap(),
        mt_index: runtime::BoundedUint::new(17).unwrap(),
    }
}
pub fn seeded(
    phase: LedgerState,
    yes: u64,
    no: u64,
    round: u64,
    empty: bool,
    has_pot: bool,
) -> Result<CircuitContext<Private>, CompactError> {
    let mut ctx = c::initial_state(
        ConstructorContext::new(Private { calls: 0 }),
        FixedBytes::new([4; 32]),
        Costs {
            seed_dust: runtime::BoundedUint::new(10)?,
            buy_in_dust: runtime::BoundedUint::new(3)?,
        },
    )?
    .into_circuit_context(runtime::ledger::ContractAddress::default());
    ctx = slots::state.write(ctx, phase)?.context;
    for (index, value) in [(4, yes), (5, no), (6, round)] {
        ctx.query = runtime::ledger::write_cell(&ctx.query, index, value, None, &ctx.cost_model)
            .map_err(|e| CompactError::LedgerQueryRejected(format!("{e:?}")))?
            .context;
    }
    ctx = slots::pot.write(ctx, pot())?.context;
    ctx = slots::pot_has_coin.write(ctx, has_pot)?.context;
    if !empty {
        ctx = slots::topic
            .write(
                ctx,
                Maybe {
                    is_some: true,
                    value: "Round 🗳️".into(),
                },
            )?
            .context;
        ctx = slots::beneficiary
            .write(
                ctx,
                MaybeCompact1 {
                    is_some: true,
                    value: ZswapCoinPublicKey {
                        bytes: FixedBytes::new([31; 32]),
                    },
                },
            )?
            .context;
        for n in [11, 12] {
            ctx = slots::committed_votes
                .insert(ctx, FixedBytes::new([n; 32]))?
                .context;
        }
        for n in [21, 22] {
            ctx = slots::committed_participants
                .insert(ctx, FixedBytes::new([n; 32]))?
                .context;
            ctx = slots::revealed_participants
                .insert(ctx, FixedBytes::new([n; 32]))?
                .context;
        }
    }
    Ok(ctx)
}
impl c::TryWitnesses<Private> for Witness {
    fn local_secret_key(
        &self,
        ctx: WitnessContext<'_, Private, c::LedgerView<'_>>,
    ) -> Result<(Private, FixedBytes<32>), CompactError> {
        self.calls.borrow_mut().push("secret");
        match self.mode {
            Mode::Failure => {
                return Err(CompactError::AssertionFailed(
                    "advance witness refused".into(),
                ));
            }
            // FixedBytes statically excludes a 31-byte successful return. Model rejection
            // at the Rust witness boundary; do not pretend this is the TS dynamic error.
            Mode::Malformed => {
                return Err(CompactError::InvalidLedgerCell(
                    "advance witness Bytes<32>".into(),
                ));
            }
            _ => {}
        }
        Ok((
            Private {
                calls: ctx.private_state.calls + 1,
            },
            FixedBytes::new(
                [if matches!(self.mode, Mode::Wrong) {
                    7
                } else {
                    4
                }; 32],
            ),
        ))
    }
    fn local_state(
        &self,
        _: WitnessContext<'_, Private, c::LedgerView<'_>>,
    ) -> Result<(Private, LocalState), CompactError> {
        panic!("unused advance witness")
    }
    fn local_advance_state(
        &self,
        _: WitnessContext<'_, Private, c::LedgerView<'_>>,
    ) -> Result<(Private, ()), CompactError> {
        panic!("unused advance witness")
    }
    fn local_record_vote(
        &self,
        _: WitnessContext<'_, Private, c::LedgerView<'_>>,
        _: bool,
    ) -> Result<(Private, ()), CompactError> {
        panic!("unused advance witness")
    }
    fn local_vote_cast(
        &self,
        _: WitnessContext<'_, Private, c::LedgerView<'_>>,
    ) -> Result<(Private, MaybeCompact3), CompactError> {
        panic!("unused advance witness")
    }
    fn local_path_of_cm(
        &self,
        _: WitnessContext<'_, Private, c::LedgerView<'_>>,
        _: FixedBytes<32>,
    ) -> Result<(Private, MaybeCompact2), CompactError> {
        panic!("unused advance witness")
    }
}
