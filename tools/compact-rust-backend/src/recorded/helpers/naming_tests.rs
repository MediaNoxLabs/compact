// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Stable private names are planner laws, independent of emitted reachability.
use super::*;
use crate::ir::{Contract, LocalBinding, Parameter, SCHEMA_VERSION};
#[path = "../../../tests/support/recorded_helper_contract.rs"]
mod helper_contract;
use helper_contract::contract;

fn planned(contract: &Contract) -> Vec<String> {
    // Mandatory native validation and public recording run before inspecting
    // the same complete source's actual shared-helper planning.
    crate::render(contract).unwrap();
    let ledger = contract
        .ledger_fields
        .iter()
        .map(|f| (f.id.as_str(), f))
        .collect();
    let witnesses = contract
        .witnesses
        .iter()
        .map(|w| (w.name.as_str(), w))
        .collect();
    let pure = contract
        .circuits
        .iter()
        .map(|c| (c.name.as_str(), c))
        .collect();
    let circuits = contract
        .stateful_circuits
        .iter()
        .map(|c| (c.name.as_str(), c))
        .collect();
    let (names, items) = plan_recorded_helpers(
        &contract.stateful_circuits,
        &ledger,
        &witnesses,
        &pure,
        &circuits,
    )
    .unwrap();
    assert!(names.contains(&contract.stateful_circuits[0].name));
    items
        .into_iter()
        .filter_map(|item| {
            if let syn::Item::Fn(function) = item {
                Some(function.sig.ident.to_string())
            } else {
                None
            }
        })
        .collect()
}

#[test]
fn real_helper_planner_keeps_raw_and_dollar_name_allocation() {
    for (name, expected) in [
        ("r#type", "__compact_recorded_body_type"),
        ("a$b", "__compact_recorded_body_a_b"),
    ] {
        assert!(
            planned(&contract(name, true, false))
                .iter()
                .any(|actual| actual == expected)
        );
    }
}

#[test]
fn real_helper_planner_allocates_stable_collision_fallbacks() {
    for (name, expected) in [
        ("inner", "__compact_recorded_body_1_x696e6e6572"),
        ("r#type", "__compact_recorded_body_2_x722374797065"),
    ] {
        assert!(
            planned(&contract(name, true, true))
                .iter()
                .any(|actual| actual == expected)
        );
    }
    let mut raw_preferred = contract("r#type", true, true);
    raw_preferred.stateful_circuits[2].name = "r#__compact_recorded_body_type".into();
    assert!(
        planned(&raw_preferred)
            .iter()
            .any(|actual| actual == "__compact_recorded_body_2_x722374797065")
    );
    let mut raw_fallback = contract("inner", true, true);
    let mut sibling = raw_fallback.stateful_circuits[1].clone();
    sibling.name = "r#__compact_recorded_body_1_x696e6e6572".into();
    raw_fallback.stateful_circuits.push(sibling);
    assert!(
        planned(&raw_fallback)
            .iter()
            .any(|actual| actual == "__compact_recorded_body_1_x696e6e6572_")
    );
}
