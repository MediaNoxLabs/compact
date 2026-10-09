// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Generated from retained source IR using the current backend during this build.
include!(concat!(env!("OUT_DIR"), "/contract.rs"));

pub const RETAINED_IR: &str =
    include_str!("../../tools/compact-rust-backend/tests/terminal-lexical-return-schema20-ir.json");
