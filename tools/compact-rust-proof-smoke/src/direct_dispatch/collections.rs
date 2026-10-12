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

//! Direct command argument parsing and execution for collections proof scenarios.
//!
//! Each handler retains its original argument checks and worker policy.

use std::error::Error;
use std::ffi::OsString;
use std::path::Path;

use crate::{
    adt_list_bytes, adt_list_enum, adt_list_field, adt_list_vector_field_4, adt_set_enum,
    adt_set_vector, check_conditional_set_proof, opaque_string_map, opaque_string_set, vector_map,
};

pub(super) fn opaque_string_set(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --opaque-string-set <proof-output>")?;
    if arguments.next().is_some() {
        return Err("usage: compact-rust-proof-smoke --opaque-string-set <proof-output>".into());
    }
    opaque_string_set::run(Path::new(&root))
}

pub(super) fn opaque_string_map(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --opaque-string-map <proof-output>")?;
    if arguments.next().is_some() {
        return Err("usage: compact-rust-proof-smoke --opaque-string-map <proof-output>".into());
    }
    opaque_string_map::run(Path::new(&root))
}

pub(super) fn conditional_set(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --conditional-set <proof-output>")?;
    if arguments.next().is_some() {
        return Err("usage: compact-rust-proof-smoke --conditional-set <proof-output>".into());
    }
    check_conditional_set_proof(Path::new(&root))
}

pub(super) fn adt_set_enum(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --adt-set-enum <proof-output>")?;
    if arguments.next().is_some() {
        return Err("usage: compact-rust-proof-smoke --adt-set-enum <proof-output>".into());
    }
    adt_set_enum::run(Path::new(&root))
}

pub(super) fn adt_set_vector(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --adt-set-vector <proof-output>")?;
    if arguments.next().is_some() {
        return Err("usage: compact-rust-proof-smoke --adt-set-vector <proof-output>".into());
    }
    adt_set_vector::run(Path::new(&root))
}

pub(super) fn adt_list_vector_field_4(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --adt-list-vector-field-4 <proof-output>")?;
    if arguments.next().is_some() {
        return Err(
            "usage: compact-rust-proof-smoke --adt-list-vector-field-4 <proof-output>".into(),
        );
    }
    adt_list_vector_field_4::run(Path::new(&root))
}

pub(super) fn adt_list_bytes(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --adt-list-bytes <proof-output>")?;
    if arguments.next().is_some() {
        return Err("usage: compact-rust-proof-smoke --adt-list-bytes <proof-output>".into());
    }
    adt_list_bytes::run(Path::new(&root))
}

pub(super) fn adt_list_enum(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --adt-list-enum <proof-output>")?;
    if arguments.next().is_some() {
        return Err("usage: compact-rust-proof-smoke --adt-list-enum <proof-output>".into());
    }
    adt_list_enum::run(Path::new(&root))
}

pub(super) fn adt_list_field(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --adt-list-field <proof-output>")?;
    if arguments.next().is_some() {
        return Err("usage: compact-rust-proof-smoke --adt-list-field <proof-output>".into());
    }
    adt_list_field::run(Path::new(&root))
}

pub(super) fn vector_map(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let identity = arguments
        .next()
        .ok_or("usage: --vector-map <identity-output> <arithmetic-constructor-output>")?;
    let arithmetic = arguments
        .next()
        .ok_or("usage: --vector-map <identity-output> <arithmetic-constructor-output>")?;
    if arguments.next().is_some() {
        return Err("usage: --vector-map <identity-output> <arithmetic-constructor-output>".into());
    }
    let proof = std::thread::Builder::new()
        .name("vector-map-proof".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            vector_map::run(Path::new(&identity), Path::new(&arithmetic)).map_err(|e| e.to_string())
        })?;
    match proof.join() {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(error.into()),
        Err(_) => Err("vector map proof thread panicked".into()),
    }
}

/// Selected Map source; use generated constructor data and actual strict funding.
pub(super) fn map_effects(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let usage = "usage: compact-rust-proof-smoke --map-effects <proof-output>";
    let root = arguments.next().ok_or(usage)?;
    if arguments.next().is_some() {
        return Err(usage.into());
    }
    let proof = std::thread::Builder::new()
        .name("map-effects-proof".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            super::super::map_effects::run(Path::new(&root)).map_err(|e| e.to_string())
        })?;
    match proof.join() {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(error.into()),
        Err(_) => Err("Map effect proof thread panicked".into()),
    }
}

/// Selected Unit source; use generated constructor data and actual strict funding.
pub(super) fn unit_composition(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let usage = "usage: compact-rust-proof-smoke --unit-composition <proof-output>";
    let root = arguments.next().ok_or(usage)?;
    if arguments.next().is_some() {
        return Err(usage.into());
    }
    let proof = std::thread::Builder::new()
        .name("unit-composition-proof".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            super::super::unit_composition::run(Path::new(&root)).map_err(|e| e.to_string())
        })?;
    match proof.join() {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(error.into()),
        Err(_) => Err("Unit composition proof thread panicked".into()),
    }
}

/// Selected aligned-hash source; use generated constructor data and actual strict funding.
pub(super) fn aligned_hashes(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let usage = "usage: compact-rust-proof-smoke --aligned-hashes <proof-output>";
    let root = arguments.next().ok_or(usage)?;
    if arguments.next().is_some() {
        return Err(usage.into());
    }
    let proof = std::thread::Builder::new()
        .name("aligned-hashes-proof".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            super::super::aligned_hashes::run(Path::new(&root)).map_err(|e| e.to_string())
        })?;
    match proof.join() {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(error.into()),
        Err(_) => Err("Aligned hash proof thread panicked".into()),
    }
}

pub(super) fn exact_hash_input(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let usage = "usage: compact-rust-proof-smoke --exact-hash-input <proof-output>";
    let root = arguments.next().ok_or(usage)?;
    if arguments.next().is_some() {
        return Err(usage.into());
    }
    let proof = std::thread::Builder::new()
        .name("exact-hash-input-proof".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            super::super::exact_hash_input::run(Path::new(&root)).map_err(|e| e.to_string())
        })?;
    match proof.join() {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(error.into()),
        Err(_) => Err("Exact hash input proof thread panicked".into()),
    }
}

pub(super) fn aligned_witness_results(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let usage = "usage: compact-rust-proof-smoke --aligned-witness-results <proof-output>";
    let root = arguments.next().ok_or(usage)?;
    if arguments.next().is_some() {
        return Err(usage.into());
    }
    let proof = std::thread::Builder::new()
        .name("aligned-witness-results-proof".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            super::super::aligned_witness_results::run(Path::new(&root)).map_err(|e| e.to_string())
        })?;
    match proof.join() {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(error.into()),
        Err(_) => Err("Aligned witness results proof thread panicked".into()),
    }
}

pub(super) fn jubjub_negation(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let usage = "usage: compact-rust-proof-smoke --jubjub-negation <proof-output>";
    let root = arguments.next().ok_or(usage)?;
    if arguments.next().is_some() {
        return Err(usage.into());
    }
    let proof = std::thread::Builder::new()
        .name("jubjub-negation-proof".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            super::super::jubjub_negation::run(Path::new(&root)).map_err(|e| e.to_string())
        })?;
    match proof.join() {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(error.into()),
        Err(_) => Err("Jubjub negation proof thread panicked".into()),
    }
}
