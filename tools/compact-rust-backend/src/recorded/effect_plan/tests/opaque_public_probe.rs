// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Whole-old retained roots distinguish candidate progress from actual ownership.
use super::*;
use crate::recorded::admission_probe::trace;

fn zerocash() -> Contract {
    serde_json::from_str(include_str!(
        "../../../../tests/zerocash-oracle-schema20-ir.json"
    ))
    .unwrap()
}

fn root_outcome(c: &Contract, name: &str, expected_gap: Option<&str>, label: &str) {
    let ledger = c.ledger_fields.iter().map(|f| (f.id.as_str(), f)).collect();
    let witnesses = c.witnesses.iter().map(|w| (w.name.as_str(), w)).collect();
    let pure = c.circuits.iter().map(|r| (r.name.as_str(), r)).collect();
    let circuits: HashMap<_, _> = c
        .stateful_circuits
        .iter()
        .map(|r| (r.name.as_str(), r))
        .collect();
    let root = circuits[name];
    assert!(
        crate::recorded::typed_plan::lower(root, &ledger, &witnesses, &pure, &circuits).is_some(),
        "{label}: complete old membership oracle must accept first"
    );
    let candidate = match check_with_calls(root, &ledger, &witnesses, &pure, &circuits) {
        Ok(plan) => {
            assert!(
                expected_gap.is_none(),
                "{label}: expected refusal {expected_gap:?}"
            );
            render(&plan).unwrap();
            serde_json::json!({"accepted": true})
        }
        Err(PlanError::Unsupported(gap)) => {
            assert_eq!(Some(gap.detail.as_str()), expected_gap, "{label}: {gap:?}");
            serde_json::json!({"accepted": false, "path": gap.path, "detail": gap.detail})
        }
        Err(PlanError::Invalid(gap)) => panic!("{label}: invariant error {gap:?}"),
    };
    let (output, rows) = trace::capture_json(|| crate::render_with_capabilities(c));
    let output = output.unwrap_or_else(|e| panic!("{label}: complete native render {e:?}"));
    let rows = rows
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| {
            row["circuit"] == name && row["phase"] == "public_emission" && row["helper"] == false
        })
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), 1, "{label}");
    assert_eq!(
        rows[0]["body_owner"], "selector.membership-optional",
        "{label}"
    );
    let capability = output
        .capabilities
        .circuits
        .iter()
        .find(|r| r.name == name)
        .unwrap();
    assert!(capability.recorded && capability.observed_call, "{label}");
    if let Some(destination) = std::env::var_os("COMPACT_OPAQUE_PUBLIC_PROBE_OUTPUT") {
        let base = std::path::PathBuf::from(destination);
        std::fs::create_dir_all(&base).unwrap();
        let dir = base.join(label);
        std::fs::create_dir(&dir).expect("opaque probe export requires fresh case directory");
        std::fs::write(
            dir.join("compact-rust-ir.json"),
            serde_json::to_vec_pretty(c).unwrap(),
        )
        .unwrap();
        std::fs::write(dir.join("lib.rs"), output.source).unwrap();
        std::fs::write(dir.join("case.json"), serde_json::to_vec_pretty(&serde_json::json!({
            "root": name, "whole_old_accepted": true, "actual_owner": rows[0]["body_owner"],
            "candidate": candidate,
            "claim": "finite retained root progress; no whole-family inclusion or retirement claim"
        })).unwrap()).unwrap();
    }
}

#[test]
fn whole_old_opaque_roots_show_candidate_progress_without_retiring_the_owner() {
    let bboard: Contract =
        serde_json::from_str(include_str!("../../../../tests/bboard-schema20-ir.json")).unwrap();
    root_outcome(&bboard, "post", None, "bboard-post");
    root_outcome(&bboard, "take_down", None, "bboard-take-down");
    root_outcome(&zerocash(), "spend", None, "zerocash-spend");
}

#[test]
fn exact_zerocash_encrypt_declaration_uses_owned_callback_input_policy() {
    let c = zerocash();
    let encrypt = c
        .witnesses
        .iter()
        .find(|w| w.name == "context$encrypt")
        .unwrap();
    assert_eq!(encrypt.parameters.len(), 2);
    assert_eq!(encrypt.parameters[0].ty, Type::OpaqueBytes);
    assert_eq!(encrypt.result, Type::OpaqueBytes);
    assert!(validate::witness_result_type(&encrypt.result));
    assert!(!validate::surface_type(&encrypt.parameters[0].ty));
    assert!(validate::surface_type(&encrypt.parameters[1].ty));
    assert!(validate::witness_parameter_type(&encrypt.parameters[0].ty));
    // Direct opaque Default remains outside the expression domain. Complete
    // retained spend above supplies its real public_key projection instead.
}
