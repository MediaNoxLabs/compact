// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Full current-generated contracts live at each integration test crate's root.
pub const BBOARD_IR: &str =
    include_str!("../../tools/compact-rust-backend/tests/bboard-schema20-ir.json");
pub const NESTED_IR: &str =
    include_str!("../../tools/compact-rust-backend/tests/opaque-observed-nested-schema20-ir.json");
