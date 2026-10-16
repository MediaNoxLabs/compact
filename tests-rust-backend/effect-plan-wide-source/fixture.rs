// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Exact wide-source contract, freshly emitted by the current backend.
//! No synthetic exports or alternate source bodies are added to this fixture.
include!(concat!(env!("OUT_DIR"), "/contract.rs"));

/// The same retained IR used by build.rs; proof artifact pairing checks this input.
pub const RETAINED_IR: &str =
    include_str!("../../tools/compact-rust-backend/tests/effect-plan-wide-schema20-ir.json");
