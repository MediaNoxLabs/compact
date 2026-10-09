// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Exact source-bound stateful call compositions; no synthetic exports.
include!(concat!(env!("OUT_DIR"), "/contract.rs"));

pub const RETAINED_IR: &str = include_str!(
    "../../tools/compact-rust-backend/tests/effect-plan-stateful-calls-schema20-ir.json"
);
