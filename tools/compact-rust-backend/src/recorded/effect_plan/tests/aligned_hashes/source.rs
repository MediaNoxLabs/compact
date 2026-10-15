// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Unchanged default-v2 source establishes whole-old premises before candidates.
use super::*;
use crate::recorded::admission_probe::trace;
use crate::recorded::profile_attempt::ProfileAttempt;

#[test]
fn genuine_aligned_hashes_compose_complete_unit_and_named_result_roots() {
    let source = include_str!("../../../../../tests/effect-plan-aligned-hashes-schema20-ir.json");
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
    let mut candidates = Vec::new();
    for name in ["close", "close_hash", "hashes", "hash_only"] {
        let root = circuits[name];
        let old = crate::recorded::typed_plan::lower_unit_composition_checked(
            root, &ledger, &witnesses, &pure, &circuits,
        );
        let old_status = match old {
            ProfileAttempt::Admitted(_) => "admitted",
            ProfileAttempt::Rejected(_) => "rejected",
            ProfileAttempt::NotApplicable => "not_applicable",
        };
        if matches!(name, "close" | "close_hash") {
            assert_eq!(old_status, "admitted", "whole old premise {name}");
        }
        let checked = check_with_calls(root, &ledger, &witnesses, &pure, &circuits).unwrap();
        let steps = render(&checked).unwrap().steps;
        let tokens = quote::quote!(#(#steps)*).to_string();
        assert!(!tokens.contains("call_local"));
        assert!(tokens.contains(if matches!(name, "close" | "hashes") {
            "hash_to_curve"
        } else {
            "persistent_hash"
        }));
        candidates.push(serde_json::json!({"root":name,"old":old_status,"checked":"supported"}));
    }
    let (output, rows) = trace::capture_json(|| crate::render_with_capabilities(&contract));
    let output = output.unwrap();
    let rows: Vec<_> = rows
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| {
            row["phase"] == "public_emission"
                && row["helper"] == false
                && matches!(
                    row["circuit"].as_str(),
                    Some("close" | "close_hash" | "hashes" | "hash_only")
                )
        })
        .collect();
    assert_eq!(rows.len(), 4);
    for row in &rows {
        assert_eq!(row["outcome"]["status"], "supported");
    }
    let metadata = serde_json::json!({"candidates":candidates,"public_emission":rows,"capabilities":output.capabilities,
        "claim":"Complete unchanged genuine default-v2 source; old premises checked before candidates. Native Rust compilation/runtime/proof qualified separately."});
    eprintln!(
        "ALIGNED_HASH_SOURCE {}",
        serde_json::to_string(&metadata).unwrap()
    );
    if let Some(output_path) = std::env::var_os("COMPACT_ALIGNED_HASH_OUTPUT") {
        let output_path = std::path::PathBuf::from(output_path);
        std::fs::create_dir(&output_path).unwrap();
        std::fs::write(output_path.join("compact-rust-ir.json"), source).unwrap();
        std::fs::write(output_path.join("lib.rs"), output.source).unwrap();
        std::fs::write(
            output_path.join("case.json"),
            serde_json::to_vec_pretty(&metadata).unwrap(),
        )
        .unwrap();
    }
}
