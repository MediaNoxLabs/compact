// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! The whole native-valid old-positive source precedes the checked candidate.
use super::*;
use crate::recorded::admission_probe::trace;
use crate::recorded::profile_attempt::ProfileAttempt;

#[test]
fn genuine_local_keccak_old_positive_now_lowers_without_native_helper_escape() {
    let source = include_str!("../../../../../tests/unit-local-keccak-schema20-ir.json");
    let contract: Contract = serde_json::from_str(source).unwrap();
    let ledger = contract
        .ledger_fields
        .iter()
        .map(|x| (x.id.as_str(), x))
        .collect();
    let witnesses = contract
        .witnesses
        .iter()
        .map(|x| (x.name.as_str(), x))
        .collect();
    let pure = contract
        .circuits
        .iter()
        .map(|x| (x.name.as_str(), x))
        .collect();
    let circuits: HashMap<_, _> = contract
        .stateful_circuits
        .iter()
        .map(|x| (x.name.as_str(), x))
        .collect();
    let root = circuits["close"];
    assert!(
        matches!(
            crate::recorded::typed_plan::lower_unit_composition_checked(
                root, &ledger, &witnesses, &pure, &circuits
            ),
            ProfileAttempt::Admitted(_)
        ),
        "complete unchanged old root must qualify before candidate"
    );
    let checked = check_with_calls(root, &ledger, &witnesses, &pure, &circuits).unwrap();
    let steps = render(&checked).unwrap().steps;
    let tokens = quote::quote!(#(#steps)*).to_string();
    assert_eq!(tokens.matches("keccak256").count(), 1);
    assert!(!tokens.contains("call_local"));
    let (rendered, rows) = trace::capture_json(|| crate::render_with_capabilities(&contract));
    let rendered = rendered.unwrap();
    let rows: Vec<_> = rows
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| {
            r["circuit"] == "close" && r["phase"] == "public_emission" && r["helper"] == false
        })
        .collect();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["body_owner"], "algebra.kernel");
    assert_eq!(rows[0]["outcome"]["status"], "supported");
    if let Some(output) = std::env::var_os("COMPACT_KECCAK_SOURCE_OUTPUT") {
        let output = std::path::PathBuf::from(output);
        std::fs::create_dir(&output).unwrap();
        std::fs::write(output.join("compact-rust-ir.json"), source).unwrap();
        std::fs::write(output.join("lib.rs"), rendered.source).unwrap();
        std::fs::write(output.join("case.json"), serde_json::to_vec_pretty(&serde_json::json!({
            "whole_old_unit_composition": "admitted", "checked": "supported", "public_emission": rows[0],
            "capabilities": rendered.capabilities,
            "claim": "Unchanged genuine frontend v3 IR; old root and checked body both supported, earlier algebra.kernel now owns the complete root; legacy selector remains available for other roots."
        })).unwrap()).unwrap();
    }
}
