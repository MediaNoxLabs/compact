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

    fn unchanged_on_error(report: &mut RustCapabilityReport, metadata: Value) -> String {
        let before = serde_json::to_value(&*report).unwrap();
        let error = report.apply_contract_info(&metadata).unwrap_err();
        assert_eq!(serde_json::to_value(&*report).unwrap(), before);
        error
    }

    #[test]
    fn late_metadata_failure_does_not_partially_classify_exports() {
        for second in [
            json!([]),
            json!([{"name": "later", "proof": null}]),
            json!([{"name": "later", "proof": 1}]),
            json!([{"name": "later", "proof": true}, {"name": "later", "proof": false}]),
        ] {
            let mut report = report();
            let mut later = report.circuits[0].clone();
            later.name = "later".into();
            report.circuits.push(later);
            let mut metadata = vec![json!({"name": "exported", "proof": true})];
            metadata.extend(second.as_array().unwrap().iter().cloned());
            let error = unchanged_on_error(&mut report, json!({"circuits": metadata}));
            assert!(error.contains("later"), "{error}");
        }
    }

    #[test]
    fn schema_guard_rejects_unknown_and_already_classified_reports_atomically() {
        for version in [0, 1, 3, 4, u32::MAX] {
            let mut report = report();
            report.schema_version = version;
            let error = unchanged_on_error(
                &mut report,
                json!({"circuits": [{"name": "exported", "proof": true}]}),
            );
            assert!(error.contains("unclassified schema-2 draft"), "{error}");
        }
        let mut report = report();
        report
            .apply_contract_info(&json!({"circuits": [{"name": "exported", "proof": false}]}))
            .unwrap();
        unchanged_on_error(
            &mut report,
            json!({"circuits": [{"name": "exported", "proof": true}]}),
        );
    }

    #[test]
    fn malformed_metadata_container_leaves_the_draft_unchanged() {
        for metadata in [
            json!(null),
            json!([]),
            json!({}),
            json!({"circuits": null}),
            json!({"circuits": {}}),
            json!({"circuits": "not an array"}),
        ] {
            let mut report = report();
            assert_eq!(
                unchanged_on_error(&mut report, metadata),
                "contract-info.json has no circuits array"
            );
        }
    }

    #[test]
    fn proof_and_both_api_flags_determine_recording_status() {
        for proof in [false, true] {
            for recorded in [false, true] {
                for observed_call in [false, true] {
                    let mut report = report();
                    report.circuits[0].recorded = recorded;
                    report.circuits[0].observed_call = observed_call;
                    report
                        .apply_contract_info(
                            &json!({"circuits": [{"name": "exported", "proof": proof}]}),
                        )
                        .unwrap();
                    let expected = match (proof, recorded, observed_call) {
                        (false, _, _) => RecordingStatus::NotApplicable,
                        (true, true, true) => RecordingStatus::Available,
                        _ => RecordingStatus::Unavailable,
                    };
                    let row = &report.circuits[0];
                    assert_eq!(row.recording_status, Some(expected));
                    assert_eq!(row.proof_required, Some(proof));
                    assert_eq!((row.recorded, row.observed_call), (recorded, observed_call));
                    assert_eq!(report.schema_version, RUST_CAPABILITY_SCHEMA_VERSION);
                }
            }
        }
    }

    #[test]
    fn metadata_join_uses_names_and_preserves_export_order() {
        let mut report = report();
        let mut second = report.circuits[0].clone();
        second.name = "other".into();
        report.circuits.push(second);
        report
            .apply_contract_info(&json!({"circuits": [
                {"name": "other", "proof": false},
                {"name": "helper"},
                {"name": "helper", "proof": "irrelevant"},
                {"name": "exported", "proof": true}
            ]}))
            .unwrap();
        let actual: Vec<_> = report
            .circuits
            .iter()
            .map(|row| (row.name.as_str(), row.proof_required))
            .collect();
        assert_eq!(actual, [("exported", Some(true)), ("other", Some(false))]);
    }

    #[test]
    fn empty_export_set_still_requires_valid_metadata_shape() {
        let mut report = RustCapabilityReport {
            schema_version: 2,
            circuits: vec![],
        };
        unchanged_on_error(&mut report, json!({}));
        report
            .apply_contract_info(&json!({"circuits": []}))
            .unwrap();
        assert_eq!(
            serde_json::to_value(&report).unwrap(),
            json!({"schema_version": 3, "circuits": []})
        );
    }

    #[test]
    fn draft_serialization_omits_unclassified_optional_fields() {
        let json = serde_json::to_value(report()).unwrap();
        let row = json["circuits"][0].as_object().unwrap();
        for key in [
            "proof_required",
            "recording_status",
            "recording_unavailable",
            "observed_call_unavailable",
        ] {
            assert!(!row.contains_key(key), "draft unexpectedly published {key}");
        }
        assert_eq!(row["recorded"], false);
        assert_eq!(row["observed_call"], false);
    }
}
