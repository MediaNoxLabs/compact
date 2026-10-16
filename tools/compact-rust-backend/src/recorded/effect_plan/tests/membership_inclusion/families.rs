// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;

#[test]
fn membership_enum_counter_and_scalar_helper_entries_are_whole_old_controls() {
    let election = source(include_str!(
        "../../../../../tests/election-schema13-ir.json"
    ));
    included(
        &election,
        "vote$commit",
        "membership-enum",
        "membership/enum",
    );
    included(
        &election,
        "vote$reveal",
        "membership-counter",
        "membership/counter",
    );
    let dao = source(include_str!(
        "../../../../../tests/micro-dao-source-schema20-ir.json"
    ));
    included(
        &dao,
        "vote_reveal",
        "membership-counter-helper",
        "membership/scalar_counter_hash",
    );
    let renamed = renamed(
        &dao,
        &[
            ("reveal_nullifier", "member_key"),
            ("commit_with_sk", "member_commitment"),
            ("round", "round_number"),
        ],
    );
    included(
        &renamed,
        "vote_reveal",
        "membership-helper-renamed",
        "membership/scalar_counter_hash",
    );
}
#[test]
fn optional_cell_and_historic_spend_families_are_whole_old_controls() {
    let c = bboard();
    included(&c, "post", "optional-post", "cell_lifecycle/input");
    included(
        &c,
        "take_down",
        "optional-take-down",
        "cell_lifecycle/output",
    );
    let c = source(include_str!(
        "../../../../../tests/zerocash-oracle-schema20-ir.json"
    ));
    included(&c, "spend", "historic-spend", "historic_spend");
}
#[test]
fn counter_comparison_boolean_and_unit_entries_are_whole_old_controls() {
    let c = counter();
    for name in ["compare", "nested", "short_circuit", "checked"] {
        included(
            &c,
            name,
            &format!("comparison-{name}"),
            "counter_comparison",
        );
    }
}
#[test]
fn qualified_set_and_cell_families_are_whole_old_controls() {
    let set = source(include_str!(
        "../../../../../tests/qualified-coin-set-schema20-ir.json"
    ));
    included(
        &set,
        "insert_coin",
        "qualified-set-parameters",
        "qualified_set_lifecycle/parameters",
    );
    let set = source(include_str!(
        "../../../../../tests/adt-set-qualified-coin-info-schema20-ir.json"
    ));
    included(
        &set,
        "test_QualifiedShieldedCoinInfo",
        "qualified-set-no-parameters",
        "qualified_set_lifecycle/observed",
    );
    included(
        &set,
        "test_ShieldedCoinInfo",
        "qualified-set-coin-conversion",
        "qualified_set_lifecycle/observed",
    );
    let cell = source(include_str!(
        "../../../../../tests/qualified-coin-cell-schema20-ir.json"
    ));
    included(
        &cell,
        "write_coin",
        "qualified-cell-replacement",
        "qualified_cell_replacement",
    );
}

pub(super) fn field_cell() -> Contract {
    serde_json::from_value(json!({
        "schema_version": crate::ir::SCHEMA_VERSION,
        "ledger_fields":[{"id":"total","index":0,"path":[0],"declaration":{"kind":"cell","ty":{"kind":"field"}}}],
        "circuits":[],"witnesses":[],
        "stateful_circuits":[{"name":"update","parameters":[{"name":"amount","ty":{"kind":"field"}}],"result":{"kind":"field"},
            "actions":[{"kind":"let","bindings":[{"name":"before","ty":{"kind":"field"},"value":{"kind":"cell_read","field":"total","index":0}}],"action":{"kind":"cell_write","field":"total","index":0,"value":{"kind":"add","left":{"kind":"parameter","name":"before"},"right":{"kind":"parameter","name":"amount"}}}}],
            "return_value":{"kind":"expression","value":{"kind":"parameter","name":"amount"}}}]
    })).unwrap()
}
#[test]
fn field_cell_and_both_assertion_results_are_whole_old_controls() {
    included(
        &field_cell(),
        "update",
        "field-cell-root",
        "field_cell_root",
    );
    let assertions = assertions();
    included(
        &assertions,
        "checked",
        "assertion-u64",
        "assertion_entry/Uint64",
    );
    included(
        &assertions,
        "unit_result",
        "assertion-unit",
        "assertion_entry/Unit",
    );
}
