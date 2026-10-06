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

use crate::{
    Environment, LabError,
    runtime::{
        LEDGER_VERSION, RUST_RUNTIME_ABI,
        ledger::{ChargedState, DefaultDB},
    },
};
use std::fmt;

pub const SNAPSHOT_VERSION: u32 = 1;

/// Caller-declared artifact identities detect mistakes; they do not authenticate provenance.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArtifactIdentity {
    pub source_sha256: [u8; 32],
    pub generated_sha256: [u8; 32],
}

/// Inspectable metadata for the in-memory snapshot format. Restore validates every field.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SnapshotMetadata {
    pub version: u32,
    pub artifacts: ArtifactIdentity,
    pub runtime_abi: u32,
    pub ledger_version: String,
    pub state_mode: String,
}
impl SnapshotMetadata {
    pub(crate) fn new(artifacts: ArtifactIdentity) -> Self {
        Self {
            version: SNAPSHOT_VERSION,
            artifacts,
            runtime_abi: RUST_RUNTIME_ABI,
            ledger_version: LEDGER_VERSION.into(),
            state_mode: "plain-ledger".into(),
        }
    }
    pub(crate) fn validate(&self, expected: &Self) -> Result<(), LabError> {
        for (matches, field) in [
            (self.version == expected.version, "version"),
            (
                self.artifacts.source_sha256 == expected.artifacts.source_sha256,
                "source",
            ),
            (
                self.artifacts.generated_sha256 == expected.artifacts.generated_sha256,
                "generated artifact",
            ),
            (self.runtime_abi == expected.runtime_abi, "runtime ABI"),
            (
                self.ledger_version == expected.ledger_version,
                "ledger version",
            ),
            (self.state_mode == expected.state_mode, "state mode"),
        ] {
            if !matches {
                return Err(LabError::SnapshotMismatch(field));
            }
        }
        Ok(())
    }
}

/// Owned in-memory checkpoint. No serialization or durable private files are implied.
#[derive(Clone)]
pub struct Snapshot<P> {
    pub metadata: SnapshotMetadata,
    pub(crate) environment: Environment,
    pub(crate) state: ChargedState<DefaultDB>,
    pub(crate) private: P,
}
impl<P> Snapshot<P> {
    pub fn public_state(&self) -> &ChargedState<DefaultDB> {
        &self.state
    }
    /// Explicit access; keep secrets out of assertion messages and logs.
    pub fn private_state(&self) -> &P {
        &self.private
    }
}
impl<P> fmt::Debug for Snapshot<P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Snapshot")
            .field("metadata", &self.metadata)
            .finish_non_exhaustive()
    }
}
