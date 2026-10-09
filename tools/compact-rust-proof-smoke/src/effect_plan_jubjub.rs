// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Exact source-bound point composition proofs; finite qualification, not grammar coverage.
use super::*;
use compact_rust_effect_plan_jubjub_source_fixture as source;
use source::{ledger_contract as contract, ledger_slots, runtime as r};

#[cfg(test)]
const OTHER_RETAINED_IR: &str = compact_rust_guarded_point_read_source_fixture::RETAINED_IR;
include!("jubjub_source_proof.rs");
