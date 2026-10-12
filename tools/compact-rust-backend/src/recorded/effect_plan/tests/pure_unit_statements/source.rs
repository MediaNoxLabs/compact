// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;

#[test]
fn retained_guard_roots_have_complete_old_admission_before_checked_composition() {
    let source = include_str!("../../../../../tests/set-composition/pure_unit_guard.json");
    for (entry, label) in [
        ("run", "retained-guard-run"),
        ("local_control", "retained-guard-local-control"),
        ("transitive_control", "retained-guard-transitive-control"),
    ] {
        probe(source, entry, label, true);
    }
}
#[test]
fn retained_set_root_has_complete_old_admission_before_checked_composition() {
    probe(
        include_str!("../../../../../tests/set-composition/set_string.json"),
        "mutate",
        "retained-set-mutate",
        true,
    );
}
#[test]
fn retained_service_root_composes_maps_after_complete_old_admission() {
    probe(
        include_str!("../../../../../tests/map-composition/service_mutation.json"),
        "set_service",
        "retained-service-set-service",
        true,
    );
}
#[test]
fn retained_nested_root_composes_maps_after_complete_old_admission() {
    probe(
        include_str!("../../../../../tests/map-nested-product/nested-enum.json"),
        "upsert",
        "retained-nested-upsert",
        true,
    );
}
#[test]
fn genuine_explicit_witness_locals_compose_the_pure_statement() {
    probe(
        include_str!("../../../../../tests/effect-plan-pure-unit-statements-schema20-ir.json"),
        "run",
        "source-run",
        false,
    );
}
#[test]
fn genuine_fallible_pure_arguments_compose_the_pure_statement() {
    probe(
        include_str!("../../../../../tests/effect-plan-pure-unit-statements-schema20-ir.json"),
        "argument_order",
        "source-argument-order",
        false,
    );
}
