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

fn fixture(name: &str) -> Contract {
    serde_json::from_str(
        &std::fs::read_to_string(format!(
            "{}/tests/relation-composition/{name}",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap(),
    )
    .unwrap()
}

fn admitted(contract: &Contract, name: &str) -> bool {
    let fields = contract
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
    let circuit = contract
        .stateful_circuits
        .iter()
        .find(|c| c.name == name)
        .unwrap();
    matches!(
        lower_checked(circuit, &fields, &witnesses, &pure, &circuits),
        ProfileAttempt::Admitted(_)
    )
}

fn circuit<'a>(value: &'a mut Value, name: &str) -> &'a mut Value {
    value["stateful_circuits"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|c| c["name"] == name)
        .unwrap()
}

fn mutate_kind(value: &mut Value, kind: &str, f: &mut impl FnMut(&mut Value)) -> bool {
    if value["kind"] == kind {
        f(value);
        return true;
    }
    match value {
        Value::Object(fields) => fields.values_mut().any(|child| mutate_kind(child, kind, f)),
        Value::Array(values) => values.iter_mut().any(|child| mutate_kind(child, kind, f)),
        _ => false,
    }
}

#[test]
fn report_existing_domain_for_pinned_relation_and_reducers() {
    for path in [
        "original-did-schema20-ir.json",
        "two-set-schema20-ir.json",
        "four-set-schema20-ir.json",
        "nested-map-schema20-ir.json",
    ] {
        let contract = fixture(path);
        let fields = contract
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
        let name = if path.contains("two-set") || path.contains("four-set") {
            "update"
        } else if path.contains("nested-map") {
            "check"
        } else {
            "setVerificationMethodRelation"
        };
        let circuit = contract
            .stateful_circuits
            .iter()
            .find(|c| c.name == name)
            .unwrap();
        assert!(
            relation_root_preflight::RelationPermit::discover(circuit, &fields, &circuits)
                .is_some(),
            "checked owner {name}"
        );
        assert!(
            matches!(
                lower_checked(circuit, &fields, &witnesses, &pure, &circuits),
                ProfileAttempt::Admitted(_)
            ),
            "recorded admission {name}"
        );
    }
}

#[test]
fn entire_original_nested_map_domain_rejects_wrong_declaration_projection_and_hidden_effect() {
    let original = fixture("original-did-schema20-ir.json");
    assert!(admitted(&original, "setVerificationMethodRelation"));
    let mut value = serde_json::to_value(&original).unwrap();
    let mut other = value["ledger_fields"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["id"] == "verificationMethods")
        .unwrap()
        .clone();
    other["id"] = "anotherIdenticalMethodMap".into();
    other["path"] = serde_json::json!([1, 99]);
    value["ledger_fields"].as_array_mut().unwrap().push(other);
    assert!(mutate_kind(
        &mut circuit(&mut value, "assertVerificationMethodRelationCompatible")["actions"],
        "map_lookup",
        &mut |v| v["field"] = "anotherIdenticalMethodMap".into()
    ));
    let altered: Contract = serde_json::from_value(value).unwrap();
    assert!(!admitted(&altered, "setVerificationMethodRelation"));

    let mut value = serde_json::to_value(&original).unwrap();
    assert!(mutate_kind(
        &mut circuit(&mut value, "assertVerificationMethodRelationCompatible")["actions"],
        "struct_field",
        &mut |v| {
            if v["field"] == "crv" {
                v["index"] = 0.into();
            }
        }
    ));
    let altered: Contract = serde_json::from_value(value).unwrap();
    assert!(!admitted(&altered, "setVerificationMethodRelation"));

    let mut value = serde_json::to_value(&original).unwrap();
    let actions = &mut circuit(&mut value, "assertVerificationMethodRelationCompatible")["actions"];
    let root = &mut actions[0]["otherwise"]["otherwise"]["actions"];
    assert!(root.is_array());
    root.as_array_mut().unwrap().push(serde_json::json!({
        "kind":"map_remove", "field":"verificationMethods", "index":1,
        "key":{"kind":"parameter","name":"methodId"}
    }));
    let altered: Contract = serde_json::from_value(value).unwrap();
    assert!(!admitted(&altered, "setVerificationMethodRelation"));
}
