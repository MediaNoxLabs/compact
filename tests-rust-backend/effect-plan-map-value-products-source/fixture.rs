// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Unchanged genuine Map product source; full generated output at crate root.
pub const RETAINED_IR: &str = include_str!(
    "../../tools/compact-rust-backend/tests/effect-plan-map-value-products-schema20-ir.json"
);
include!(concat!(env!("OUT_DIR"), "/contract.rs"));
pub mod support;
