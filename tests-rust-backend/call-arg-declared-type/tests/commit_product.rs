// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
#![cfg(feature = "ledger-transaction")]
//! Synthetic compiled fixed-product control; no new source, TS, or proof claim.
include!(concat!(env!("OUT_DIR"), "/product.rs"));
#[path = "product/execution.rs"]
mod execution;
