// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Source-owned execution results do not require transaction output encoding.
use super::*;

#[test]
fn complete_source_tuple_results_have_checked_public_ownership() {
    for (label, ir) in [
        (
            "Tuple12 Field result",
            include_str!("../../../tests/tuple12-result-ir.json"),
        ),
        (
            "Tuple13 owned result",
            include_str!("../../../tests/tuple13-result-ir.json"),
        ),
    ] {
        let contract = parse_fixture(ir);
        // Complete old admission and native/current emission are mandatory.
        // Actual Rust execution is a separate generated-consumer gate.
        included(&contract, "read", label);
    }
}
