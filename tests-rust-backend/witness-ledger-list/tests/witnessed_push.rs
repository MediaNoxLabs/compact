// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
#![cfg(feature = "ledger-transaction")]
//! Explicit synthetic complete-IR composition law; no source/TS/proof claim.
include!(concat!(env!("OUT_DIR"), "/witnessed_push.rs"));
#[path = "witnessed_push/ordering.rs"]
mod ordering;
