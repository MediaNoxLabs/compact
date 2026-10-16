// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
#![cfg(feature = "ledger-transaction")]
use compact_rust_did_digest_read_reducer_fixture::{ledger_contract as c, runtime as r};
use r::context::WitnessContext;
use serde_json::{Value, json};
use std::cell::RefCell;
fn state(row: &Value) -> r::ledger::ContractState<r::ledger::DefaultDB> {
    midnight_serialize::tagged_deserialize(
        &mut hex::decode(row["before"].as_str().unwrap())
            .unwrap()
            .as_slice(),
    )
    .unwrap()
}
#[path = "digest_qualification/mod.rs"]
mod qualification;
