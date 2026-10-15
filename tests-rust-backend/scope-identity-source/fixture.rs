// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Exact source-bound lexical binding identity regression sources; no synthetic exports.
// The alias regression intentionally retains the private original `same` circuit.
// Consumers exercise its two public aliases; the original has no call site.
#[allow(dead_code)]
mod generated {
    include!(concat!(env!("OUT_DIR"), "/contract.rs"));
}
pub use generated::*;

pub const RETAINED_IR: &str =
    include_str!("../../tools/compact-rust-backend/tests/scope-identity-schema20-ir.json");
