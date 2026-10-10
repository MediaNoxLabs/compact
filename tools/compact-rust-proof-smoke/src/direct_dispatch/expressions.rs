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

//! Direct command argument parsing and execution for expressions proof scenarios.
//!
//! Each handler retains its original argument checks and worker policy.

use std::error::Error;
use std::ffi::OsString;
use std::path::Path;

use crate::{
    boolean_pair_assert, closed_pure_field, field_observation, field_pair_hash, field_to_bytes32,
    guarded_recording, impure_field_helper, inline_type_scope, let_return, literal_bytes_field,
    mixed_width_recording, persistent_commit, pure_assert_call, pure_field_arguments,
    schnorr_attestation, struct_constructor_cell, terminal_lexical_return, unsigned_recording,
    unused_field_reads,
};

pub(super) fn mixed_width_recording(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --mixed-width-recording <proof-output>")?;
    if arguments.next().is_some() {
        return Err(
            "usage: compact-rust-proof-smoke --mixed-width-recording <proof-output>".into(),
        );
    }
    let proof = std::thread::Builder::new()
        .name("mixed-width-recording-proof".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || mixed_width_recording::run(Path::new(&root)).map_err(|e| e.to_string()))?;
    match proof.join() {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(error.into()),
        Err(_) => Err("mixed-width recording proof thread panicked".into()),
    }
}

pub(super) fn wide_unsigned_field_cast(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --wide-unsigned-field-cast <proof-output>")?;
    if arguments.next().is_some() {
        return Err(
            "usage: compact-rust-proof-smoke --wide-unsigned-field-cast <proof-output>".into(),
        );
    }
    unsigned_recording::wide_cast(Path::new(&root))
}

pub(super) fn closed_pure_unsigned_call(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --closed-pure-unsigned-call <proof-output>")?;
    if arguments.next().is_some() {
        return Err(
            "usage: compact-rust-proof-smoke --closed-pure-unsigned-call <proof-output>".into(),
        );
    }
    unsigned_recording::widening_call(Path::new(&root))
}

pub(super) fn guarded_recording(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --guarded-recording <proof-output>")?;
    if arguments.next().is_some() {
        return Err("usage: compact-rust-proof-smoke --guarded-recording <proof-output>".into());
    }
    guarded_recording::run(Path::new(&root))
}

pub(super) fn schnorr_attestation(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --schnorr-attestation <proof-output>")?;
    if arguments.next().is_some() {
        return Err("usage: compact-rust-proof-smoke --schnorr-attestation <proof-output>".into());
    }
    schnorr_attestation::run(Path::new(&root))
}

pub(super) fn closed_pure_field(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --closed-pure-field <proof-output>")?;
    if arguments.next().is_some() {
        return Err("usage: compact-rust-proof-smoke --closed-pure-field <proof-output>".into());
    }
    closed_pure_field::run(Path::new(&root))
}

pub(super) fn struct_constructor_cell(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --struct-constructor-cell <proof-output>")?;
    if arguments.next().is_some() {
        return Err(
            "usage: compact-rust-proof-smoke --struct-constructor-cell <proof-output>".into(),
        );
    }
    struct_constructor_cell::run(Path::new(&root))
}

pub(super) fn pure_assert_call(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let pure_root = arguments.next().ok_or(
        "usage: compact-rust-proof-smoke --pure-assert-call <pure-call-proof> <assert-proof>",
    )?;
    let assert_root = arguments.next().ok_or(
        "usage: compact-rust-proof-smoke --pure-assert-call <pure-call-proof> <assert-proof>",
    )?;
    if arguments.next().is_some() {
        return Err(
            "usage: compact-rust-proof-smoke --pure-assert-call <pure-call-proof> <assert-proof>"
                .into(),
        );
    }
    pure_assert_call::run(Path::new(&pure_root), Path::new(&assert_root))
}

pub(super) fn field_pair_hash(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --field-pair-hash <proof-output>")?;
    if arguments.next().is_some() {
        return Err("usage: compact-rust-proof-smoke --field-pair-hash <proof-output>".into());
    }
    let proof = std::thread::Builder::new()
        .name("field-pair-hash-proof".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || field_pair_hash::run(Path::new(&root)).map_err(|e| e.to_string()))?;
    match proof.join() {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(error.into()),
        Err(_) => Err("field pair hash proof thread panicked".into()),
    }
}

pub(super) fn impure_field_helper(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --impure-field-helper <proof-output>")?;
    if arguments.next().is_some() {
        return Err("usage: compact-rust-proof-smoke --impure-field-helper <proof-output>".into());
    }
    let proof = std::thread::Builder::new()
        .name("impure-field-helper-proof".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || impure_field_helper::run(Path::new(&root)).map_err(|e| e.to_string()))?;
    match proof.join() {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(error.into()),
        Err(_) => Err("impure Field helper proof thread panicked".into()),
    }
}

pub(super) fn unused_field_reads(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let seven = arguments.next().ok_or("expected seven proof output")?;
    let eight_a = arguments.next().ok_or("expected eight-a proof output")?;
    let eight_b = arguments.next().ok_or("expected eight-b proof output")?;
    if arguments.next().is_some() {
        return Err("unexpected unused-field-reads argument".into());
    }
    unused_field_reads::run(Path::new(&seven), Path::new(&eight_a), Path::new(&eight_b))
}

pub(super) fn let_return(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --let-return <proof-output>")?;
    if arguments.next().is_some() {
        return Err("usage: compact-rust-proof-smoke --let-return <proof-output>".into());
    }
    let proof = std::thread::Builder::new()
        .name("let-return-proof".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || let_return::run(Path::new(&root)).map_err(|e| e.to_string()))?;
    match proof.join() {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(error.into()),
        Err(_) => Err("root-Let proof thread panicked".into()),
    }
}

pub(super) fn field_to_bytes32(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --field-to-bytes32 <proof-output>")?;
    if arguments.next().is_some() {
        return Err("usage: compact-rust-proof-smoke --field-to-bytes32 <proof-output>".into());
    }
    let proof = std::thread::Builder::new()
        .name("field-to-bytes32-proof".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || field_to_bytes32::run(Path::new(&root)).map_err(|e| e.to_string()))?;
    match proof.join() {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(error.into()),
        Err(_) => Err("Field-to-Bytes32 proof thread panicked".into()),
    }
}

pub(super) fn literal_bytes_field(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --literal-bytes-field <proof-output>")?;
    if arguments.next().is_some() {
        return Err("usage: compact-rust-proof-smoke --literal-bytes-field <proof-output>".into());
    }
    let proof = std::thread::Builder::new()
        .name("literal-bytes-field-proof".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || literal_bytes_field::run(Path::new(&root)).map_err(|e| e.to_string()))?;
    match proof.join() {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(error.into()),
        Err(_) => Err("literal Bytes-to-Field proof thread panicked".into()),
    }
}

pub(super) fn terminal_lexical_return(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --terminal-lexical-return <proof-output>")?;
    if arguments.next().is_some() {
        return Err(
            "usage: compact-rust-proof-smoke --terminal-lexical-return <proof-output>".into(),
        );
    }
    let proof = std::thread::Builder::new()
        .name("terminal-lexical-return-proof".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || terminal_lexical_return::run(Path::new(&root)).map_err(|e| e.to_string()))?;
    match proof.join() {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(error.into()),
        Err(_) => Err("terminal lexical return proof thread panicked".into()),
    }
}

pub(super) fn field_observation(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --field-observation <proof-output>")?;
    if arguments.next().is_some() {
        return Err("usage: compact-rust-proof-smoke --field-observation <proof-output>".into());
    }
    let proof = std::thread::Builder::new()
        .name("field-observation-proof".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || field_observation::run(Path::new(&root)).map_err(|e| e.to_string()))?;
    match proof.join() {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(error.into()),
        Err(_) => Err("stateful struct proof thread panicked".into()),
    }
}

pub(super) fn boolean_pair_assert(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --boolean-pair-assert <proof-output>")?;
    if arguments.next().is_some() {
        return Err("usage: compact-rust-proof-smoke --boolean-pair-assert <proof-output>".into());
    }
    let proof = std::thread::Builder::new()
        .name("boolean-pair-assert-proof".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || boolean_pair_assert::run(Path::new(&root)).map_err(|e| e.to_string()))?;
    match proof.join() {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(error.into()),
        Err(_) => Err("Boolean pair assertion proof thread panicked".into()),
    }
}

pub(super) fn pure_field_arguments(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let internal = arguments.next().ok_or(
        "usage: compact-rust-proof-smoke --pure-field-arguments <internal-proof> <ternary-proof>",
    )?;
    let ternary = arguments.next().ok_or(
        "usage: compact-rust-proof-smoke --pure-field-arguments <internal-proof> <ternary-proof>",
    )?;
    if arguments.next().is_some() {
        return Err("usage: compact-rust-proof-smoke --pure-field-arguments <internal-proof> <ternary-proof>".into());
    }
    // Ledger proof composition exceeds macOS's default main-thread stack.
    // Keep the larger stack scoped to this proof smoke, rather than raising
    // the stack limit for every target-gate process.
    let proof = std::thread::Builder::new()
        .name("pure-field-arguments-proof".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            pure_field_arguments::run(Path::new(&internal), Path::new(&ternary))
                .map_err(|error| error.to_string())
        })?;
    match proof.join() {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(error.into()),
        Err(_) => Err("pure Field argument proof thread panicked".into()),
    }
}

pub(super) fn persistent_commit(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --persistent-commit <proof-output>")?;
    if arguments.next().is_some() {
        return Err("usage: compact-rust-proof-smoke --persistent-commit <proof-output>".into());
    }
    persistent_commit::run(Path::new(&root))
}

pub(super) fn inline_type_scope(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --inline-type-scope <proof-output>")?;
    if arguments.next().is_some() {
        return Err("usage: compact-rust-proof-smoke --inline-type-scope <proof-output>".into());
    }
    let proof = std::thread::Builder::new()
        .name("inline-type-scope-proof".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || inline_type_scope::run(Path::new(&root)).map_err(|e| e.to_string()))?;
    match proof.join() {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(error.into()),
        Err(_) => Err("inline type scope proof thread panicked".into()),
    }
}

pub(super) fn opaque_public(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let usage = "usage: compact-rust-proof-smoke --opaque-public <proof-output>";
    let root = arguments.next().ok_or(usage)?;
    if arguments.next().is_some() {
        return Err(usage.into());
    }
    let proof = std::thread::Builder::new()
        .name("opaque-public-proof".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            super::super::opaque_public::run(Path::new(&root)).map_err(|e| e.to_string())
        })?;
    match proof.join() {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(error.into()),
        Err(_) => Err("opaque public proof thread panicked".into()),
    }
}

pub(super) fn owned_witness_inputs(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let usage = "usage: compact-rust-proof-smoke --owned-witness-inputs <proof-output>";
    let root = arguments.next().ok_or(usage)?;
    if arguments.next().is_some() {
        return Err(usage.into());
    }
    let proof = std::thread::Builder::new()
        .name("owned-witness-inputs-proof".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            super::super::owned_witness_inputs::run(Path::new(&root)).map_err(|e| e.to_string())
        })?;
    match proof.join() {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(error.into()),
        Err(_) => Err("owned witness inputs proof thread panicked".into()),
    }
}

pub(super) fn public_products(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let usage = "usage: compact-rust-proof-smoke --public-products <proof-output>";
    let root = arguments.next().ok_or(usage)?;
    if arguments.next().is_some() {
        return Err(usage.into());
    }
    let proof = std::thread::Builder::new()
        .name("public-products-proof".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            super::super::public_products::run(Path::new(&root)).map_err(|e| e.to_string())
        })?;
    match proof.join() {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(error.into()),
        Err(_) => Err("public products proof thread panicked".into()),
    }
}

pub(super) fn wide_casts(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let usage = "usage: compact-rust-proof-smoke --wide-casts <proof-output>";
    let root = arguments.next().ok_or(usage)?;
    if arguments.next().is_some() {
        return Err(usage.into());
    }
    let proof = std::thread::Builder::new()
        .name("wide-casts-proof".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            super::super::wide_casts::run(Path::new(&root)).map_err(|e| e.to_string())
        })?;
    match proof.join() {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(error.into()),
        Err(_) => Err("wide casts proof thread panicked".into()),
    }
}

pub(super) fn wide_add(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let usage = "usage: compact-rust-proof-smoke --wide-add <proof-output>";
    let root = arguments.next().ok_or(usage)?;
    if arguments.next().is_some() {
        return Err(usage.into());
    }
    let proof = std::thread::Builder::new()
        .name("wide-add-proof".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || super::super::wide_add::run(Path::new(&root)).map_err(|e| e.to_string()))?;
    match proof.join() {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(error.into()),
        Err(_) => Err("wide addition proof thread panicked".into()),
    }
}
