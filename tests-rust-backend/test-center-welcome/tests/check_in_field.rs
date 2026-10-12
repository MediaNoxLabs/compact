// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
#![cfg(feature = "ledger-transaction")]
include!(concat!(env!("OUT_DIR"), "/check_in_field.rs"));
type WitnessResult = runtime::Field;
fn witness_result() -> WitnessResult {
    runtime::Field::from(42_u64)
}
#[path = "common/check_in_sequence.rs"]
mod checks;

fn expected_private_output() -> runtime::fab::AlignedValue {
    runtime::fab::AlignedValue::from(runtime::Field::from(42_u64))
}
