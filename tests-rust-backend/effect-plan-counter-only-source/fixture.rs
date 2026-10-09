// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Exact Counter-only source; no synthetic exports.
include!(concat!(env!("OUT_DIR"), "/contract.rs"));

pub const RETAINED_IR: &str = include_str!(
    "../../tools/compact-rust-backend/tests/effect-plan-counter-only-schema20-ir.json"
);
