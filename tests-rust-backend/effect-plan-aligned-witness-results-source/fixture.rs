// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// This boundary fixture deliberately exposes an eleven-member witness tuple.
#![allow(clippy::type_complexity)]
include!(concat!(env!("OUT_DIR"), "/contract.rs"));
pub const RETAINED_IR: &str = include_str!(
    "../../tools/compact-rust-backend/tests/effect-plan-aligned-witness-results-schema20-ir.json"
);
