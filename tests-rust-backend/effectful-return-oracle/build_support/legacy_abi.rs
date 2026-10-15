// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Test-only projection of the authenticated fixed-value ABI51 oracle.
use sha2::{Digest, Sha256};
pub const ORIGINAL_SHA: &str = "4c6b6a4004607ce2c14ca0709af58d345bae1f0a9ff8f87942b42cf452c2c16b";
const OLD: &str = "const _: () = assert!(runtime::RUST_RUNTIME_ABI == 51);";
const NEW: &str = "const _: () = assert!(runtime::RUST_RUNTIME_ABI == 52);";

pub fn project(source: &str) -> Result<String, &'static str> {
    if format!("{:x}", Sha256::digest(source.as_bytes())) != ORIGINAL_SHA {
        return Err("frozen legacy oracle differs from reviewed whole source");
    }
    if source.matches(OLD).count() != 2 || source.contains(NEW) {
        return Err("expected exactly two original ABI51 assertions");
    }
    let projected = source.replace(OLD, NEW);
    if projected.matches(NEW).count() != 2 || projected.replace(NEW, OLD) != source {
        return Err("legacy test projection changed more than two ABI guards");
    }
    Ok(projected)
}
