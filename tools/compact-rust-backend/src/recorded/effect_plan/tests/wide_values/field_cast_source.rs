// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Direct wide Field casts: full source retained for independent consumer qualification.
use super::*;
use crate::recorded::admission_probe::trace;
use crate::recorded::profile_attempt::ProfileAttempt;
use serde_json::json;

#[test]
fn genuine_wide_field_casts_compose_with_counter_and_lazy_witnesses() {
    let source = include_str!("../../../../../tests/effect-plan-wide-field-cast-schema20-ir.json");
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
    for entry in ["cast129", "cast248", "witness129", "witness248"] {
        let root = stateful[entry];
        let old_status = match crate::recorded::typed_plan::lower_unit_composition_checked(
            root, &ledger, &witnesses, &pure, &stateful,
        ) {
            ProfileAttempt::Admitted(_) => "admitted",
            ProfileAttempt::NotApplicable => "not_applicable",
            ProfileAttempt::Rejected(_) => "rejected",
        };
        let checked = check_with_calls(root, &ledger, &witnesses, &pure, &stateful)
            .unwrap_or_else(|gap| panic!("{entry}: direct wide Field cast must compose: {gap:?}"));
        render(&checked).unwrap();
        let (rendered, invocations) =
            trace::capture_json(|| crate::render_with_capabilities(&contract));
        let rendered = rendered.unwrap();
        let rows: Vec<_> = invocations
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| {
                r["circuit"] == entry && r["phase"] == "public_emission" && r["helper"] == false
            })
            .collect();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["outcome"]["status"], "supported");
        assert_eq!(rows[0]["body_owner"], "algebra.ordinary");
        let label = format!("source-{entry}");
        let metadata = json!({
            "label":label,"root":entry,"internal":root.internal,
            "source_variant":"unchanged retained full source IR",
            "whole_old_required":false,"whole_old_unit_composition":old_status,
            "checked":{"status":"supported"},
            "full_native_render":true,"public_emission":rows[0],"capabilities":rendered.capabilities,
            "claim":"Whole source checked composition and native generation; Rust compilation/runtime/proof qualified separately."
        });
        eprintln!(
            "WIDE_FIELD_CAST_PROBE {}",
            serde_json::to_string(&metadata).unwrap()
        );
        if let Some(base) = std::env::var_os("COMPACT_WIDE_FIELD_CAST_OUTPUT") {
            let directory = std::path::PathBuf::from(base).join(label);
            std::fs::create_dir_all(directory.parent().unwrap()).unwrap();
            std::fs::create_dir(&directory).expect("fresh case output required");
            std::fs::write(directory.join("compact-rust-ir.json"), source).unwrap();
            std::fs::write(directory.join("lib.rs"), rendered.source).unwrap();
            std::fs::write(
                directory.join("case.json"),
                serde_json::to_vec_pretty(&metadata).unwrap(),
            )
            .unwrap();
        }
    }
}
