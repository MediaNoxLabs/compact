// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Complete retained frontend value-effects source, emitted without modification.
pub const RETAINED_IR: &str = include_str!("retained-ir.json");
include!(concat!(env!("OUT_DIR"), "/contract.rs"));
