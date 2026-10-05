// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Explicit final DAO state, not a funded proposal/voting lifecycle.
#[path = "advance.rs"]
pub(crate) mod advance;
pub use advance::Private;
use compact_rust_test_center_micro_dao_fixture::{ledger_slots as slots, types::*};
use midnight_compact_runtime as runtime;
use runtime::{CompactError, FixedBytes, context::CircuitContext};
pub struct Settings {
    pub phase: LedgerState,
    pub yes: u64,
    pub no: u64,
    pub round: u64,
    pub value: u128,
    pub empty_collections: bool,
    pub topic: String,
    pub absent: bool,
    pub wrong_recipient: bool,
    pub pot_flag: bool,
    pub missing_key: bool,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            phase: LedgerState::r#final,
            yes: 4,
            no: 3,
            round: 7,
            value: 99,
            empty_collections: false,
            topic: "Proposal 🗳️".into(),
            absent: false,
            wrong_recipient: false,
            pot_flag: true,
            missing_key: false,
        }
    }
}
pub fn seeded(s: &Settings) -> Result<CircuitContext<Private>, CompactError> {
    let mut ctx = advance::seeded(
        s.phase,
        s.yes,
        s.no,
        s.round,
        s.empty_collections,
        s.pot_flag,
    )?;
    ctx = slots::topic
        .write(
            ctx,
            Maybe {
                is_some: !s.topic.is_empty(),
                value: s.topic.clone().into(),
            },
        )?
        .context;
    ctx = slots::beneficiary
        .write(
            ctx,
            MaybeCompact1 {
                is_some: !s.absent,
                value: ZswapCoinPublicKey {
                    bytes: FixedBytes::new([if s.wrong_recipient { 8 } else { 7 }; 32]),
                },
            },
        )?
        .context;
    ctx = slots::pot
        .write(
            ctx,
            QualifiedShieldedCoinInfo {
                nonce: FixedBytes::new([41; 32]),
                color: FixedBytes::new([42; 32]),
                value: runtime::BoundedUint::new(s.value)?,
                mt_index: runtime::BoundedUint::new(0)?,
            },
        )?
        .context;
    if !s.missing_key {
        ctx = ctx.with_coin_public_key_bytes([7; 32]);
    }
    ctx.set_zswap_output_start(2)?;
    Ok(ctx)
}
