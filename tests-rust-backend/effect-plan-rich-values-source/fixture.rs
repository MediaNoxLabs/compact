// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Exact source-bound rich-value compositions; no synthetic exports.
include!(concat!(env!("OUT_DIR"), "/contract.rs"));

pub const RETAINED_IR: &str =
    include_str!("../../tools/compact-rust-backend/tests/effect-plan-rich-values-schema20-ir.json");

/// Independent source-semantics oracle shared by runtime and proof qualification.
pub mod support;
