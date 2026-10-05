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

//! Shared live-ledger witness adapter for execution and proof tests.
#![allow(
    dead_code,
    reason = "execution tests exercise rejection modes while proof tests use valid paths"
)]
use compact_rust_election_oracle_fixture::{ledger_contract as contract, types::*};
use midnight_compact_runtime as runtime;
use runtime::context::{CircuitContext, ConstructorContext, WitnessContext};
use runtime::{CompactError, FixedBytes};
use std::cell::RefCell;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Private {
    pub phase: u8,
    pub ballot: u8,
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

#[derive(Default)]
pub struct Witness {
    pub calls: RefCell<Vec<&'static str>>,
    pub mode: Mode,
}

impl Witness {
    fn step(
        &self,
        name: &'static str,
        context: &WitnessContext<'_, Private, contract::LedgerView<'_>>,
    ) -> Private {
        self.calls.borrow_mut().push(name);
        let mut state = context.private_state.clone();
        state.calls += 1;
        state
    }
}

impl contract::TryWitnesses<Private> for Witness {
    fn private_secret_key(
        &self,
        context: WitnessContext<'_, Private, contract::LedgerView<'_>>,
    ) -> Result<(Private, FixedBytes<32>), CompactError> {
        Ok((self.step("secret", &context), FixedBytes::new([7; 32])))
    }
    fn private_state(
        &self,
        context: WitnessContext<'_, Private, contract::LedgerView<'_>>,
    ) -> Result<(Private, PrivateState), CompactError> {
        let phase = match context.private_state.phase {
            0 => PrivateState::initial,
            1 => PrivateState::committed,
            _ => PrivateState::revealed,
        };
        Ok((self.step("state", &context), phase))
    }
    fn private_state_advance(
        &self,
        context: WitnessContext<'_, Private, contract::LedgerView<'_>>,
    ) -> Result<(Private, ()), CompactError> {
        let mut state = self.step("advance", &context);
        state.phase += 1;
        Ok((state, ()))
    }
    fn private_vote_record(
        &self,
        context: WitnessContext<'_, Private, contract::LedgerView<'_>>,
        vote: PermissibleVotes,
    ) -> Result<(Private, ()), CompactError> {
        let mut state = self.step("record", &context);
        state.ballot = match vote {
            PermissibleVotes::yes => 0,
            PermissibleVotes::no => 1,
        };
        Ok((state, ()))
    }
    fn private_vote(
        &self,
        _context: WitnessContext<'_, Private, contract::LedgerView<'_>>,
    ) -> Result<(Private, PermissibleVotes), CompactError> {
        panic!("unexpected vote read")
    }
    fn context_eligible_voters_path_of(
        &self,
        context: WitnessContext<'_, Private, contract::LedgerView<'_>>,
        pk: FixedBytes<32>,
    ) -> Result<(Private, MaybeCompact1), CompactError> {
        let state = self.step("path", &context);
        let key = if matches!(self.mode, Mode::WrongLeaf) {
            FixedBytes::new([8; 32])
        } else {
            pk
        };
        let path = context.ledger.eligible_voters()?.find_path_for_leaf(key);
        let mut result = if let Some(mut path) = path {
            if matches!(self.mode, Mode::Malformed) {
                path.path.pop();
            }
            MaybeCompact1 {
                is_some: true,
                value: MerkleTreePath::from_ledger_path(path)?,
            }
        } else {
            MaybeCompact1::default()
        };
        if matches!(self.mode, Mode::WrongRoot) {
            let mut entries = result.value.path.into_array();
            entries[0].sibling.field = entries[0].sibling.field + runtime::Field::from(1_u64);
            result.value.path = runtime::FixedVector::new(entries);
        }
        Ok((state, result))
    }
    fn context_committed_votes_path_of(
        &self,
        _context: WitnessContext<'_, Private, contract::LedgerView<'_>>,
        _cm: FixedBytes<32>,
    ) -> Result<(Private, MaybeCompact1), CompactError> {
        panic!("unexpected commitment path")
    }
}

pub fn authority() -> FixedBytes<32> {
    let mut prefix = [0; 32];
    prefix[..18].copy_from_slice(b"lares:election:pk:");
    runtime::persistent_hash((FixedBytes::new(prefix), FixedBytes::new([7; 32])))
}

pub fn seeded(
    no_voter: bool,
    other_voter: bool,
    no_advance: bool,
) -> Result<CircuitContext<Private>, CompactError> {
    let witness = Witness::default();
    let initial =
        contract::initial_state(ConstructorContext::new(Private::default()), authority())?;
    let mut context = initial.into_circuit_context(runtime::ledger::ContractAddress::default());
    context = contract::set_topic(context, &witness, "vote".into())?.context;
    if !no_voter {
        context = contract::add_voter(
            context,
            &witness,
            if other_voter {
                FixedBytes::new([8; 32])
            } else {
                authority()
            },
        )?
        .context;
    }
    if !no_advance {
        context = contract::advance(context, &witness)?.context;
    }
    context.private_state = Private::default();
    Ok(context)
}
