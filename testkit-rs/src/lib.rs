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

//! Typed in-memory scenarios using generated Compact calls and the real ledger VM.
//!
//! Native execution and verified local replay are distinct report levels. Neither
//! is transaction preparation, cryptographic proving, or network acceptance.
//! Adapters are trusted Rust orchestration of generated calls, not sandboxed code.
//! The lab checks the supplied starting query and returned environment plus sealed
//! identity/intents and replay state/effects. It cannot detect a callback temporarily
//! changing gas/cost/identity policy and restoring it before returning. Replay gas
//! is measured separately; it does not enforce an aggregate execution budget.
//!
//! Private checkpoints require owned data (or immutable persistence): `Clone`
//! cannot undo external witness side effects or shared interior mutation.

mod environment;
mod error;
mod lab;
mod report;
mod snapshot;
mod witness;

pub use environment::Environment;
pub use error::LabError;
pub use lab::ContractLab;
pub use midnight_compact_runtime as runtime;
pub use report::{CallReport, ReplayReport};
pub use snapshot::{ArtifactIdentity, SNAPSHOT_VERSION, Snapshot, SnapshotMetadata};
pub use witness::WitnessScript;
