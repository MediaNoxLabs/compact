// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Maintained full-source controls complement the separate authenticated corpus export.
use super::*;
use crate::recorded::{admission_probe::trace, profile_attempt::ProfileAttempt};
use serde_json::json;

fn inspect(source: &'static str, roots: &'static [&'static str], old_required: bool) {
    // Match the established complete-DID qualification envelope. Ordinary worker
    // limits are still checked independently by the fixed 2 MiB resource tests.
    std::thread::Builder::new()
        .stack_size(8 * 1024 * 1024)
        .spawn(move || {
            let c: Contract = serde_json::from_str(source).unwrap();
            let ledger = c.ledger_fields.iter().map(|f| (f.id.as_str(), f)).collect();
            let witnesses = c.witnesses.iter().map(|w| (w.name.as_str(), w)).collect();
            let pure = c.circuits.iter().map(|f| (f.name.as_str(), f)).collect();
            let stateful: HashMap<_, _> = c
                .stateful_circuits
                .iter()
                .map(|f| (f.name.as_str(), f))
                .collect();
            for name in roots {
                let root = stateful[name];
                let old = crate::recorded::typed_plan::lower_unit_composition_checked(
                    root, &ledger, &witnesses, &pure, &stateful,
                );
                if old_required {
                    assert!(
                        matches!(old, ProfileAttempt::Admitted(_)),
                        "{name}: maintained whole old premise"
                    );
                }
                let plan = check_with_calls(root, &ledger, &witnesses, &pure, &stateful)
                    .unwrap_or_else(|gap| {
                        panic!("{name}: complete Map source must compose: {gap:?}")
                    });
                render(&plan).unwrap();
            }
            let (rendered, invocations) =
                trace::capture_json(|| crate::render_with_capabilities(&c));
            let rendered = rendered.unwrap();
            for name in roots {
                let rows: Vec<_> = invocations
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|r| {
                        r["circuit"] == *name
                            && r["phase"] == "public_emission"
                            && r["helper"] == false
                    })
                    .collect();
                assert_eq!(rows.len(), 1);
                let (owner, status) = if old_required {
                    (
                        match *name {
                            "remove_service" | "drop" | "remove" => "algebra.membership",
                            _ => "algebra.unit-composition",
                        },
                        "supported",
                    )
                } else if *name == "direct_member" {
                    // The one-string Boolean interface now admits direct Map
                    // membership; the separate Field lookup interface does not.
                    ("algebra.membership", "supported")
                } else if *name == "direct_lookup" {
                    ("return.dispatch", "unsupported")
                } else {
                    ("algebra.composite-values", "supported")
                };
                assert_eq!(rows[0]["body_owner"], owner, "{name}: actual owner");
                assert_eq!(
                    rows[0]["outcome"]["status"], status,
                    "{name}: public dispatch"
                );
                let capability = rendered
                    .capabilities
                    .circuits
                    .iter()
                    .find(|c| c.name == *name)
                    .unwrap();
                assert_eq!(capability.recorded, status == "supported");
                assert_eq!(capability.observed_call, status == "supported");
                eprintln!(
                    "MAP_SOURCE_PROBE {}",
                    json!({"root":name,"checked":"supported","old_required":old_required,
                "actual":rows[0],"capabilities":rendered.capabilities,
                "source_variant":"maintained whole fixture; external corpus provenance may differ"})
                );
            }
        })
        .unwrap()
        .join()
        .unwrap();
}
#[test]
fn maintained_map_cohort_preserves_complete_old_admission_and_composes() {
    inspect(
        include_str!("../../../../../tests/relation-composition/original-did-schema20-ir.json"),
        &[
            "setVerificationMethod",
            "removeVerificationMethod",
            "setSchnorrJubjubVerificationMethod",
            "removeSchnorrJubjubVerificationMethod",
            "setVerificationMethodRelation",
            "setService",
            "removeService",
        ],
        true,
    );
    inspect(
        include_str!("../../../../../tests/relation-composition/nested-map-schema20-ir.json"),
        &["check"],
        true,
    );
    inspect(
        include_str!("../../../../../tests/map-composition/service_mutation.json"),
        &["set_service", "remove_service"],
        true,
    );
    inspect(
        include_str!("../../../../../tests/map-nested-product/nested-enum-wide.json"),
        &["put", "drop"],
        true,
    );
    inspect(
        include_str!("../../../../../tests/map-nested-product/nested-enum.json"),
        &["upsert", "remove"],
        true,
    );
    inspect(
        include_str!("../../../../../tests/map-point-composition/point_nested.json"),
        &["upsert", "remove"],
        true,
    );
}
#[test]
fn genuine_scalar_and_nested_map_sources_have_checked_operations() {
    inspect(
        include_str!("../../../../../tests/effect-plan-maps-schema20-ir.json"),
        &[
            "member",
            "lookup",
            "put",
            "remove",
            "lazy_put",
            "direct_member",
            "direct_lookup",
        ],
        false,
    );
    inspect(
        include_str!("../../../../../tests/effect-plan-nested-map-member-schema20-ir.json"),
        &["member"],
        false,
    );
}

#[test]
fn calibrated_nested_map_lookup_has_terminal_recording_owner() {
    let contract: Contract = serde_json::from_str(include_str!(
        "../../../../resource_limits/test_inputs/native_map-32.json"
    ))
    .unwrap();
    let (rendered, invocations) =
        trace::capture_json(|| crate::render_with_capabilities(&contract));
    let rendered = rendered.unwrap();
    let capability = &rendered.capabilities.circuits[0];
    assert_eq!(capability.name, "entry");
    assert!(capability.recorded && capability.observed_call);
    let rows: Vec<_> = invocations
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| {
            r["circuit"] == "entry" && r["phase"] == "public_emission" && r["helper"] == false
        })
        .collect();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["body_owner"], "algebra.terminal-returns");
    assert_eq!(rows[0]["outcome"]["status"], "supported");
}
