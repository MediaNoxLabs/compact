// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Explicit commit-phase state; prior token issuance is not proved here.
use compact_rust_test_center_micro_dao_fixture::{
    ledger_contract as c, ledger_slots as slots, types::*,
};
use midnight_compact_runtime as runtime;
use runtime::context::{CircuitContext, ConstructorContext, WitnessContext};
use runtime::{CompactError, Field, FixedBytes};
use serde_json::{Value, json};
use std::cell::RefCell;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Private {
    pub phase: u8,
    pub vote: Option<bool>,
    pub calls: u64,
}
pub struct Witness {
    pub calls: RefCell<Vec<Value>>,
    pub mode: String,
}
impl Witness {
    pub fn new(mode: &str) -> Self {
        Self {
            calls: RefCell::new(vec![]),
            mode: mode.into(),
        }
    }
    fn step(
        &self,
        name: &str,
        ctx: &WitnessContext<'_, Private, c::LedgerView<'_>>,
    ) -> Result<Private, CompactError> {
        self.calls.borrow_mut().push(json!(name));
        if self.mode == format!("{name}Failure") {
            return Err(CompactError::InvalidLedgerCell(format!(
                "{name} witness refused"
            )));
        }
        let mut p = ctx.private_state.clone();
        p.calls += 1;
        Ok(p)
    }
}
pub fn participant(round: u64) -> FixedBytes<32> {
    let mut prefix = [0; 32];
    let label = b"lares:udao:cm-nul:";
    prefix[..label.len()].copy_from_slice(label);
    let encoded = FixedBytes::<32>::new(Field::from(round).as_le_bytes().try_into().unwrap());
    runtime::persistent_hash((FixedBytes::new(prefix), encoded, FixedBytes::new([4; 32])))
}
pub fn commitment(ballot: bool, round: u64) -> FixedBytes<32> {
    let mut prefix = [0; 32];
    let label: &[u8] = if ballot { b"yes" } else { b"no" };
    prefix[..label.len()].copy_from_slice(label);
    let encoded = FixedBytes::<32>::new(Field::from(round).as_le_bytes().try_into().unwrap());
    runtime::persistent_hash((FixedBytes::new(prefix), encoded, FixedBytes::new([4; 32])))
}
pub fn seeded(
    phase: LedgerState,
    round: u64,
    private_phase: u8,
    duplicate: bool,
    full_tree: bool,
) -> Result<CircuitContext<Private>, CompactError> {
    let mut ctx = c::initial_state(
        ConstructorContext::new(Private {
            phase: private_phase,
            vote: None,
            calls: 0,
        }),
        FixedBytes::new([4; 32]),
        Costs {
            seed_dust: runtime::BoundedUint::new(10)?,
            buy_in_dust: runtime::BoundedUint::new(3)?,
        },
    )?
    .into_circuit_context(runtime::ledger::ContractAddress::default());
    ctx = slots::state.write(ctx, phase)?.context;
    ctx.query = runtime::ledger::write_cell(&ctx.query, 6, round, None, &ctx.cost_model)
        .map_err(|e| CompactError::LedgerQueryRejected(format!("{e:?}")))?
        .context;
    if duplicate {
        ctx = slots::committed_participants
            .insert(ctx, participant(round))?
            .context;
    }
    if full_tree {
        ctx.query = runtime::ledger::write_cell_at_path(
            &ctx.query,
            &[7, 1],
            1024u64,
            None,
            &ctx.cost_model,
        )
        .map_err(|e| CompactError::LedgerQueryRejected(format!("{e:?}")))?
        .context;
    }
    ctx = ctx.with_coin_public_key_bytes([7; 32]);
    ctx.set_zswap_output_start(2)?;
    Ok(ctx)
}
impl c::TryWitnesses<Private> for Witness {
    fn local_state(
        &self,
        ctx: WitnessContext<'_, Private, c::LedgerView<'_>>,
    ) -> Result<(Private, LocalState), CompactError> {
        let phase = match ctx.private_state.phase {
            0 => LocalState::initial,
            1 => LocalState::committed,
            _ => LocalState::revealed,
        };
        Ok((self.step("state", &ctx)?, phase))
    }
    fn local_secret_key(
        &self,
        ctx: WitnessContext<'_, Private, c::LedgerView<'_>>,
    ) -> Result<(Private, FixedBytes<32>), CompactError> {
        let p = self.step("secret", &ctx)?;
        if self.mode == "secretMalformed" {
            return Err(CompactError::InvalidLedgerCell(
                "secret witness Bytes<32>".into(),
            ));
        }
        Ok((p, FixedBytes::new([4; 32])))
    }
    fn local_record_vote(
        &self,
        ctx: WitnessContext<'_, Private, c::LedgerView<'_>>,
        vote: bool,
    ) -> Result<(Private, ()), CompactError> {
        self.calls.borrow_mut().push(json!(["record", vote]));
        if self.mode == "recordFailure" {
            return Err(CompactError::InvalidLedgerCell(
                "record witness refused".into(),
            ));
        }
        let mut p = ctx.private_state.clone();
        p.calls += 1;
        p.vote = Some(vote);
        Ok((p, ()))
    }
    fn local_advance_state(
        &self,
        ctx: WitnessContext<'_, Private, c::LedgerView<'_>>,
    ) -> Result<(Private, ()), CompactError> {
        let mut p = self.step("advance", &ctx)?;
        p.phase += 1;
        Ok((p, ()))
    }
    fn local_vote_cast(
        &self,
        _: WitnessContext<'_, Private, c::LedgerView<'_>>,
    ) -> Result<(Private, MaybeCompact3), CompactError> {
        panic!("commit must not read ballot")
    }
    fn local_path_of_cm(
        &self,
        _: WitnessContext<'_, Private, c::LedgerView<'_>>,
        _: FixedBytes<32>,
    ) -> Result<(Private, MaybeCompact2), CompactError> {
        panic!("commit must not request path")
    }
}
