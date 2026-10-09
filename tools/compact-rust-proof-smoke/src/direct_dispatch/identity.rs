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

//! Direct command argument parsing and execution for identity proof scenarios.
//!
//! Each handler retains its original argument checks and worker policy.

use std::error::Error;
use std::ffi::OsString;
use std::path::Path;

use crate::{did_deactivate, did_digest_reducer, did_point_lifecycle, did_primitive_reducers};

pub(super) fn did_relation_lifecycle(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --did-relation-lifecycle <proof-output>")?;
    if arguments.next().is_some() {
        return Err("unexpected DID relation lifecycle argument".into());
    }
    let proof = std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .name("did-relation-lifecycle-proof".into())
        .spawn(move || {
            did_point_lifecycle::run_relations(Path::new(&root)).map_err(|e| e.to_string())
        })?;
    match proof.join() {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(error.into()),
        Err(_) => Err("DID relation proof worker panicked".into()),
    }
}

pub(super) fn did_relation_schnorr(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --did-relation-schnorr <proof-output>")?;
    if arguments.next().is_some() {
        return Err("unexpected DID relation argument".into());
    }
    let proof = std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .name("did-relation-schnorr-proof".into())
        .spawn(move || {
            did_point_lifecycle::run_relation_schnorr(Path::new(&root)).map_err(|e| e.to_string())
        })?;
    match proof.join() {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(error.into()),
        Err(_) => Err("DID relation Schnorr proof worker panicked".into()),
    }
}

pub(super) fn did_point_lifecycle(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --did-point-lifecycle <proof-output>")?;
    if arguments.next().is_some() {
        return Err("unexpected DID lifecycle argument".into());
    }
    let proof = std::thread::Builder::new()
        .name("did-point-lifecycle-proof".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || did_point_lifecycle::run(Path::new(&root)).map_err(|e| e.to_string()))?;
    match proof.join() {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(error.into()),
        Err(_) => Err("DID lifecycle proof thread panicked".into()),
    }
}

pub(super) fn did_alias_lifecycle(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --did-alias-lifecycle <proof-output>")?;
    if arguments.next().is_some() {
        return Err("unexpected DID lifecycle argument".into());
    }
    let proof = std::thread::Builder::new()
        .name("did-alias-lifecycle-proof".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            did_point_lifecycle::run_aliases(Path::new(&root)).map_err(|e| e.to_string())
        })?;
    match proof.join() {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(error.into()),
        Err(_) => Err("DID lifecycle proof thread panicked".into()),
    }
}

pub(super) fn did_service_lifecycle(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --did-service-lifecycle <proof-output>")?;
    if arguments.next().is_some() {
        return Err("unexpected DID lifecycle argument".into());
    }
    let proof = std::thread::Builder::new()
        .name("did-service-lifecycle-proof".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            did_point_lifecycle::run_services(Path::new(&root)).map_err(|e| e.to_string())
        })?;
    match proof.join() {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(error.into()),
        Err(_) => Err("DID lifecycle proof thread panicked".into()),
    }
}

pub(super) fn did_schnorr_method_lifecycle(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --did-schnorr-method-lifecycle <proof-output>")?;
    if arguments.next().is_some() {
        return Err("unexpected DID lifecycle argument".into());
    }
    let proof = std::thread::Builder::new()
        .name("did-schnorr-method-lifecycle-proof".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            did_point_lifecycle::run_schnorr_methods(Path::new(&root)).map_err(|e| e.to_string())
        })?;
    match proof.join() {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(error.into()),
        Err(_) => Err("DID lifecycle proof thread panicked".into()),
    }
}

pub(super) fn did_jwk_method_lifecycle(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --did-jwk-method-lifecycle <proof-output>")?;
    if arguments.next().is_some() {
        return Err("unexpected DID lifecycle argument".into());
    }
    let proof = std::thread::Builder::new()
        .name("did-jwk-method-lifecycle-proof".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            did_point_lifecycle::run_jwk_methods(Path::new(&root)).map_err(|e| e.to_string())
        })?;
    match proof.join() {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(error.into()),
        Err(_) => Err("DID lifecycle proof thread panicked".into()),
    }
}

pub(super) fn did_primitive_reducer(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let kind = arguments.next().ok_or("expected reducer kind")?;
    let root = arguments.next().ok_or("expected proof output directory")?;
    if arguments.next().is_some() {
        return Err("unexpected reducer proof arguments".into());
    }
    let kind = kind.to_str().ok_or("invalid reducer kind")?.to_owned();
    if !matches!(
        kind.as_str(),
        "relation-two" | "relation-four" | "relation-nested"
    ) {
        return did_primitive_reducers::run(Path::new(&root), &kind);
    }
    let proof = std::thread::Builder::new()
        .name("did-relation-reducer-proof".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            did_primitive_reducers::run(Path::new(&root), &kind).map_err(|e| e.to_string())
        })?;
    match proof.join() {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(error.into()),
        Err(_) => Err("DID relation reducer proof thread panicked".into()),
    }
}

pub(super) fn did_digest_reducer(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --did-digest-reducer <proof-output>")?;
    if arguments.next().is_some() {
        return Err("unexpected DID digest reducer argument".into());
    }
    let proof = std::thread::Builder::new()
        .name("did-digest-reducer-proof".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || did_digest_reducer::run(Path::new(&root)).map_err(|e| e.to_string()))?;
    match proof.join() {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(error.into()),
        Err(_) => Err("DID digest reducer proof thread panicked".into()),
    }
}

pub(super) fn did_digest_verification(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --did-digest-verification <proof-output>")?;
    if arguments.next().is_some() {
        return Err("unexpected DID digest argument".into());
    }
    let proof = std::thread::Builder::new()
        .name("did-digest-verification-proof".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            did_point_lifecycle::run_digest(Path::new(&root)).map_err(|e| e.to_string())
        })?;
    match proof.join() {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(error.into()),
        Err(_) => Err("DID digest proof thread panicked".into()),
    }
}

pub(super) fn did_deactivate(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let root = arguments
        .next()
        .ok_or("usage: compact-rust-proof-smoke --did-deactivate <proof-output>")?;
    if arguments.next().is_some() {
        return Err("usage: compact-rust-proof-smoke --did-deactivate <proof-output>".into());
    }
    let proof = std::thread::Builder::new()
        .name("did-deactivate-proof".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || did_deactivate::run(Path::new(&root)).map_err(|e| e.to_string()))?;
    match proof.join() {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(error.into()),
        Err(_) => Err("DID deactivate proof thread panicked".into()),
    }
}
