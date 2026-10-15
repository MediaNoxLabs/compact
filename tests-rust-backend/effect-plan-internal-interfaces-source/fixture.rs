// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Complete generated internal tuple interfaces require explicit Rust tuple types.
#![allow(clippy::type_complexity)]
include!(concat!(env!("OUT_DIR"), "/contract.rs"));
pub const RETAINED_IR: &str = include_str!(
    "../../tools/compact-rust-backend/tests/effect-plan-internal-interfaces-schema20-ir.json"
);
#[cfg(test)]
mod native_internal_tests;
pub mod support;
