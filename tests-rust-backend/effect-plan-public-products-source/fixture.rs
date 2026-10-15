// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// These ABI boundary controls deliberately expose maximum-width nested tuple types.
#![allow(clippy::type_complexity)]
include!(concat!(env!("OUT_DIR"), "/contract.rs"));
pub const RETAINED_IR: &str = include_str!(
    "../../tools/compact-rust-backend/tests/effect-plan-public-products-schema20-ir.json"
);
pub mod support;
