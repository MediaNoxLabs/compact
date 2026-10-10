// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Baseline evidence only: candidate refusals are not actual-owner failures.
use super::*;
use crate::recorded::admission_probe::trace;

fn zerocash() -> Contract {
    serde_json::from_str(include_str!(
        "../../../../tests/zerocash-oracle-schema20-ir.json"
    ))
    .unwrap()
}

fn root_gap(c: &Contract, name: &str, path: &str, detail: &str, label: &str) {
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
    let error = match check_with_calls(root, &ledger, &witnesses, &pure, &circuits) {
        Err(PlanError::Unsupported(gap)) => gap,
        other => panic!("{label}: expected source refusal, got {other:?}"),
    };
    assert_eq!(error.path, path, "{label}");
    assert_eq!(error.detail, detail, "{label}");
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
            "candidate_refusal": {"path": error.path, "detail": error.detail},
            "claim": "baseline first refusal; no hypothetical root-only or whole-family inclusion claim"
        })).unwrap()).unwrap();
    }
}

#[test]
fn whole_old_opaque_roots_have_precise_candidate_boundary_refusals() {
    let bboard: Contract =
        serde_json::from_str(include_str!("../../../../tests/bboard-schema20-ir.json")).unwrap();
    root_gap(
        &bboard,
        "post",
        "parameters[0]",
        "unsupported parameter type",
        "bboard-post",
    );
    root_gap(
        &bboard,
        "take_down",
        "return_value",
        "unsupported circuit result type",
        "bboard-take-down",
    );
    root_gap(
        &zerocash(),
        "spend",
        "parameters[0]",
        "unsupported parameter type",
        "zerocash-spend",
    );
}

#[test]
fn exact_zerocash_encrypt_declaration_has_an_independent_witness_parameter_gap() {
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
    // Isolate the unchanged source declaration under an ordinary Unit interface.
    // No mutated public root or hypothetical production eligibility is involved.
    let root = StatefulCircuit {
        source: None,
        name: "isolated_encrypt_policy".into(),
        internal: false,
        parameters: vec![],
        result: Type::Unit,
        return_value: StateReturn::Unit,
        actions: vec![StateAction::Expression {
            value: Expr::WitnessCall {
                name: encrypt.name.clone(),
                arguments: encrypt
                    .parameters
                    .iter()
                    .map(|p| Expr::Default { ty: p.ty.clone() })
                    .collect(),
            },
        }],
    };
    let gap = match check(
        &root,
        &HashMap::new(),
        &HashMap::from([(encrypt.name.as_str(), encrypt)]),
    ) {
        Err(PlanError::Unsupported(gap)) => gap,
        other => panic!("expected isolated witness policy refusal, got {other:?}"),
    };
    assert_eq!(gap.detail, "unsupported witness parameter type");
    // This establishes a separate live policy refusal. It does not assert that
    // all preceding/following nodes of the complete spend body already compose.
}
