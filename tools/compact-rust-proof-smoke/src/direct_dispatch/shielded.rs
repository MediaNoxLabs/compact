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

//! Direct command argument parsing and execution for shielded proof scenarios.
//!
//! Each handler retains its original argument checks and worker policy.

use std::error::Error;
use std::ffi::{OsStr, OsString};
use std::path::Path;

use crate::{
    adt_set_qualified_coin_info, canonical_output_order, composite_zswap, kernel_shielded_effects,
    qualified_coin_cell, qualified_coin_set, shielded_merge, shielded_receive, shielded_send,
    transient_receive_send, wallet_funded_receive, zswap_transfer,
};

pub(super) fn zswap_transfer(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --zswap-transfer <proof-output>")?;
    if arguments.next().is_some() {
        return Err("unexpected extra arguments".into());
    }
    zswap_transfer::run(Path::new(&root))
}

pub(super) fn shielded_receive(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --shielded-receive <proof-output>")?;
    if arguments.next().is_some() {
        return Err("unexpected shielded receive proof argument".into());
    }
    shielded_receive::run(Path::new(&root))
}

pub(super) fn shielded_merge(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --shielded-merge <proof-output>")?;
    if arguments.next().is_some() {
        return Err("unexpected shielded merge proof argument".into());
    }
    shielded_merge::run(Path::new(&root))
}

pub(super) fn shielded_send(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --shielded-send <proof-output>")?;
    if arguments.next().is_some() {
        return Err("unexpected shielded send proof argument".into());
    }
    shielded_send::run(Path::new(&root))
}

pub(super) fn observational(
    first: Option<&OsStr>,
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("expected observational proof root")?;
    if arguments.next().is_some() {
        return Err("unexpected observational proof argument".into());
    }
    if first == Some(OsStr::new("--observational-receive")) {
        wallet_funded_receive::run_observational(Path::new(&root))
    } else {
        shielded_send::run_observational(Path::new(&root))
    }
}

pub(super) fn observational_shielded(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let receive = arguments.next().ok_or("expected receive proof root")?;
    let send = arguments.next().ok_or("expected send proof root")?;
    if arguments.next().is_some() {
        return Err("unexpected observation proof argument".into());
    }
    wallet_funded_receive::run_observational(Path::new(&receive))?;
    shielded_send::run_observational(Path::new(&send))
}

pub(super) fn wallet_funded_receive(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --wallet-funded-receive <proof-output>")?;
    if arguments.next().is_some() {
        return Err("unexpected wallet-funded receive proof argument".into());
    }
    wallet_funded_receive::run(Path::new(&root))
}

pub(super) fn kernel_shielded_effects(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --kernel-shielded-effects <proof-output>")?;
    if arguments.next().is_some() {
        return Err("unexpected kernel proof argument".into());
    }
    kernel_shielded_effects::run(Path::new(&root))
}

pub(super) fn qualified_coin_set(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --qualified-coin-set <proof-output>")?;
    if arguments.next().is_some() {
        return Err("usage: compact-rust-proof-smoke --qualified-coin-set <proof-output>".into());
    }
    qualified_coin_set::run(Path::new(&root))
}

pub(super) fn qualified_coin_set_chunked(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --qualified-coin-set-chunked <proof-output>")?;
    if arguments.next().is_some() {
        return Err(
            "usage: compact-rust-proof-smoke --qualified-coin-set-chunked <proof-output>".into(),
        );
    }
    crate::qualified_coin_set_chunked::run(Path::new(&root))
}

pub(super) fn qualified_coin_cell(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --qualified-coin-cell <proof-output>")?;
    if arguments.next().is_some() {
        return Err("usage: compact-rust-proof-smoke --qualified-coin-cell <proof-output>".into());
    }
    qualified_coin_cell::run(Path::new(&root))
}

pub(super) fn adt_set_qualified_coin_info(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --adt-set-qualified-coin-info <proof-output>")?;
    if arguments.next().is_some() {
        return Err(
            "usage: compact-rust-proof-smoke --adt-set-qualified-coin-info <proof-output>".into(),
        );
    }
    adt_set_qualified_coin_info::run(Path::new(&root))
}

pub(super) fn transient_receive_send(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --transient-receive-send <proof-output>")?;
    if arguments.next().is_some() {
        return Err(
            "usage: compact-rust-proof-smoke --transient-receive-send <proof-output>".into(),
        );
    }
    let proof = std::thread::Builder::new()
        .name("transient-receive-send-proof".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || transient_receive_send::run(Path::new(&root)).map_err(|e| e.to_string()))?;
    match proof.join() {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(error.into()),
        Err(_) => Err("transient receive/send proof thread panicked".into()),
    }
}

pub(super) fn canonical_output_order(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --canonical-output-order <proof-output>")?;
    if arguments.next().is_some() {
        return Err(
            "usage: compact-rust-proof-smoke --canonical-output-order <proof-output>".into(),
        );
    }
    let proof = std::thread::Builder::new()
        .name("canonical-output-order-proof".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || canonical_output_order::run(Path::new(&root)).map_err(|e| e.to_string()))?;
    match proof.join() {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(error.into()),
        Err(_) => Err("terminal lexical return proof thread panicked".into()),
    }
}

pub(super) fn composite_zswap(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments.next().ok_or(
        "usage: compact-rust-proof-smoke --composite-zswap <original-proof> <transfer-proof>",
    )?;
    let transfer = arguments.next().ok_or("missing transfer proof directory")?;
    if arguments.next().is_some() {
        return Err("unexpected composite Zswap argument".into());
    }
    let proof = std::thread::Builder::new()
        .name("composite-zswap-proof".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            composite_zswap::run(Path::new(&root), Path::new(&transfer)).map_err(|e| e.to_string())
        })?;
    match proof.join() {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(error.into()),
        Err(_) => Err("composite Zswap proof thread panicked".into()),
    }
}
