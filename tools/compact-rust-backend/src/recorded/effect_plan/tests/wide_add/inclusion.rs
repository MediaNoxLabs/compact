// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Whole old merge admission precedes candidate checks; no family retirement claim.
use super::*;
#[test]
fn complete_old_merge_roots_now_compose_and_render() {
    let source: Contract = serde_json::from_str(include_str!(
        "../../../../../tests/shielded-merge-schema20-ir.json"
    ))
    .unwrap();
    let ledger = source
        .ledger_fields
        .iter()
        .map(|f| (f.id.as_str(), f))
        .collect();
    let witnesses = source
        .witnesses
        .iter()
        .map(|w| (w.name.as_str(), w))
        .collect();
    let pure = source
        .circuits
        .iter()
        .map(|c| (c.name.as_str(), c))
        .collect();
    let circuits: HashMap<_, _> = source
        .stateful_circuits
        .iter()
        .map(|c| (c.name.as_str(), c))
        .collect();
    for name in ["merge_qualified", "receive_then_merge"] {
        let root = circuits[name];
        assert!(
            crate::recorded::typed_plan::lower_shielded_merge(
                root, &ledger, &witnesses, &pure, &circuits
            )
            .is_some(),
            "{name}: mandatory complete old oracle acceptance"
        );
        let checked = check_with_calls(root, &ledger, &witnesses, &pure, &circuits)
            .unwrap_or_else(|e| panic!("{name}: next candidate gap {e:?}"));
        render(&checked).unwrap();
    }
    let output = crate::render_with_capabilities(&source).unwrap();
    if let Some(base) = std::env::var_os("COMPACT_WIDE_ADD_PROBE_OUTPUT") {
        let base = std::path::PathBuf::from(base);
        std::fs::create_dir_all(&base).unwrap();
        let dir = base.join("whole-old-merge");
        std::fs::create_dir(&dir).expect("use a fresh probe output directory");
        std::fs::write(
            dir.join("compact-rust-ir.json"),
            serde_json::to_vec_pretty(&source).unwrap(),
        )
        .unwrap();
        std::fs::write(dir.join("lib.rs"), output.source).unwrap();
    }
}
#[test]
fn genuine_wide_add_source_uses_existing_composite_ownership() {
    let source: Contract = serde_json::from_str(include_str!(
        "../../../../../tests/effect-plan-wide-add-schema20-ir.json"
    ))
    .unwrap();
    let (output, rows) = crate::recorded::admission_probe::trace::capture_json(|| {
        crate::render_with_capabilities(&source)
    });
    let output = output.unwrap();
    assert_eq!(output.capabilities.circuits.len(), 4);
    for name in ["add128", "mixed", "narrow128", "witnessed"] {
        let capability = output
            .capabilities
            .circuits
            .iter()
            .find(|c| c.name == name)
            .unwrap();
        assert!(capability.recorded && capability.observed_call, "{name}");
        let owners: Vec<_> = rows
            .as_array()
            .unwrap()
            .iter()
            .filter(|row| {
                row["circuit"] == name
                    && row["phase"] == "public_emission"
                    && row["helper"] == false
            })
            .collect();
        assert_eq!(owners.len(), 1, "{name}");
        assert_eq!(owners[0]["body_owner"], "algebra.ordinary", "{name}");
    }
}
