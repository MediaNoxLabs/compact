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

//! Direct command argument parsing and execution for contracts proof scenarios.
//!
//! Each handler retains its original argument checks and worker policy.

use std::error::Error;
use std::ffi::OsString;
use std::path::Path;

use crate::{
    bboard, coracle_concede, coracle_guess, coracle_start, coracle_withdraw, election_commit,
    election_reveal, election_topic, micro_dao_advance, micro_dao_buy_in, micro_dao_cash_out,
    micro_dao_reveal, micro_dao_set_topic, micro_dao_token, micro_dao_vote_commit,
    test_center_counter, test_center_welcome, zerocash_mint, zerocash_spend,
};

pub(super) fn test_center_counter(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --test-center-counter <proof-output>")?;
    if arguments.next().is_some() {
        return Err("usage: compact-rust-proof-smoke --test-center-counter <proof-output>".into());
    }
    test_center_counter::run(Path::new(&root))
}

pub(super) fn test_center_welcome(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --test-center-welcome <proof-output>")?;
    if arguments.next().is_some() {
        return Err("usage: compact-rust-proof-smoke --test-center-welcome <proof-output>".into());
    }
    test_center_welcome::run(Path::new(&root))
}

pub(super) fn test_center_welcome_organizers(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --test-center-welcome-organizers <proof-output>")?;
    if arguments.next().is_some() {
        return Err(
            "usage: compact-rust-proof-smoke --test-center-welcome-organizers <proof-output>"
                .into(),
        );
    }
    test_center_welcome::run_organizers(Path::new(&root))
}

pub(super) fn coracle_start(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --coracle-start <proof-output>")?;
    if arguments.next().is_some() {
        return Err("usage: compact-rust-proof-smoke --coracle-start <proof-output>".into());
    }
    let proof = std::thread::Builder::new()
        .name("coracle-start-proof".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || coracle_start::run(Path::new(&root)).map_err(|e| e.to_string()))?;
    match proof.join() {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(error.into()),
        Err(_) => Err("Coracle start proof thread panicked".into()),
    }
}

pub(super) fn coracle_concede(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --coracle-concede <proof-output>")?;
    if arguments.next().is_some() {
        return Err("usage: compact-rust-proof-smoke --coracle-concede <proof-output>".into());
    }
    let proof = std::thread::Builder::new()
        .name("coracle-concede-proof".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || coracle_concede::run(Path::new(&root)).map_err(|e| e.to_string()))?;
    match proof.join() {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(error.into()),
        Err(_) => Err("Coracle concede proof thread panicked".into()),
    }
}

pub(super) fn coracle_withdraw(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --coracle-withdraw <proof-output>")?;
    if arguments.next().is_some() {
        return Err("usage: compact-rust-proof-smoke --coracle-withdraw <proof-output>".into());
    }
    let proof = std::thread::Builder::new()
        .name("coracle-withdraw-proof".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || coracle_withdraw::run(Path::new(&root)).map_err(|e| e.to_string()))?;
    match proof.join() {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(error.into()),
        Err(_) => Err("Coracle withdraw proof thread panicked".into()),
    }
}

pub(super) fn coracle_guess(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --coracle-guess <proof-output>")?;
    if arguments.next().is_some() {
        return Err("usage: compact-rust-proof-smoke --coracle-guess <proof-output>".into());
    }
    let proof = std::thread::Builder::new()
        .name("coracle-guess-proof".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || coracle_guess::run(Path::new(&root)).map_err(|e| e.to_string()))?;
    match proof.join() {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(error.into()),
        Err(_) => Err("microDAO reveal proof thread panicked".into()),
    }
}

pub(super) fn micro_dao_buy_in(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --micro-dao-buy-in <proof-output>")?;
    if arguments.next().is_some() {
        return Err("unexpected argument".into());
    }
    micro_dao_buy_in::run(Path::new(&root))
}

pub(super) fn micro_dao_cash_out(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --micro-dao-cash-out <proof-output>")?;
    if arguments.next().is_some() {
        return Err("unexpected argument".into());
    }
    micro_dao_cash_out::run(Path::new(&root))
}

pub(super) fn micro_dao_vote_commit(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --micro-dao-vote-commit <proof-output>")?;
    if arguments.next().is_some() {
        return Err("unexpected extra argument".into());
    }
    micro_dao_vote_commit::run(Path::new(&root))
}

pub(super) fn micro_dao_set_topic(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --micro-dao-set-topic <proof-output>")?;
    if arguments.next().is_some() {
        return Err("unexpected argument".into());
    }
    micro_dao_set_topic::run(Path::new(&root))
}

pub(super) fn micro_dao_advance(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --micro-dao-advance <proof-output>")?;
    if arguments.next().is_some() {
        return Err("usage: compact-rust-proof-smoke --micro-dao-advance <proof-output>".into());
    }
    let proof = std::thread::Builder::new()
        .name("micro-dao-advance-proof".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || micro_dao_advance::run(Path::new(&root)).map_err(|e| e.to_string()))?;
    match proof.join() {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(error.into()),
        Err(_) => Err("microDAO advance proof thread panicked".into()),
    }
}

pub(super) fn micro_dao_reveal(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --micro-dao-reveal <proof-output>")?;
    if arguments.next().is_some() {
        return Err("usage: compact-rust-proof-smoke --micro-dao-reveal <proof-output>".into());
    }
    let proof = std::thread::Builder::new()
        .name("micro-dao-reveal-proof".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || micro_dao_reveal::run(Path::new(&root)).map_err(|e| e.to_string()))?;
    match proof.join() {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(error.into()),
        Err(_) => Err("microDAO reveal proof thread panicked".into()),
    }
}

pub(super) fn micro_dao_token(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --micro-dao-token <proof-output>")?;
    if arguments.next().is_some() {
        return Err("usage: compact-rust-proof-smoke --micro-dao-token <proof-output>".into());
    }
    let proof = std::thread::Builder::new()
        .name("micro-dao-token-proof".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || micro_dao_token::run(Path::new(&root)).map_err(|e| e.to_string()))?;
    match proof.join() {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(error.into()),
        Err(_) => Err("microDAO token proof thread panicked".into()),
    }
}

pub(super) fn zerocash_spend(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: --zerocash-spend <proof-output>")?;
    if arguments.next().is_some() {
        return Err("usage: --zerocash-spend <proof-output>".into());
    }
    zerocash_spend::run(Path::new(&root))
}

pub(super) fn bboard(arguments: &mut impl Iterator<Item = OsString>) -> Result<(), Box<dyn Error>> {
    let root = arguments.next().ok_or("usage: --bboard <proof-output>")?;
    if arguments.next().is_some() {
        return Err("usage: --bboard <proof-output>".into());
    }
    bboard::run(Path::new(&root))
}

pub(super) fn election_reveal(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: --election-reveal <proof-output>")?;
    if arguments.next().is_some() {
        return Err("usage: --election-reveal <proof-output>".into());
    }
    election_reveal::run(Path::new(&root))
}

pub(super) fn election_commit(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: --election-commit <proof-output>")?;
    if arguments.next().is_some() {
        return Err("usage: --election-commit <proof-output>".into());
    }
    election_commit::run(Path::new(&root))
}

pub(super) fn election_add_voter(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: --election-add-voter <proof-output>")?;
    if arguments.next().is_some() {
        return Err("usage: --election-add-voter <proof-output>".into());
    }
    election_topic::add_voter(Path::new(&root))
}

pub(super) fn election_advance(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: --election-advance <proof-output>")?;
    if arguments.next().is_some() {
        return Err("usage: --election-advance <proof-output>".into());
    }
    election_topic::advance(Path::new(&root))
}

pub(super) fn zerocash_mint(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: --zerocash-mint <proof-output>")?;
    if arguments.next().is_some() {
        return Err("usage: --zerocash-mint <proof-output>".into());
    }
    zerocash_mint::run(Path::new(&root))
}

pub(super) fn election_topic(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: --election-topic <proof-output>")?;
    if arguments.next().is_some() {
        return Err("usage: --election-topic <proof-output>".into());
    }
    election_topic::run(Path::new(&root))
}
