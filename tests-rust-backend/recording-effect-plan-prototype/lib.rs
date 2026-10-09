// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

// The synthetic branch-swap probe intentionally expresses negation as
// Equal(condition, false); retain the native emitter's literal IR translation.
#![allow(clippy::bool_comparison)]

// The default workspace does not activate the experimental backend dependency.
// The generated root is required because ledger descriptors use crate paths.
#[cfg(feature = "experimental-recording-plan")]
include!(concat!(env!("OUT_DIR"), "/contract.rs"));
