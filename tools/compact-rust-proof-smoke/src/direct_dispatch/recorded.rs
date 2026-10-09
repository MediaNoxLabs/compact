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

//! Direct command argument parsing and execution for recorded proof scenarios.
//!
//! Each handler retains its original argument checks and worker policy.

use std::error::Error;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use crate::{
    check_annotated_uint8_proof, check_boolean_observation_proof,
    check_closed_curve_argument_proof, check_closed_ternary_struct_member_proof,
    check_closed_unsigned_ternary_comparison_proof, check_composite_cell_proof,
    check_conditional_assert_eq_proof, check_conditional_counter_proof,
    check_conditional_field_proof, check_conditional_field_vector_proof, check_nested_uint4_proof,
    check_nested_uint64_cell_counter_proof, check_unsigned_sequential_proof, counter_less_than,
    effectful_return, pair_hash_cell, root_let_action_return, stateful_assert, stateful_pair_hash,
    stateful_pure_return, stateful_struct, witness_assert, witness_vector_action,
    witness_vector_let,
};

pub(super) fn stateful_struct(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --stateful-struct <proof-output>")?;
    if arguments.next().is_some() {
        return Err("usage: compact-rust-proof-smoke --stateful-struct <proof-output>".into());
    }
    let proof = std::thread::Builder::new()
        .name("stateful-struct-proof".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || stateful_struct::run(Path::new(&root)).map_err(|e| e.to_string()))?;
    match proof.join() {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(error.into()),
        Err(_) => Err("stateful struct proof thread panicked".into()),
    }
}

pub(super) fn stateful_assert(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --stateful-assert <proof-output>")?;
    if arguments.next().is_some() {
        return Err("usage: compact-rust-proof-smoke --stateful-assert <proof-output>".into());
    }
    let proof = std::thread::Builder::new()
        .name("stateful-assert-proof".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || stateful_assert::run(Path::new(&root)).map_err(|e| e.to_string()))?;
    match proof.join() {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(error.into()),
        Err(_) => Err("stateful assert proof thread panicked".into()),
    }
}

pub(super) fn effectful_return(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --effectful-return <proof-output>")?;
    if arguments.next().is_some() {
        return Err("usage: compact-rust-proof-smoke --effectful-return <proof-output>".into());
    }
    let proof = std::thread::Builder::new()
        .name("effectful-return-proof".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || effectful_return::run(Path::new(&root)).map_err(|e| e.to_string()))?;
    match proof.join() {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(error.into()),
        Err(_) => Err("effectful return proof thread panicked".into()),
    }
}

pub(super) fn root_let_action_return(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --root-let-action-return <proof-output>")?;
    if arguments.next().is_some() {
        return Err(
            "usage: compact-rust-proof-smoke --root-let-action-return <proof-output>".into(),
        );
    }
    let proof = std::thread::Builder::new()
        .name("root-let-action-return-proof".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || root_let_action_return::run(Path::new(&root)).map_err(|e| e.to_string()))?;
    match proof.join() {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(error.into()),
        Err(_) => Err("root Let action-return proof thread panicked".into()),
    }
}

pub(super) fn counter_less_than(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --counter-less-than <proof-output>")?;
    if arguments.next().is_some() {
        return Err("usage: compact-rust-proof-smoke --counter-less-than <proof-output>".into());
    }
    let proof = std::thread::Builder::new()
        .name("counter-less-than-proof".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || counter_less_than::run(Path::new(&root)).map_err(|e| e.to_string()))?;
    match proof.join() {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(error.into()),
        Err(_) => Err("Counter lessThan proof thread panicked".into()),
    }
}

pub(super) fn pair_hash_cell(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --pair-hash-cell <proof-output>")?;
    if arguments.next().is_some() {
        return Err("usage: compact-rust-proof-smoke --pair-hash-cell <proof-output>".into());
    }
    let proof = std::thread::Builder::new()
        .name("pair-hash-cell-proof".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || pair_hash_cell::run(Path::new(&root)).map_err(|e| e.to_string()))?;
    match proof.join() {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(error.into()),
        Err(_) => Err("pair hash Cell proof thread panicked".into()),
    }
}

pub(super) fn stateful_pair_hash(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --stateful-pair-hash <proof-output>")?;
    if arguments.next().is_some() {
        return Err("usage: compact-rust-proof-smoke --stateful-pair-hash <proof-output>".into());
    }
    let proof = std::thread::Builder::new()
        .name("stateful-pair-hash-proof".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || stateful_pair_hash::run(Path::new(&root)).map_err(|e| e.to_string()))?;
    match proof.join() {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(error.into()),
        Err(_) => Err("stateful pair hash proof thread panicked".into()),
    }
}

pub(super) fn stateful_pure_return(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --stateful-pure-return <proof-output>")?;
    if arguments.next().is_some() {
        return Err("usage: compact-rust-proof-smoke --stateful-pure-return <proof-output>".into());
    }
    let proof = std::thread::Builder::new()
        .name("stateful-pure-return-proof".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || stateful_pure_return::run(Path::new(&root)).map_err(|e| e.to_string()))?;
    match proof.join() {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(error.into()),
        Err(_) => Err("stateful pure return proof thread panicked".into()),
    }
}

pub(super) fn assert_witness(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --assert-witness <proof-output>")?;
    if arguments.next().is_some() {
        return Err("usage: compact-rust-proof-smoke --assert-witness <proof-output>".into());
    }
    witness_assert::run(Path::new(&root))
}

pub(super) fn witness_vector_let(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --witness-vector-let <proof-output>")?;
    if arguments.next().is_some() {
        return Err("usage: compact-rust-proof-smoke --witness-vector-let <proof-output>".into());
    }
    witness_vector_let::run(Path::new(&root))
}

pub(super) fn witness_vector_bare(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --witness-vector-bare <proof-output>")?;
    if arguments.next().is_some() {
        return Err("usage: compact-rust-proof-smoke --witness-vector-bare <proof-output>".into());
    }
    witness_vector_let::run_bare(Path::new(&root))
}

pub(super) fn witness_vector_discard(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --witness-vector-discard <proof-output>")?;
    if arguments.next().is_some() {
        return Err(
            "usage: compact-rust-proof-smoke --witness-vector-discard <proof-output>".into(),
        );
    }
    witness_vector_action::run(Path::new(&root))
}

pub(super) fn composite_cell(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --composite-cell <proof-output>")?;
    if arguments.next().is_some() {
        return Err("usage: compact-rust-proof-smoke --composite-cell <proof-output>".into());
    }
    check_composite_cell_proof(Path::new(&root))
}

pub(super) fn boolean_observation(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --boolean-observation <proof-output>")?;
    if arguments.next().is_some() {
        return Err("usage: compact-rust-proof-smoke --boolean-observation <proof-output>".into());
    }
    check_boolean_observation_proof(Path::new(&root))
}

pub(super) fn conditional_counter(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --conditional-counter <proof-output>")?;
    if arguments.next().is_some() {
        return Err("usage: compact-rust-proof-smoke --conditional-counter <proof-output>".into());
    }
    check_conditional_counter_proof(Path::new(&root))
}

pub(super) fn unsigned_sequential(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --unsigned-sequential <proof-output>")?;
    if arguments.next().is_some() {
        return Err("usage: compact-rust-proof-smoke --unsigned-sequential <proof-output>".into());
    }
    check_unsigned_sequential_proof(Path::new(&root))
}

pub(super) fn conditional_field(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --conditional-field <proof-output>")?;
    if arguments.next().is_some() {
        return Err("usage: compact-rust-proof-smoke --conditional-field <proof-output>".into());
    }
    check_conditional_field_proof(Path::new(&root))
}

pub(super) fn conditional_assert_eq(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --conditional-assert-eq <proof-output>")?;
    if arguments.next().is_some() {
        return Err(
            "usage: compact-rust-proof-smoke --conditional-assert-eq <proof-output>".into(),
        );
    }
    check_conditional_assert_eq_proof(Path::new(&root))
}

pub(super) fn conditional_compare_eq(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --conditional-compare-eq <proof-output>")?;
    if arguments.next().is_some() {
        return Err(
            "usage: compact-rust-proof-smoke --conditional-compare-eq <proof-output>".into(),
        );
    }
    check_closed_unsigned_ternary_comparison_proof(Path::new(&root))
}

pub(super) fn nested_uint64_cell_counter(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --nested-uint64-cell-counter <proof-output>")?;
    if arguments.next().is_some() {
        return Err(
            "usage: compact-rust-proof-smoke --nested-uint64-cell-counter <proof-output>".into(),
        );
    }
    let root = PathBuf::from(root);
    match std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            check_nested_uint64_cell_counter_proof(&root).map_err(|error| error.to_string())
        })?
        .join()
    {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(error.into()),
        Err(_) => Err("nested Uint<64> Cell/Counter proof thread panicked".into()),
    }
}

pub(super) fn nested_uint4(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --nested-uint4 <proof-output>")?;
    if arguments.next().is_some() {
        return Err("usage: compact-rust-proof-smoke --nested-uint4 <proof-output>".into());
    }
    // The ledger/ZKIR proof composition needs more than the default
    // macOS main-thread stack. Give this focused mode its own bound.
    let root = PathBuf::from(root);
    match std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(move || check_nested_uint4_proof(&root).map_err(|error| error.to_string()))?
        .join()
    {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(error.into()),
        Err(_) => Err("nested Uint<4> proof thread panicked".into()),
    }
}

pub(super) fn conditional_field_vector(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --conditional-field-vector <proof-output>")?;
    if arguments.next().is_some() {
        return Err(
            "usage: compact-rust-proof-smoke --conditional-field-vector <proof-output>".into(),
        );
    }
    let root = PathBuf::from(root);
    match std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            check_conditional_field_vector_proof(&root).map_err(|error| error.to_string())
        })?
        .join()
    {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(error.into()),
        Err(_) => Err("conditional Field vector proof thread panicked".into()),
    }
}

pub(super) fn annotated_uint8(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --annotated-uint8 <proof-output>")?;
    if arguments.next().is_some() {
        return Err("usage: compact-rust-proof-smoke --annotated-uint8 <proof-output>".into());
    }
    let root = PathBuf::from(root);
    match std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(move || check_annotated_uint8_proof(&root).map_err(|error| error.to_string()))?
        .join()
    {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(error.into()),
        Err(_) => Err("annotated Uint<8> proof thread panicked".into()),
    }
}

pub(super) fn native_curve_arg(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --native-curve-arg <proof-output>")?;
    if arguments.next().is_some() {
        return Err("usage: compact-rust-proof-smoke --native-curve-arg <proof-output>".into());
    }
    let root = PathBuf::from(root);
    match std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(move || check_closed_curve_argument_proof(&root).map_err(|error| error.to_string()))?
        .join()
    {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(error.into()),
        Err(_) => Err("closed curve argument proof thread panicked".into()),
    }
}

pub(super) fn conditional_struct_member(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --conditional-struct-member <proof-output>")?;
    if arguments.next().is_some() {
        return Err(
            "usage: compact-rust-proof-smoke --conditional-struct-member <proof-output>".into(),
        );
    }
    check_closed_ternary_struct_member_proof(Path::new(&root))
}
