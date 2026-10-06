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

use crate::runtime::CompactError;
use std::{error::Error, fmt};

/// A failed invocation never commits its working state to the lab.
/// Debug/Display deliberately omit circuit and VM error payloads.
pub enum LabError {
    Execution(CompactError),
    Replay(String),
    SnapshotMismatch(&'static str),
    UnsupportedState,
    EnvironmentChanged,
    InitialContextMismatch,
    IdentityMismatch,
    ReplayStateMismatch,
    ReplayEffectsMismatch,
}
impl LabError {
    /// Explicit access to a potentially private application error message.
    pub fn execution_error(&self) -> Option<&CompactError> {
        match self {
            Self::Execution(error) => Some(error),
            _ => None,
        }
    }
    /// Explicit access to a potentially private VM diagnostic.
    pub fn replay_error(&self) -> Option<&str> {
        match self {
            Self::Replay(error) => Some(error),
            _ => None,
        }
    }
}
impl fmt::Display for LabError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Execution(_) => f.write_str("circuit execution failed (details redacted)"),
            Self::Replay(_) => f.write_str("VM replay failed (details redacted)"),
            Self::SnapshotMismatch(field) => write!(f, "snapshot {field} mismatch"),
            Self::UnsupportedState => {
                f.write_str("plain-ledger scenarios require default intents and an empty wallet")
            }
            Self::EnvironmentChanged => {
                f.write_str("call changed the configured execution environment")
            }
            Self::InitialContextMismatch => {
                f.write_str("recorded call starts from a different context")
            }
            Self::IdentityMismatch => f.write_str("recorded execution identity mismatch"),
            Self::ReplayStateMismatch => f.write_str("replayed state differs from execution"),
            Self::ReplayEffectsMismatch => f.write_str("replayed effects differ from execution"),
        }
    }
}
impl fmt::Debug for LabError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
// Do not expose private error payloads through generic error-chain printers.
impl Error for LabError {}
