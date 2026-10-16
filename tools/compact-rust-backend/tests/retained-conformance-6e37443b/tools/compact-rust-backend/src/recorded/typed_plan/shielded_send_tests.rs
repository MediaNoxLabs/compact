// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//  	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! shielded send tests for recording profile admission and lowering.

use super::*;
use serde_json::{Value, json};

fn source() -> Value {
    serde_json::from_str(include_str!(
        "../../../tests/shielded-send-schema20-ir.json"
    ))
    .unwrap()
}

fn admitted(source: &Value) -> Vec<bool> {
    let contract: crate::ir::Contract = serde_json::from_value(source.clone()).unwrap();
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
    contract
        .stateful_circuits
        .iter()
        .map(|c| lower_shielded_send(c, &ledger, &witnesses, &pure, &circuits).is_some())
        .collect()
}

#[test]
fn unchanged_send_helper_and_wrappers_have_typed_ordered_plans() {
    let source = source();
    assert_eq!(admitted(&source), vec![true; 5]);
    let contract: crate::ir::Contract = serde_json::from_value(source).unwrap();
    let ledger = HashMap::new();
    let witnesses = HashMap::new();
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
    let plan = lower_shielded_send(
        &contract.stateful_circuits[1],
        &ledger,
        &witnesses,
        &pure,
        &circuits,
    )
    .unwrap();
    let statements = &plan.steps;
    let steps = quote::quote!(#(#statements)*).to_string();
    assert!(steps.find("create_zswap_input").unwrap() < steps.find("subtract_unsigned").unwrap());
    assert!(steps.find("subtract_unsigned").unwrap() < steps.find("create_zswap_output").unwrap());
    assert!(steps.contains("transient_hash") && steps.contains("upgrade_from_transient"));
}

#[test]
fn checked_send_profile_distinguishes_unrelated_and_admitted_shapes() {
    let source = source();
    let contract: crate::ir::Contract = serde_json::from_value(source).unwrap();
    let ledger = HashMap::new();
    let witnesses = HashMap::new();
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
    let send = &contract.stateful_circuits[1];
    assert!(matches!(
        lower_shielded_send_checked(send, &ledger, &witnesses, &pure, &circuits),
        ProfileAttempt::Admitted(_)
    ));
    let mut unrelated = send.clone();
    unrelated.result = Type::Unit;
    unrelated.return_value = StateReturn::Unit;
    assert!(matches!(
        lower_shielded_send_checked(&unrelated, &ledger, &witnesses, &pure, &circuits),
        ProfileAttempt::NotApplicable
    ));
}

#[test]
fn hidden_queries_in_unused_bindings_branches_and_arguments_are_refused() {
    let source = source();
    let mut unused = source.clone();
    let original = unused["stateful_circuits"][1]["return_value"]["value"].take();
    unused["stateful_circuits"][1]["return_value"]["value"] = json!({
        "kind":"let", "bindings":[{"name":"hidden", "ty":{"kind":"boolean"},
            "value":{"kind":"set_member", "field":"coins", "index":0,
                "value":{"kind":"parameter", "name":"input"}}}], "body":original
    });
    unused["ledger_fields"] = json!([{"id":"coins", "index":0,
        "declaration":{"kind":"set", "ty": source["stateful_circuits"][0]["parameters"][0]["ty"]}}]);
    assert!(!admitted(&unused)[1]);

    let mut branch = source.clone();
    let original = branch["stateful_circuits"][1]["return_value"]["value"].take();
    branch["stateful_circuits"][1]["return_value"]["value"] = json!({
        "kind":"if", "condition":{"kind":"boolean", "value":true},
        "then":original.clone(),
        "otherwise":{"kind":"let", "bindings":[{"name":"hidden", "ty":{"kind":"unsigned", "max":"18446744073709551615"},
            "value":{"kind":"set_size", "field":"coins", "index":0}}], "body": original}
    });
    branch["ledger_fields"] = unused["ledger_fields"].clone();
    assert!(!admitted(&branch)[1]);

    let mut argument = source;
    let original = argument["stateful_circuits"][1]["return_value"]["value"]["arguments"][0].take();
    argument["stateful_circuits"][1]["return_value"]["value"]["arguments"][0] = json!({
        "kind":"let", "bindings":[{"name":"hidden", "ty":{"kind":"boolean"},
            "value":{"kind":"set_member", "field":"coins", "index":0,
                "value":{"kind":"parameter", "name":"input"}}}], "body":original
    });
    argument["ledger_fields"] = unused["ledger_fields"].clone();
    assert!(!admitted(&argument)[1]);
}

#[test]
fn recursive_impure_and_malformed_helpers_are_refused() {
    let source = source();
    let mut recursive = source.clone();
    recursive["stateful_circuits"][0]["return_value"]["value"] = json!({
        "kind":"call", "name":"sendShielded", "arguments":[
            {"kind":"parameter", "name":"input"},
            {"kind":"parameter", "name":"recipient"},
            {"kind":"parameter", "name":"value"}]
    });
    assert!(!admitted(&recursive)[1]);

    let mut impure = source.clone();
    let helper = impure["circuits"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|c| c["name"] == "coinCommitment")
        .unwrap();
    helper["body"] = json!({"kind":"cell_read", "field":"hidden", "index":0});
    assert!(!admitted(&impure)[1]);

    let mut malformed = source;
    malformed["stateful_circuits"][0]["result"] = json!({"kind":"boolean"});
    assert!(!admitted(&malformed)[1]);
}
