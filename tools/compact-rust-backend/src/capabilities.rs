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

//! Capability metadata and its atomic join with frontend proof applicability.
//!
//! Lowering decides which methods can be emitted. This component reports those
//! decisions and joins the frontend proof flags without changing admission.

use crate::{RecordingGap, ir};
use serde::Serialize;
use serde_json::Value;

pub const RUST_CAPABILITY_SCHEMA_VERSION: u32 = 3;

/// Compiler metadata for the developer-facing Rust proving surface.
#[derive(Debug, Clone, Serialize)]
pub struct RustCapabilityReport {
    pub schema_version: u32,
    pub circuits: Vec<RustCircuitCapability>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RustCircuitCapability {
    pub name: String,
    pub source: Option<ir::SourceLocation>,
    pub recorded: bool,
    pub observed_call: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proof_required: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recording_status: Option<RecordingStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recording_unavailable: Option<RecordingGap>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub observed_call_unavailable: Option<RecordingGap>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RecordingStatus {
    Available,
    Unavailable,
    NotApplicable,
}

impl RustCapabilityReport {
    /// Finish the report with the frontend's authoritative proof applicability.
    /// Contract-info may also contain nonexported helpers, which are ignored.
    pub fn apply_contract_info(&mut self, contract_info: &Value) -> Result<(), String> {
        if self.schema_version != 2 {
            return Err(format!(
                "capability report must be an unclassified schema-2 draft, got schema {}",
                self.schema_version
            ));
        }
        let circuits = contract_info
            .get("circuits")
            .and_then(Value::as_array)
            .ok_or("contract-info.json has no circuits array")?;
        let mut proof_flags = Vec::with_capacity(self.circuits.len());
        for capability in &self.circuits {
            let matches = circuits
                .iter()
                .filter(|entry| {
                    entry.get("name").and_then(Value::as_str) == Some(capability.name.as_str())
                })
                .collect::<Vec<_>>();
            if matches.len() != 1 {
                return Err(format!(
                    "contract-info.json must contain exactly one entry for exported circuit {:?}; found {}",
                    capability.name,
                    matches.len()
                ));
            }
            let proof_required = matches[0]
                .get("proof")
                .and_then(Value::as_bool)
                .ok_or_else(|| {
                    format!(
                        "contract-info.json circuit {:?} has no Boolean proof flag",
                        capability.name
                    )
                })?;
            proof_flags.push(proof_required);
        }
        for (capability, proof_required) in self.circuits.iter_mut().zip(proof_flags) {
            capability.proof_required = Some(proof_required);
            capability.recording_status = Some(if !proof_required {
                RecordingStatus::NotApplicable
            } else if capability.recorded && capability.observed_call {
                RecordingStatus::Available
            } else {
                RecordingStatus::Unavailable
            });
        }
        self.schema_version = RUST_CAPABILITY_SCHEMA_VERSION;
        Ok(())
    }
}

#[cfg(test)]
mod proof_applicability_tests {
    use super::*;
    use serde_json::json;

    fn report() -> RustCapabilityReport {
        RustCapabilityReport {
            schema_version: 2,
            circuits: vec![RustCircuitCapability {
                name: "exported".into(),
                source: None,
                recorded: false,
                observed_call: false,
                proof_required: None,
                recording_status: None,
                recording_unavailable: None,
                observed_call_unavailable: None,
            }],
        }
    }

    #[test]
    fn proof_join_allows_extra_nonexported_helpers() {
        let mut report = report();
        report
            .apply_contract_info(&json!({"circuits": [
                {"name": "helper", "proof": false},
                {"name": "exported", "proof": true}
            ]}))
            .unwrap();
        assert_eq!(report.schema_version, 3);
        assert_eq!(report.circuits[0].proof_required, Some(true));
        assert_eq!(
            report.circuits[0].recording_status,
            Some(RecordingStatus::Unavailable)
        );
        let serialized = serde_json::to_value(&report).unwrap();
        assert_eq!(serialized["circuits"][0]["recording_status"], "unavailable");
    }

    #[test]
    fn nonproof_row_is_not_applicable_even_when_api_missing() {
        let mut report = report();
        report
            .apply_contract_info(&json!({"circuits": [{"name": "exported", "proof": false}]}))
            .unwrap();
        assert_eq!(
            report.circuits[0].recording_status,
            Some(RecordingStatus::NotApplicable)
        );
    }

    #[test]
    fn proof_join_rejects_missing_duplicate_and_non_boolean_flags() {
        for metadata in [
            json!({"circuits": [{"name": "helper", "proof": false}]}),
            json!({"circuits": [{"name": "exported", "proof": true}, {"name": "exported", "proof": false}]}),
            json!({"circuits": [{"name": "exported", "proof": "true"}]}),
        ] {
            let mut report = report();
            assert!(report.apply_contract_info(&metadata).is_err());
            assert_eq!(report.schema_version, 2);
        }
    }
}
