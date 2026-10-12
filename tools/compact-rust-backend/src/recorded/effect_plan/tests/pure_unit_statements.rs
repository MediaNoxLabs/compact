// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Complete legacy premises distinguish candidate gains from unchanged ownership.
use super::*;
use crate::recorded::admission_probe::trace;
use crate::recorded::profile_attempt::ProfileAttempt;
use serde_json::json;

#[path = "pure_unit_statements/source.rs"]
mod source;
#[path = "pure_unit_statements/validation.rs"]
mod validation;

fn probe(source: &str, entry: &str, label: &str, require_old: bool) {
    let contract: Contract = serde_json::from_str(source).unwrap();
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
    let stateful: HashMap<_, _> = contract
        .stateful_circuits
        .iter()
        .map(|v| (v.name.as_str(), v))
        .collect();
    let root = stateful[entry];
    let old = crate::recorded::typed_plan::lower_unit_composition_checked(
        root, &ledger, &witnesses, &pure, &stateful,
    );
    let old_status = match old {
        ProfileAttempt::Admitted(_) => "admitted",
        ProfileAttempt::NotApplicable => "not_applicable",
        ProfileAttempt::Rejected(_) => "rejected",
    };
    if require_old {
        assert_eq!(
            old_status, "admitted",
            "{label}: complete old root premise FIRST"
        );
    }
    let checked = match check_with_calls(root, &ledger, &witnesses, &pure, &stateful) {
        Ok(plan) => {
            render(&plan).unwrap();
            json!({"status":"supported"})
        }
        Err(PlanError::Unsupported(gap)) => {
            panic!("{label}: complete pure-statement and Map source must compose: {gap:?}");
        }
        Err(PlanError::Invalid(gap)) => panic!("{label}: invalid plan: {gap:?}"),
    };
    let (rendered, invocations) =
        trace::capture_json(|| crate::render_with_capabilities(&contract));
    let rendered =
        rendered.unwrap_or_else(|error| panic!("{label}: full native render: {error:?}"));
    let rows = invocations
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| {
            row["circuit"] == entry && row["phase"] == "public_emission" && row["helper"] == false
        })
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), 1, "{label}: one actual public emission");
    // Existing membership priority covers one Enum formal and Boolean-only
    // roots. A String+Enum signature and the two now-composing Map roots keep
    // their existing unit-composition owner; candidate success is independent.
    let expected_owner = if label.starts_with("retained-guard-") || !require_old {
        "algebra.membership"
    } else {
        "selector.unit-composition"
    };
    assert_eq!(rows[0]["body_owner"], expected_owner);
    assert_eq!(rows[0]["outcome"]["status"], "supported");
    let metadata = json!({
        "label":label,"root":entry,"internal":root.internal,
        "whole_old_required":require_old,"whole_old_unit_composition":old_status,
        "checked":checked,
        "full_native_render":true,"public_emission":rows[0],"capabilities":rendered.capabilities,
        "claim":"Whole-root candidate observation and full native generation; compilation, runtime and proof are separate."
    });
    eprintln!(
        "PURE_UNIT_STATEMENT_PROBE {}",
        serde_json::to_string(&metadata).unwrap()
    );
    if let Some(base) = std::env::var_os("COMPACT_PURE_UNIT_STATEMENTS_OUTPUT") {
        let directory = std::path::PathBuf::from(base).join(label);
        std::fs::create_dir_all(directory.parent().unwrap()).unwrap();
        std::fs::create_dir(&directory).expect("fresh export directory required");
        std::fs::write(directory.join("compact-rust-ir.json"), source).unwrap();
        std::fs::write(directory.join("lib.rs"), rendered.source).unwrap();
        std::fs::write(
            directory.join("case.json"),
            serde_json::to_vec_pretty(&metadata).unwrap(),
        )
        .unwrap();
    }
}
