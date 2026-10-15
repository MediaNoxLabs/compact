// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Exact unchanged opaque_cell_inputs source, generated at crate root.
pub const RETAINED_IR: &str =
    include_str!("../../tools/compact-rust-backend/tests/opaque-cell-inputs-schema20-ir.json");
include!(concat!(env!("OUT_DIR"), "/contract.rs"));
