// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;

#[test]
fn retained_whole_sources_and_helper_planning_keep_semantics_and_native_precedence() {
    for source in [
        include_str!("../../../../tests/schnorr-attestation-schema20-ir.json"),
        include_str!("../../../../tests/asset-registry-schema20-ir.json"),
    ] {
        let c: Contract = serde_json::from_str(source).unwrap();
        // Each direct comparison uses the whole immutable declaration universe.
        // Internal helpers are qualified independently of the public visibility gate.
        for root in &c.stateful_circuits {
            equivalent(&c, &root.name);
        }
        let (output, rows) = trace::capture_json(|| crate::render_with_capabilities(&c));
        let output = output.unwrap();
        assert!(
            output
                .capabilities
                .circuits
                .iter()
                .all(|v| v.recorded && v.observed_call)
        );
        assert!(
            rows.as_array()
                .unwrap()
                .iter()
                .any(|v| v["phase"] == "helper_planning")
        );
        for row in rows.as_array().unwrap() {
            if row["body_owner"] == "algebra.ordinary" {
                let attempts = row["attempts"].as_array().unwrap();
                assert_eq!(attempts.len(), 1);
                assert_eq!(attempts[0]["route"], "algebra.ordinary");
            }
        }
        let mut invalid = c.clone();
        let public = invalid
            .stateful_circuits
            .iter_mut()
            .find(|v| !v.internal)
            .unwrap();
        public.name = "invalid-root-name".into();
        assert!(crate::render_with_capabilities(&invalid).is_err());
    }
    // Resource acceptance is exercised by the existing calibrated ordinary and
    // recursive worker gates. These differential laws introduce no new budget.
    assert!(std::mem::size_of::<effect_plan::LoweringRejection>() <= 120);
}
