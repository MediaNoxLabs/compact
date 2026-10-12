// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
include!(concat!(env!("OUT_DIR"), "/contract.rs"));
pub const RETAINED_IR: &str = include_str!(
    "../../tools/compact-rust-backend/tests/effect-plan-aligned-hashes-schema20-ir.json"
);
