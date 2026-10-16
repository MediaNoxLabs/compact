// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;

#[test]
fn complete_original_did_roots_measure_direct_coordinate_composition() {
    let source =
        include_str!("../../../../../tests/relation-composition/original-did-schema20-ir.json");
    for entry in [
        "rotateControllerKey",
        "recoverControllerKey",
        "setAlsoKnownAs",
        "setVerificationMethod",
        "removeVerificationMethod",
        "setSchnorrJubjubVerificationMethod",
        "removeSchnorrJubjubVerificationMethod",
        "setVerificationMethodRelation",
        "setService",
        "removeService",
        "deactivate",
    ] {
        probe(source.into(), entry, format!("did-{entry}"), true, false);
    }
}
#[test]
fn complete_crypto_roots_measure_direct_coordinate_composition() {
    let source = include_str!("../../../../../tests/unit-composition/crypto.json");
    for entry in ["close", "verifyOnly"] {
        probe(source.into(), entry, format!("crypto-{entry}"), true, false);
    }
}
#[test]
fn complete_point_guard_measures_direct_coordinate_composition() {
    probe(
        include_str!("../../../../../tests/point-composition/point_guard.json").into(),
        "check",
        "point-guard-check".into(),
        true,
        false,
    );
}
#[test]
fn genuine_supplied_and_lazy_witness_points_compose_direct_x_and_y() {
    let source =
        include_str!("../../../../../tests/effect-plan-jubjub-projections-schema20-ir.json");
    for entry in ["project", "observe"] {
        probe(
            source.into(),
            entry,
            format!("source-{entry}"),
            false,
            false,
        );
    }
}
#[test]
fn typed_y_first_variants_compose_separate_projection_operation() {
    let mut source: Json = serde_json::from_str(include_str!(
        "../../../../../tests/effect-plan-jubjub-projections-schema20-ir.json"
    ))
    .unwrap();
    assert_eq!(y_projections(&mut source), 4);
    for entry in ["project", "observe"] {
        probe(
            serde_json::to_string_pretty(&source).unwrap(),
            entry,
            format!("source-y-{entry}"),
            false,
            true,
        );
    }
}
