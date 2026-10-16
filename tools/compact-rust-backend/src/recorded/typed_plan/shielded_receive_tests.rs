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

//! shielded receive tests for recording profile admission and lowering.

use super::*;
use serde_json::{Value, json};

fn source() -> Value {
    serde_json::from_str(include_str!(
        "../../../tests/shielded-receive-schema20-ir.json"
    ))
    .unwrap()
}

fn admitted(value: &Value) -> Option<TypedPlan> {
    let contract: crate::ir::Contract = serde_json::from_value(value.clone()).unwrap();
    let circuit = contract
        .stateful_circuits
        .iter()
        .find(|circuit| circuit.name == "accept")?;
    lower_shielded_receive(
        circuit,
        &contract
            .ledger_fields
            .iter()
            .map(|f| (f.id.as_str(), f))
            .collect(),
        &contract
            .witnesses
            .iter()
            .map(|w| (w.name.as_str(), w))
            .collect(),
        &contract
            .circuits
            .iter()
            .map(|p| (p.name.as_str(), p))
            .collect(),
        &contract
            .stateful_circuits
            .iter()
            .map(|c| (c.name.as_str(), c))
            .collect(),
    )
}

#[test]
fn receive_wrapper_records_exactly_one_output_and_claim_with_audited_pure_calls() {
    let source = source();
    let plan = admitted(&source).expect("unchanged receive helper must be admitted");
    let steps = plan.steps;
    let tokens = quote::quote!(#(#steps)*).to_string();
    assert_eq!(tokens.matches("kernel_self").count(), 1);
    assert_eq!(tokens.matches("create_zswap_output").count(), 1);
    assert_eq!(tokens.matches("CoinReceive").count(), 1);
    assert_eq!(tokens.matches("pure_circuits :: coinCommitment").count(), 1);
    assert!(tokens.find("kernel_self").unwrap() < tokens.find("create_zswap_output").unwrap());
    assert!(tokens.find("create_zswap_output").unwrap() < tokens.find("CoinReceive").unwrap());

    let mut renamed = source.clone();
    renamed["stateful_circuits"][1]["name"] = "another_entry".into();
    let contract: crate::ir::Contract = serde_json::from_value(renamed).unwrap();
    assert!(
        lower_shielded_receive(
            &contract.stateful_circuits[1],
            &HashMap::new(),
            &HashMap::new(),
            &contract
                .circuits
                .iter()
                .map(|p| (p.name.as_str(), p))
                .collect(),
            &contract
                .stateful_circuits
                .iter()
                .map(|c| (c.name.as_str(), c))
                .collect(),
        )
        .is_some()
    );
}

#[test]
fn receive_rejects_unused_queries_nested_arguments_and_impure_helpers() {
    let source = source();
    let mut unused_counter = source.clone();
    unused_counter["ledger_fields"] = json!([{
        "id":"low", "index":0, "path":[0], "declaration":{"kind":"counter"}
    }]);
    unused_counter["stateful_circuits"][0]["actions"][0]["bindings"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "name":"unused", "ty":{"kind":"boolean"},
            "value":{"kind":"counter_less_than", "field":"low", "index":0,
                "threshold":{"kind":"unsigned_literal", "value":"1",
                    "max":"18446744073709551615"}}
        }));
    assert!(admitted(&unused_counter).is_none());

    let mut nested_member = source.clone();
    nested_member["ledger_fields"] = json!([{
        "id":"keys", "index":0, "path":[0],
        "declaration":{"kind":"set", "ty":{"kind":"bytes", "length":32}}
    }]);
    let argument = nested_member["stateful_circuits"][1]["actions"][0]["arguments"][0].clone();
    nested_member["stateful_circuits"][1]["actions"][0]["arguments"][0] = json!({
        "kind":"let", "bindings":[{
            "name":"unused", "ty":{"kind":"boolean"},
            "value":{"kind":"set_member", "field":"keys", "index":0,
                "value":{"kind":"bytes_literal", "bytes":vec![0;32]}}
        }], "body":argument
    });
    assert!(admitted(&nested_member).is_none());

    let mut impure = source.clone();
    let helper = impure["circuits"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|c| c["name"] == "coinCommitment")
        .unwrap();
    let old_body = helper["body"].clone();
    helper["body"] = json!({"kind":"let", "bindings":[{
        "name":"hidden", "ty":{"kind":"unit"},
        "value":{"kind":"create_zswap_output", "coin":{"kind":"parameter", "name":"coin"},
            "recipient":{"kind":"parameter", "name":"recipient"}}
    }], "body":old_body});
    assert!(admitted(&impure).is_none());

    let mut wrong_pure_result = source.clone();
    let helper = wrong_pure_result["circuits"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|c| c["name"] == "coinCommitment")
        .unwrap();
    helper["result"] = json!({"kind":"boolean"});
    assert!(admitted(&wrong_pure_result).is_none());

    let mut duplicate_pure_formal = source.clone();
    let helper = duplicate_pure_formal["circuits"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|c| c["name"] == "coinCommitment")
        .unwrap();
    helper["parameters"][1]["name"] = "coin".into();
    assert!(admitted(&duplicate_pure_formal).is_none());

    let mut pure_cycle = source.clone();
    let helper = pure_cycle["circuits"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|c| c["name"] == "coinCommitment")
        .unwrap();
    let left = helper["parameters"][0]["name"].clone();
    let right = helper["parameters"][1]["name"].clone();
    helper["body"] = json!({"kind":"call", "name":"coinCommitment",
        "arguments":[{"kind":"parameter", "name":left},
            {"kind":"parameter", "name":right}]});
    assert!(admitted(&pure_cycle).is_none());

    let mut recursive = source.clone();
    recursive["stateful_circuits"][1]["actions"][0]["name"] = "accept".into();
    assert!(admitted(&recursive).is_none());

    let mut wrong_claim = source.clone();
    wrong_claim["stateful_circuits"][0]["actions"][0]["action"]["actions"][1]["action"]["value"]
        ["claim"] = "coin_spend".into();
    assert!(admitted(&wrong_claim).is_none());
}
