// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Projection composition: whole old roots, checked gaps and actual owners are distinct.
use super::*;
use crate::recorded::admission_probe::trace;
use crate::recorded::profile_attempt::ProfileAttempt;
use serde_json::{Value as Json, json};

#[path = "jubjub_projections/source.rs"]
mod source;
#[path = "jubjub_projections/validation.rs"]
mod validation;

fn probe(source: String, entry: &'static str, label: String, require_old: bool, y_first: bool) {
    std::thread::Builder::new()
        .stack_size(8 * 1024 * 1024)
        .spawn(move || inspect(&source, entry, &label, require_old, y_first))
        .unwrap()
        .join()
        .unwrap();
}

fn inspect(source: &str, entry: &str, label: &str, require_old: bool, y_first: bool) {
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
            "{label}: complete old admission BEFORE candidate"
        );
    }
    let retained_schnorr = label.starts_with("did-") || label.starts_with("crypto-");
    let checked = match check_with_calls(root, &ledger, &witnesses, &pure, &stateful) {
        Ok(plan) => {
            if retained_schnorr {
                super::retained_schnorr::assert_retained_root_label(label);
            }
            render(&plan).unwrap();
            json!({"status":"supported"})
        }
        Err(PlanError::Unsupported(gap)) => {
            panic!("{label}: complete source must compose: {gap:?}");
        }
        Err(PlanError::Invalid(gap)) => panic!("{label}: invariant failure {gap:?}"),
    };
    let (rendered, invocations) =
        trace::capture_json(|| crate::render_with_capabilities(&contract));
    let rendered = rendered.unwrap_or_else(|error| panic!("{label}: full native render {error:?}"));
    let rows = invocations
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| {
            row["circuit"] == entry && row["phase"] == "public_emission" && row["helper"] == false
        })
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), 1, "{label}: actual public emission");
    if require_old {
        assert_eq!(rows[0]["body_owner"], "algebra.unit-composition");
        assert_eq!(rows[0]["outcome"]["status"], "supported");
    } else {
        assert_eq!(rows[0]["body_owner"], "algebra.composite-values");
        assert_eq!(rows[0]["outcome"]["status"], "supported");
    }
    let metadata = json!({
        "label":label,"root":entry,"internal":root.internal,
        "source_variant":if y_first {"typed X-to-Y projection substitution in complete genuine IR"} else {"unchanged retained full source IR"},
        "whole_old_required":require_old,"whole_old_unit_composition":old_status,
        "checked":checked,
        "full_native_render":true,"public_emission":rows[0],"capabilities":rendered.capabilities,
        "claim":"Finite whole-root admission/native rendering, not Rust compilation, full family inclusion, runtime or proof."
    });
    eprintln!(
        "JUBJUB_PROJECTION_PROBE {}",
        serde_json::to_string(&metadata).unwrap()
    );
    if let Some(base) = std::env::var_os("COMPACT_JUBJUB_PROJECTIONS_OUTPUT") {
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

fn y_projections(value: &mut Json) -> usize {
    let mut changed = 0;
    match value {
        Json::Object(object) => {
            if object.get("kind").and_then(Json::as_str) == Some("jubjub_point_x") {
                object.insert("kind".into(), json!("jubjub_point_y"));
                changed += 1;
            }
            for child in object.values_mut() {
                changed += y_projections(child);
            }
        }
        Json::Array(values) => {
            for child in values {
                changed += y_projections(child);
            }
        }
        _ => {}
    }
    changed
}
