// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Whole-old premise, checked result, actual route and raw-consumer status stay distinct.

use super::inputs::{Input, digest};
use crate::{
    ir::Contract,
    recorded::{admission_probe::trace, effect_plan, profile_attempt::ProfileAttempt, typed_plan},
};
use serde_json::{Value, json};
use std::{collections::HashMap, path::Path};

pub(super) fn measure(input: &Input, output: &Path) -> Value {
    let case = &input.case;
    let contract: Contract = match serde_json::from_slice(&input.bytes) {
        Ok(contract) => contract,
        Err(error) => {
            return json!({
                "id":case.id,"rules":case.rules,"root":case.root,"input_file":case.file,
                "input_sha256":case.sha256,"intent":"exploratory_rule_control_not_presumed_old_positive",
                "classification":"malformed_IR_control", "IR_decode_error":error.to_string(),
                "old":{"status":"not_evaluated_invalid_IR"},
                "checked":{"status":"not_evaluated_invalid_IR"},
                "native":{"status":"not_evaluated_invalid_IR"},
                "rust_compilation":{"status":"not_run"},"export":null,
            });
        }
    };
    let ledger = contract
        .ledger_fields
        .iter()
        .map(|v| (v.id.as_str(), v))
        .collect();
    let witnesses = contract
        .witnesses
        .iter()
        .map(|v| (v.name.as_str(), v))
        .collect();
    let pure = contract
        .circuits
        .iter()
        .map(|v| (v.name.as_str(), v))
        .collect();
    let circuits: HashMap<_, _> = contract
        .stateful_circuits
        .iter()
        .map(|v| (v.name.as_str(), v))
        .collect();
    let root = circuits[case.root.as_str()];
    // This must remain first: grammar acceptance or actual ownership alone
    // cannot establish inclusion of the complete former implementation.
    let old = match typed_plan::lower_unit_composition_checked(
        root, &ledger, &witnesses, &pure, &circuits,
    ) {
        ProfileAttempt::Admitted(_) => json!({"status":"admitted"}),
        ProfileAttempt::NotApplicable => json!({"status":"not_applicable"}),
        // Debug includes the full gap, root action, precision and rejection phase
        // without widening visibility or the public capabilities JSON schema.
        ProfileAttempt::Rejected(rejection) => {
            json!({"status":"rejected","rejection_debug":format!("{rejection:?}")})
        }
    };
    let checked = match effect_plan::lower_with_calls(root, &ledger, &witnesses, &pure, &circuits) {
        Ok(_) => json!({"status":"supported"}),
        Err(effect_plan::PlanError::Unsupported(gap)) => json!({"status":"unsupported","gap":gap}),
        Err(effect_plan::PlanError::Invalid(gap)) => json!({"status":"invalid","gap":gap}),
    };
    let (rendered, invocations) =
        trace::capture_json(|| crate::render_with_capabilities(&contract));
    let mut export = None;
    let native = match rendered {
        Ok(rendered) => {
            let public: Vec<_> = invocations
                .as_array()
                .unwrap()
                .iter()
                .filter(|v| {
                    v["circuit"] == case.root
                        && v["phase"] == "public_emission"
                        && v["helper"] == false
                })
                .collect();
            let capability = rendered
                .capabilities
                .circuits
                .iter()
                .find(|c| c.name == case.root);
            if old["status"] == "admitted" {
                let directory = format!("exports/{}", case.id);
                let target = output.join(&directory);
                std::fs::create_dir(&target).unwrap();
                std::fs::write(target.join("compact-rust-ir.json"), &input.bytes).unwrap();
                std::fs::write(target.join("lib.rs"), &rendered.source).unwrap();
                export = Some(
                    json!({"directory":directory,"ir_sha256":digest(&input.bytes),"rust_sha256":digest(rendered.source.as_bytes())}),
                );
            }
            json!({"status":"render_accepted","capability":capability,"public_emission":public})
        }
        Err(error) => {
            json!({"status":"render_rejected","error":error.to_string(),"error_debug":format!("{error:?}"),"invocations":invocations})
        }
    };
    let classification = if native["status"] != "render_accepted" {
        "native_invalid_control"
    } else if old["status"] == "admitted" {
        if checked["status"] == "supported" {
            "old_admitted_candidate_supported"
        } else {
            "old_admitted_candidate_gap"
        }
    } else if checked["status"] == "supported" {
        "candidate_expansion_control"
    } else {
        "refusal_control"
    };
    let source_authority = if [
        "persistent_hash-string",
        "persistent_hash-opaque_bytes",
        "persistent_hash-struct",
        "persistent_hash-nested",
    ]
    .contains(&case.id.as_str())
    {
        "historical_IR_opaque_persistent_hash_frontend_rejects_type_family_exclusion_requires_decision"
    } else {
        "synthetic_IR_not_a_genuine_source_claim"
    };
    let row = json!({"id":case.id,"rules":case.rules,"root":case.root,"input_file":case.file,
        "input_sha256":case.sha256,"intent":"exploratory_rule_control_not_presumed_old_positive",
        "source_authority":source_authority,"old":old,"checked":checked,"native":native,
        "classification":classification,"rust_compilation":{"status":"not_run"},"export":export});
    if let Some(export) = export {
        let path = output
            .join(export["directory"].as_str().unwrap())
            .join("case.json");
        std::fs::write(path, serde_json::to_vec_pretty(&row).unwrap()).unwrap();
    }
    row
}
