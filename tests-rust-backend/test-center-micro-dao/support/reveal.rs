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
//! Explicit prior-state fixture; this does not execute a funded commit lifecycle.
#![allow(
    dead_code,
    reason = "execution tests cover rejection modes; proof uses valid paths"
)]
use compact_rust_test_center_micro_dao_fixture::{
    ledger_contract as c, ledger_slots as slots, types::*,
};
use midnight_compact_runtime as runtime;
use runtime::context::{CircuitContext, ConstructorContext, WitnessContext};
use runtime::{CompactError, Field, FixedBytes};
use std::cell::RefCell;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Private {
    pub phase: u8,
    pub vote: Option<bool>,
    pub calls: u64,
}
#[derive(Clone, Copy, Default)]
pub enum Mode {
    #[default]
    Normal,
    WrongRoot,
    WrongLeaf,
    Malformed,
}
pub struct Witness {
    pub calls: RefCell<Vec<&'static str>>,
    pub arguments: RefCell<Vec<[u8; 32]>>,
    pub mode: Mode,
    pub round: u64,
}
impl Witness {
    pub fn new(round: u64, mode: Mode) -> Self {
        Self {
            calls: RefCell::new(vec![]),
            arguments: RefCell::new(vec![]),
            mode,
            round,
        }
    }
    fn step(
        &self,
        name: &'static str,
        ctx: &WitnessContext<'_, Private, c::LedgerView<'_>>,
    ) -> Private {
        self.calls.borrow_mut().push(name);
        let mut p = ctx.private_state.clone();
        p.calls += 1;
        p
    }
}
pub fn commitment(vote: bool, round: u64) -> FixedBytes<32> {
    let mut ballot = [0; 32];
    let label: &[u8] = if vote { b"yes" } else { b"no" };
    ballot[..label.len()].copy_from_slice(label);
    let encoded = FixedBytes::<32>::new(Field::from(round).as_le_bytes().try_into().unwrap());
    runtime::persistent_hash((FixedBytes::new(ballot), encoded, FixedBytes::new([7; 32])))
}
pub fn seeded(
    round: u64,
    vote: bool,
    tree_round: Option<u64>,
) -> Result<CircuitContext<Private>, CompactError> {
    let private = Private {
        phase: 1,
        vote: Some(vote),
        calls: 0,
    };
    let mut ctx = c::initial_state(
        ConstructorContext::new(private),
        FixedBytes::new([4; 32]),
        Costs {
            seed_dust: runtime::BoundedUint::new(10)?,
            buy_in_dust: runtime::BoundedUint::new(3)?,
        },
    )?
    .into_circuit_context(runtime::ledger::ContractAddress::default());
    ctx = slots::state.write(ctx, LedgerState::reveal)?.context;
    ctx.query = runtime::ledger::write_cell(&ctx.query, 6, round, None, &ctx.cost_model)
        .map_err(|e| CompactError::LedgerQueryRejected(format!("{e:?}")))?
        .context;
    if let Some(tree_round) = tree_round {
        ctx = slots::committed_votes
            .insert(ctx, commitment(vote, tree_round))?
            .context;
        ctx = slots::committed_votes
            .insert(ctx, commitment(!vote, tree_round))?
            .context;
    }
    Ok(ctx)
}
impl c::TryWitnesses<Private> for Witness {
    fn local_secret_key(
        &self,
        ctx: WitnessContext<'_, Private, c::LedgerView<'_>>,
    ) -> Result<(Private, FixedBytes<32>), CompactError> {
        Ok((self.step("secret", &ctx), FixedBytes::new([7; 32])))
    }
    fn local_state(
        &self,
        ctx: WitnessContext<'_, Private, c::LedgerView<'_>>,
    ) -> Result<(Private, LocalState), CompactError> {
        let p = match ctx.private_state.phase {
            0 => LocalState::initial,
            1 => LocalState::committed,
            _ => LocalState::revealed,
        };
        Ok((self.step("state", &ctx), p))
    }
    fn local_advance_state(
        &self,
        ctx: WitnessContext<'_, Private, c::LedgerView<'_>>,
    ) -> Result<(Private, ()), CompactError> {
        let mut p = self.step("advance", &ctx);
        p.phase = 2;
        Ok((p, ()))
    }
    fn local_record_vote(
        &self,
        _ctx: WitnessContext<'_, Private, c::LedgerView<'_>>,
        _vote: bool,
    ) -> Result<(Private, ()), CompactError> {
        panic!("reveal must not record a ballot")
    }
    fn local_vote_cast(
        &self,
        ctx: WitnessContext<'_, Private, c::LedgerView<'_>>,
    ) -> Result<(Private, MaybeCompact3), CompactError> {
        let value = MaybeCompact3 {
            is_some: ctx.private_state.vote.is_some(),
            value: ctx.private_state.vote.unwrap_or(false),
        };
        Ok((self.step("vote", &ctx), value))
    }
    fn local_path_of_cm(
        &self,
        ctx: WitnessContext<'_, Private, c::LedgerView<'_>>,
        cm: FixedBytes<32>,
    ) -> Result<(Private, MaybeCompact2), CompactError> {
        let state = self.step("path", &ctx);
        self.arguments.borrow_mut().push(cm.0);
        let key = if matches!(self.mode, Mode::WrongLeaf) {
            commitment(!ctx.private_state.vote.unwrap(), self.round)
        } else {
            cm
        };
        // Local inspection of the actual readonly ledger view; no metered VM read,
        // matching the independent TS snapshot findPathForLeaf witness.
        let path = ctx.ledger.committed_votes()?.find_path_for_leaf(key);
        let mut result = if let Some(mut path) = path {
            if matches!(self.mode, Mode::Malformed) {
                path.path.pop();
            }
            MaybeCompact2 {
                is_some: true,
                value: MerkleTreePath::from_ledger_path(path)?,
            }
        } else {
            MaybeCompact2::default()
        };
        if matches!(self.mode, Mode::WrongRoot) {
            let mut path = result.value.path.into_array();
            path[0].sibling.field = path[0].sibling.field + Field::from(1u64);
            result.value.path = runtime::FixedVector::new(path);
        }
        Ok((state, result))
    }
}
