// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Synthetic complete-IR execution controls, distinct from the genuine TS fixture.
#![cfg(feature = "ledger-transaction")]
include!(concat!(env!("OUT_DIR"), "/synthetic.rs"));
#[path = "synthetic/common.rs"]
mod common;
#[path = "synthetic/order.rs"]
mod order;
#[path = "../../support/witness_only_recording.rs"]
#[allow(dead_code)]
mod support;
#[path = "synthetic/values.rs"]
mod values;
