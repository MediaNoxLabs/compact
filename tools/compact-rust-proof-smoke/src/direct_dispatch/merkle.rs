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

//! Direct command argument parsing and execution for merkle proof scenarios.
//!
//! Each handler retains its original argument checks and worker policy.

use std::error::Error;
use std::ffi::OsString;
use std::path::Path;

use crate::{merkle_hash, merkle_indexed, merkle_reset, merkle_root, merkle_verify};

pub(super) fn merkle_verify(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --merkle-verify <compiled-contract-output>")?;
    if arguments.next().is_some() {
        return Err(
            "usage: compact-rust-proof-smoke --merkle-verify <compiled-contract-output>".into(),
        );
    }
    merkle_verify::run(Path::new(&root))
}

pub(super) fn merkle_reset(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --merkle-reset <proof-output>")?;
    if arguments.next().is_some() {
        return Err("usage: compact-rust-proof-smoke --merkle-reset <proof-output>".into());
    }
    merkle_reset::run(Path::new(&root))
}

pub(super) fn merkle_root(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --merkle-root <proof-output>")?;
    if arguments.next().is_some() {
        return Err("usage: compact-rust-proof-smoke --merkle-root <proof-output>".into());
    }
    merkle_root::run(Path::new(&root))
}

pub(super) fn merkle_hash(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --merkle-hash <proof-output>")?;
    if arguments.next().is_some() {
        return Err("usage: compact-rust-proof-smoke --merkle-hash <proof-output>".into());
    }
    let proof = std::thread::Builder::new()
        .name("merkle-hash-proof".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || merkle_hash::run(Path::new(&root)).map_err(|e| e.to_string()))?;
    match proof.join() {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(error.into()),
        Err(_) => Err("Merkle hash proof thread panicked".into()),
    }
}

pub(super) fn historic_merkle_hash(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --historic-merkle-hash <proof-output>")?;
    if arguments.next().is_some() {
        return Err("usage: compact-rust-proof-smoke --historic-merkle-hash <proof-output>".into());
    }
    let proof = std::thread::Builder::new()
        .name("historic-merkle-hash-proof".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || merkle_hash::run_historic(Path::new(&root)).map_err(|e| e.to_string()))?;
    match proof.join() {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(error.into()),
        Err(_) => Err("historic Merkle hash proof thread panicked".into()),
    }
}

pub(super) fn historic_merkle_indexed_hash(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --historic-merkle-indexed-hash <proof-output>")?;
    if arguments.next().is_some() {
        return Err(
            "usage: compact-rust-proof-smoke --historic-merkle-indexed-hash <proof-output>".into(),
        );
    }
    let proof = std::thread::Builder::new()
        .name("historic-merkle-indexed-hash-proof".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            merkle_hash::run_historic_indexed(Path::new(&root)).map_err(|e| e.to_string())
        })?;
    match proof.join() {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(error.into()),
        Err(_) => Err("historic Merkle indexed hash proof thread panicked".into()),
    }
}

pub(super) fn historic_merkle_reset_history(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --historic-merkle-reset-history <proof-output>")?;
    if arguments.next().is_some() {
        return Err(
            "usage: compact-rust-proof-smoke --historic-merkle-reset-history <proof-output>".into(),
        );
    }
    let proof = std::thread::Builder::new()
        .name("historic-merkle-reset-history-proof".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            merkle_hash::run_historic_reset_history(Path::new(&root)).map_err(|e| e.to_string())
        })?;
    match proof.join() {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(error.into()),
        Err(_) => Err("historic Merkle reset-history proof thread panicked".into()),
    }
}

pub(super) fn historic_merkle_reset_tree(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --historic-merkle-reset-tree <proof-output>")?;
    if arguments.next().is_some() {
        return Err(
            "usage: compact-rust-proof-smoke --historic-merkle-reset-tree <proof-output>".into(),
        );
    }
    let proof = std::thread::Builder::new()
        .name("historic-merkle-reset-tree-proof".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            merkle_hash::run_historic_reset_tree(Path::new(&root)).map_err(|e| e.to_string())
        })?;
    match proof.join() {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(error.into()),
        Err(_) => Err("historic Merkle tree reset proof thread panicked".into()),
    }
}

pub(super) fn historic_merkle_known(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --historic-merkle-known <proof-output>")?;
    if arguments.next().is_some() {
        return Err(
            "usage: compact-rust-proof-smoke --historic-merkle-known <proof-output>".into(),
        );
    }
    let proof = std::thread::Builder::new()
        .name("historic-merkle-known-proof".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            merkle_hash::run_historic_known(Path::new(&root)).map_err(|e| e.to_string())
        })?;
    match proof.join() {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(error.into()),
        Err(_) => Err("historic Merkle known proof thread panicked".into()),
    }
}

pub(super) fn merkle_indexed(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --merkle-indexed <proof-output>")?;
    if arguments.next().is_some() {
        return Err("usage: compact-rust-proof-smoke --merkle-indexed <proof-output>".into());
    }
    let proof = std::thread::Builder::new()
        .name("merkle-indexed-proof".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || merkle_indexed::run(Path::new(&root)).map_err(|e| e.to_string()))?;
    match proof.join() {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(error.into()),
        Err(_) => Err("indexed Merkle proof thread panicked".into()),
    }
}

pub(super) fn historic_merkle_indexed(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let insert_root = arguments.next().ok_or(
        "usage: compact-rust-proof-smoke --historic-merkle-indexed <insert-output> <default-output>",
    )?;
    let default_root = arguments.next().ok_or(
        "usage: compact-rust-proof-smoke --historic-merkle-indexed <insert-output> <default-output>",
    )?;
    if arguments.next().is_some() {
        return Err("usage: compact-rust-proof-smoke --historic-merkle-indexed <insert-output> <default-output>".into());
    }
    let proof = std::thread::Builder::new()
        .name("historic-merkle-indexed-proof".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            merkle_indexed::run_historic(Path::new(&insert_root), Path::new(&default_root))
                .map_err(|e| e.to_string())
        })?;
    match proof.join() {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(error.into()),
        Err(_) => Err("historic indexed Merkle proof thread panicked".into()),
    }
}
