// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//  	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Direct command argument parsing and execution for material proof scenarios.
//!
//! Each handler retains its original argument checks and worker policy.

use std::error::Error;
use std::ffi::{OsStr, OsString};

use crate::proof_material;

pub(super) fn prepare_proof_material(
    arguments: &mut impl Iterator<Item = OsString>,
) -> Result<(), Box<dyn Error>> {
    let verify_only = match arguments.next().as_deref() {
        None => false,
        Some(value) if value == OsStr::new("--verify-only") => true,
        _ => {
            return Err(
                "usage: compact-rust-proof-smoke --prepare-proof-material [--verify-only]".into(),
            );
        }
    };
    if arguments.next().is_some() {
        return Err(
            "usage: compact-rust-proof-smoke --prepare-proof-material [--verify-only]".into(),
        );
    }
    proof_material::run(verify_only)
}
