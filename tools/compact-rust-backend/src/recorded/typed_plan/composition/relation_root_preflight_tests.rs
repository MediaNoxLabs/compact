// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use super::*;
use crate::ir::Contract;
use serde_json::Value;

fn contract(path: &str) -> Contract {
    serde_json::from_str(
        &std::fs::read_to_string(format!(
            "{}/tests/relation-composition/{path}",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap(),
    )
    .unwrap()
}
fn check(
    c: &Contract,
    root: &str,
    names: [&str; 3],
    compatible: Option<&str>,
) -> Option<Vec<Vec<Event>>> {
    let fields = c.ledger_fields.iter().map(|f| (f.id.as_str(), f)).collect();
    let get = |name: &str| c.stateful_circuits.iter().find(|c| c.name == name);
    let family =
        SelectedSetFamily::checked(get(names[0])?, get(names[1])?, get(names[2])?, &fields)?;
    let map = if let Some(name) = compatible {
        Some(NestedMapRead::checked(get(name)?, &fields)?)
    } else {
        None
    };
    let root = get(root)?;
    let mut owner = RootOwner {
        selected: &family,
        compatible: map.as_ref(),
        origin: None,
    };
    owner.checked(root)
}
#[test]
fn original_and_renamed_root_preserve_scoped_read_write_paths() {
    let did = contract("original-did-schema20-ir.json");
    let paths = check(
        &did,
        "setVerificationMethodRelation",
        [
            "verificationMethodRelationMember",
            "insertVerificationMethodRelation",
            "removeVerificationMethodRelationFromLedger",
        ],
        Some("assertVerificationMethodRelationCompatible"),
    )
    .unwrap();
    assert!(paths.contains(&vec![Event::Read, Event::Compatible, Event::Insert]));
    assert!(paths.contains(&vec![Event::Read, Event::Remove]));
    let small = contract("two-set-schema20-ir.json");
    let paths = check(&small, "update", ["member", "insert", "remove"], None).unwrap();
    assert!(paths.contains(&vec![Event::Read, Event::Insert]));
    assert!(paths.contains(&vec![Event::Read, Event::Remove]));
    let four = contract("four-set-schema20-ir.json");
    let paths = check(&four, "update", ["member", "insert", "remove"], None).unwrap();
    assert!(paths.contains(&vec![Event::Read, Event::Insert]));
    assert!(paths.contains(&vec![Event::Read, Event::Remove]));
}

fn root_mut(value: &mut Value) -> &mut Value {
    value["stateful_circuits"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|c| c["name"] == "setVerificationMethodRelation")
        .unwrap()
}
fn call_mut<'a>(value: &'a mut Value, name: &str) -> Option<&'a mut Value> {
    if value["kind"] == "circuit_call" && value["name"] == name {
        return Some(value);
    }
    match value {
        Value::Object(map) => {
            for child in map.values_mut() {
                if let Some(found) = call_mut(child, name) {
                    return Some(found);
                }
            }
            None
        }
        Value::Array(values) => values.iter_mut().find_map(|v| call_mut(v, name)),
        _ => None,
    }
}
fn call(value: &Value, name: &str) -> Option<Value> {
    if value["kind"] == "circuit_call" && value["name"] == name {
        return Some(value.clone());
    }
    match value {
        Value::Object(fields) => fields.values().find_map(|child| call(child, name)),
        Value::Array(values) => values.iter().find_map(|child| call(child, name)),
        _ => None,
    }
}
fn swap_calls(value: &mut Value, a: &str, b: &str) -> bool {
    match value {
        Value::Array(values) => {
            if let Some(index) = values
                .windows(2)
                .position(|window| window[0]["name"] == a && window[1]["name"] == b)
            {
                values.swap(index, index + 1);
                return true;
            }
            values.iter_mut().any(|v| swap_calls(v, a, b))
        }
        Value::Object(map) => map.values_mut().any(|v| swap_calls(v, a, b)),
        _ => false,
    }
}
fn original_checked(c: &Contract) -> Option<Vec<Vec<Event>>> {
    check(
        c,
        "setVerificationMethodRelation",
        [
            "verificationMethodRelationMember",
            "insertVerificationMethodRelation",
            "removeVerificationMethodRelationFromLedger",
        ],
        Some("assertVerificationMethodRelationCompatible"),
    )
}
#[test]
fn rebind_same_typed_key_or_move_insert_before_compatibility_refuses() {
    let original = contract("original-did-schema20-ir.json");
    let mut value = serde_json::to_value(&original).unwrap();
    let root = root_mut(&mut value);
    let mut alternate = root["parameters"][1].clone();
    alternate["name"] = "alternateId".into();
    root["parameters"].as_array_mut().unwrap().push(alternate);
    call_mut(&mut root["actions"], "insertVerificationMethodRelation").unwrap()["arguments"][1]["value"]
        ["name"] = "alternateId".into();
    let altered: Contract = serde_json::from_value(value).unwrap();
    assert!(original_checked(&altered).is_none());

    let mut value = serde_json::to_value(original).unwrap();
    assert!(swap_calls(
        &mut root_mut(&mut value)["actions"],
        "assertVerificationMethodRelationCompatible",
        "insertVerificationMethodRelation",
    ));
    let altered: Contract = serde_json::from_value(value).unwrap();
    assert!(original_checked(&altered).is_none());
}

#[test]
fn unrelated_declarations_do_not_enter_candidate_cartesian_search() {
    let original = contract("original-did-schema20-ir.json");
    let original_root = original
        .stateful_circuits
        .iter()
        .find(|c| c.name == "setVerificationMethodRelation")
        .unwrap();
    let before = called_helpers(&original_root.actions, &original_root.return_value);
    let mut value = serde_json::to_value(&original).unwrap();
    let template = value["stateful_circuits"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"] == "insertVerificationMethodRelation")
        .unwrap()
        .clone();
    for index in 0..100 {
        let mut distractor = template.clone();
        distractor["name"] = format!("unrelatedSetWriter{index}").into();
        value["stateful_circuits"]
            .as_array_mut()
            .unwrap()
            .push(distractor);
    }
    let expanded: Contract = serde_json::from_value(value).unwrap();
    let expanded_root = expanded
        .stateful_circuits
        .iter()
        .find(|c| c.name == "setVerificationMethodRelation")
        .unwrap();
    let after = called_helpers(&expanded_root.actions, &expanded_root.return_value);
    assert_eq!(before.expression, after.expression);
    assert_eq!(before.action, after.action);
    assert!(
        RelationPermit::discover(
            expanded_root,
            &expanded
                .ledger_fields
                .iter()
                .map(|f| (f.id.as_str(), f))
                .collect(),
            &expanded
                .stateful_circuits
                .iter()
                .map(|c| (c.name.as_str(), c))
                .collect(),
        )
        .is_some()
    );
}

#[test]
fn referenced_matching_distractors_are_classified_once_and_refuse_ambiguity() {
    let original = contract("original-did-schema20-ir.json");
    let mut value = serde_json::to_value(&original).unwrap();
    let template = value["stateful_circuits"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"] == "insertVerificationMethodRelation")
        .unwrap()
        .clone();
    let original_call = call(
        &root_mut(&mut value)["actions"],
        "insertVerificationMethodRelation",
    )
    .unwrap();
    for index in 0..64 {
        let name = format!("alternateWriter{index}");
        let mut declaration = template.clone();
        declaration["name"] = name.clone().into();
        value["stateful_circuits"]
            .as_array_mut()
            .unwrap()
            .push(declaration);
        let mut selected = original_call.clone();
        selected["name"] = name.into();
        root_mut(&mut value)["actions"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({
                "kind":"if", "condition":{"kind":"boolean","value":false},
                "then":selected,
                "otherwise":{"kind":"sequence","actions":[]}
            }));
    }
    let expanded: Contract = serde_json::from_value(value).unwrap();
    let root = expanded
        .stateful_circuits
        .iter()
        .find(|c| c.name == "setVerificationMethodRelation")
        .unwrap();
    let fields = expanded
        .ledger_fields
        .iter()
        .map(|f| (f.id.as_str(), f))
        .collect();
    let circuits = expanded
        .stateful_circuits
        .iter()
        .map(|c| (c.name.as_str(), c))
        .collect();
    let calls = called_helpers(&root.actions, &root.return_value);
    let roles = ClassifiedRoles::from_called(&calls, &fields, &circuits);
    let expression_declarations = calls
        .expression
        .iter()
        .filter(|name| circuits.contains_key(name.as_str()))
        .count();
    let action_declarations = calls
        .action
        .iter()
        .filter(|name| circuits.contains_key(name.as_str()))
        .count();
    assert_eq!(
        roles.inspected,
        expression_declarations + 3 * action_declarations
    );
    assert_eq!(roles.inserts.len(), 65);
    assert!(RelationPermit::discover(root, &fields, &circuits).is_none());
}

#[test]
fn sequential_empty_branches_deduplicate_and_second_selected_write_refuses() {
    let original = contract("original-did-schema20-ir.json");
    let mut value = serde_json::to_value(&original).unwrap();
    let root = root_mut(&mut value);
    let harmless = serde_json::json!({
        "kind":"if", "condition":{"kind":"boolean","value":true},
        "then":{"kind":"assert","condition":{"kind":"boolean","value":true},"message":"ok"},
        "otherwise":{"kind":"assert","condition":{"kind":"boolean","value":true},"message":"ok"}
    });
    root["actions"]
        .as_array_mut()
        .unwrap()
        .extend(std::iter::repeat_n(harmless, 24));
    let widened: Contract = serde_json::from_value(value.clone()).unwrap();
    let paths = original_checked(&widened).unwrap();
    assert!(
        paths.len() <= 5,
        "finite event state set, not 2^24 histories"
    );

    let root = root_mut(&mut value);
    let insert = call(&root["actions"], "insertVerificationMethodRelation").unwrap();
    let remove = call(
        &root["actions"],
        "removeVerificationMethodRelationFromLedger",
    )
    .unwrap();
    for _ in 0..16 {
        root["actions"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({
                "kind":"if", "condition":{"kind":"boolean","value":true},
                "then":insert, "otherwise":remove
            }));
    }
    let repeated: Contract = serde_json::from_value(value).unwrap();
    assert!(original_checked(&repeated).is_none());
}

#[test]
fn transitive_selected_write_through_ordinary_unit_helper_refuses() {
    let original = contract("original-did-schema20-ir.json");
    let mut value = serde_json::to_value(&original).unwrap();
    let insert = call(
        &root_mut(&mut value)["actions"],
        "insertVerificationMethodRelation",
    )
    .unwrap();
    let mut wrapper = value["stateful_circuits"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"] == "insertVerificationMethodRelation")
        .unwrap()
        .clone();
    wrapper["name"] = "apparentlyUnrelatedUnit".into();
    let arguments = wrapper["parameters"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| serde_json::json!({"kind":"parameter","name":p["name"]}))
        .collect::<Vec<_>>();
    wrapper["actions"] = serde_json::json!([{
        "kind":"circuit_call", "name":"insertVerificationMethodRelation", "arguments":arguments
    }]);
    value["stateful_circuits"]
        .as_array_mut()
        .unwrap()
        .push(wrapper);
    let mut disguised = insert;
    disguised["name"] = "apparentlyUnrelatedUnit".into();
    root_mut(&mut value)["actions"]
        .as_array_mut()
        .unwrap()
        .push(disguised);
    let altered: Contract = serde_json::from_value(value).unwrap();
    let root = altered
        .stateful_circuits
        .iter()
        .find(|c| c.name == "setVerificationMethodRelation")
        .unwrap();
    let fields = altered
        .ledger_fields
        .iter()
        .map(|f| (f.id.as_str(), f))
        .collect();
    let circuits = altered
        .stateful_circuits
        .iter()
        .map(|c| (c.name.as_str(), c))
        .collect();
    assert!(RelationPermit::discover(root, &fields, &circuits).is_none());
}
