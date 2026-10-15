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

//! Ordered asset lookup, guard, write, and removal profiles.

use compact_rust_backend::ir::{Contract, Expr, SCHEMA_VERSION, StateAction};
use compact_rust_backend::render_with_capabilities;

#[test]
fn asset_removal_recording_preserves_scoped_keys_and_source_mutation_order() {
    let mut contract: Contract =
        serde_json::from_str(include_str!("../asset-removal-schema13-ir.json")).unwrap();
    contract.schema_version = SCHEMA_VERSION;
    let recorded = |contract: &Contract| {
        let rendered = render_with_capabilities(contract).unwrap();
        let capability = rendered
            .capabilities
            .circuits
            .iter()
            .find(|capability| capability.name == "removeRecord")
            .unwrap();
        (capability.recorded, rendered.source)
    };
    let (available, source) = recorded(&contract);
    assert!(available);
    assert!(source.contains("pub fn removeRecord<Private"));
    assert!(source.contains("record_remove(frame"));

    let mut swapped = contract.clone();
    let circuit = swapped
        .stateful_circuits
        .iter_mut()
        .find(|circuit| circuit.name == "removeRecord")
        .unwrap();
    let StateAction::Let { action, .. } = &mut circuit.actions[0] else {
        unreachable!()
    };
    let StateAction::Sequence { actions } = action.as_mut() else {
        unreachable!()
    };
    actions.swap(3, 4);
    let (available, swapped_source) = recorded(&swapped);
    assert!(available);
    let original = super::lifecycle_recording::generated_body(&source, "recorded", "removeRecord");
    let swapped =
        super::lifecycle_recording::generated_body(&swapped_source, "recorded", "removeRecord");
    assert!(original.find("record_remove").unwrap() < original.find("record_insert").unwrap());
    assert!(swapped.find("record_insert").unwrap() < swapped.find("record_remove").unwrap());

    let circuit = contract
        .stateful_circuits
        .iter_mut()
        .find(|circuit| circuit.name == "removeRecord")
        .unwrap();
    let StateAction::Let { action, .. } = &mut circuit.actions[0] else {
        unreachable!()
    };
    let StateAction::Sequence { actions } = action.as_mut() else {
        unreachable!()
    };
    let StateAction::MapRemove { key, .. } = &mut actions[3] else {
        unreachable!()
    };
    *key = Expr::Parameter {
        name: "recordId".into(),
    };
    assert!(
        recorded(&contract).0,
        "the in-scope typed parameter is also a valid key"
    );
    let StateAction::Let { action, .. } = &mut contract
        .stateful_circuits
        .iter_mut()
        .find(|c| c.name == "removeRecord")
        .unwrap()
        .actions[0]
    else {
        unreachable!()
    };
    let StateAction::Sequence { actions } = action.as_mut() else {
        unreachable!()
    };
    let StateAction::MapRemove { key, .. } = &mut actions[3] else {
        unreachable!()
    };
    *key = Expr::Parameter {
        name: "missing_key".into(),
    };
    assert!(
        render_with_capabilities(&contract).is_err(),
        "out-of-scope keys remain invalid"
    );
}

#[test]
fn custody_grant_recording_composes_typed_lookup_and_pure_assertions() {
    let mut contract: Contract =
        serde_json::from_str(include_str!("../asset-grant-effective-schema12-ir.json")).unwrap();
    contract.schema_version = SCHEMA_VERSION;
    let recorded = |contract: &Contract| {
        let rendered = render_with_capabilities(contract).unwrap();
        let capability = rendered
            .capabilities
            .circuits
            .iter()
            .find(|capability| capability.name == "assertGrantEffective")
            .unwrap();
        if capability.recorded {
            let body = super::lifecycle_recording::generated_body(
                &rendered.source,
                "recorded",
                "assertGrantEffective",
            );
            assert!(body.contains("record_lookup"));
        }
        (capability.recorded, rendered.source)
    };
    let (available, source) = recorded(&contract);
    assert!(available);
    assert!(source.contains("pub fn assertGrantEffective<Private"));
    assert!(source.contains("record_lookup(frame"));

    let mut wrong_key = contract.clone();
    let StateAction::Let { action, .. } = &mut wrong_key.stateful_circuits[0].actions[0] else {
        unreachable!()
    };
    let StateAction::Sequence { actions } = action.as_mut() else {
        unreachable!()
    };
    let StateAction::Let { bindings, .. } = &mut actions[1] else {
        unreachable!()
    };
    let Expr::MapLookup { key, .. } = &mut bindings[0].value else {
        unreachable!()
    };
    **key = Expr::Parameter {
        name: "grantId".into(),
    };
    assert!(
        recorded(&wrong_key).0,
        "an in-scope typed public key is valid"
    );

    let mut changed_guard = contract;
    let Expr::Sequence { steps, .. } = &mut changed_guard.circuits[0].body else {
        unreachable!()
    };
    steps.insert(
        0,
        Expr::Assert {
            condition: Box::new(Expr::NotEqual {
                left: Box::new(Expr::FieldLiteral { value: "1".into() }),
                right: Box::new(Expr::FieldLiteral { value: "0".into() }),
            }),
            message: "extra guard".into(),
        },
    );
    assert!(
        recorded(&changed_guard).0,
        "additional certified assertions compose"
    );
}

#[test]
fn guarded_struct_map_read_composes_typed_lookup_and_freshness_guards() {
    let mut contract: Contract = serde_json::from_str(include_str!(
        "../asset-stored-record-fresh-schema12-ir.json"
    ))
    .unwrap();
    contract.schema_version = SCHEMA_VERSION;
    let recorded = |contract: &Contract| {
        let rendered = render_with_capabilities(contract).unwrap();
        let capability = rendered
            .capabilities
            .circuits
            .iter()
            .find(|capability| capability.name == "assertStoredRecordFresh")
            .unwrap();
        if capability.recorded {
            let body = super::lifecycle_recording::generated_body(
                &rendered.source,
                "recorded",
                "assertStoredRecordFresh",
            );
            assert!(body.contains("record_lookup"));
        }
        (capability.recorded, rendered.source)
    };
    let (available, source) = recorded(&contract);
    assert!(available);
    assert!(source.contains("pub fn assertStoredRecordFresh<Private"));
    assert!(source.contains("record_lookup(frame"));

    let mut wrong_key = contract.clone();
    let StateAction::Let { action, .. } = &mut wrong_key.stateful_circuits[0].actions[0] else {
        unreachable!()
    };
    let StateAction::Sequence { actions } = action.as_mut() else {
        unreachable!()
    };
    let StateAction::Let { bindings, .. } = &mut actions[1] else {
        unreachable!()
    };
    let Expr::MapLookup { key, .. } = &mut bindings[0].value else {
        unreachable!()
    };
    **key = Expr::Parameter {
        name: "recordId".into(),
    };
    assert!(
        recorded(&wrong_key).0,
        "an in-scope typed public key is valid"
    );

    let mut extra_pure_work = contract;
    let Expr::Sequence { steps, .. } = &mut extra_pure_work.circuits[0].body else {
        unreachable!()
    };
    steps.insert(
        0,
        Expr::Assert {
            condition: Box::new(Expr::NotEqual {
                left: Box::new(Expr::FieldLiteral { value: "1".into() }),
                right: Box::new(Expr::FieldLiteral { value: "0".into() }),
            }),
            message: "extra guard".into(),
        },
    );
    assert!(
        recorded(&extra_pure_work).0,
        "additional certified assertions compose"
    );
}

#[test]
fn typed_opaque_struct_map_write_composes_scoped_values_and_source_order() {
    let mut contract: Contract = serde_json::from_str(include_str!(
        "../asset-custody-grant-write-schema12-ir.json"
    ))
    .unwrap();
    contract.schema_version = SCHEMA_VERSION;
    let recorded = |contract: &Contract| {
        let rendered = render_with_capabilities(contract).unwrap();
        let capability = rendered
            .capabilities
            .circuits
            .iter()
            .find(|capability| capability.name == "setCustodyGrant")
            .unwrap();
        if capability.recorded {
            let body = super::lifecycle_recording::generated_body(
                &rendered.source,
                "recorded",
                "setCustodyGrant",
            );
            assert!(body.contains("record_insert"));
            assert!(body.contains("record_remove"));
            assert!(body.contains("record_write"));
        }
        (capability.recorded, rendered.source)
    };
    let (available, source) = recorded(&contract);
    assert!(available);
    assert!(source.contains("pub fn setCustodyGrant<Private"));
    assert!(source.contains("crate::ledger_slots::custodyGrants\n                .record_insert("));

    fn actions(contract: &mut Contract) -> &mut Vec<StateAction> {
        let circuit = contract
            .stateful_circuits
            .iter_mut()
            .find(|circuit| circuit.name == "setCustodyGrant")
            .unwrap();
        let [StateAction::Let { action, .. }] = circuit.actions.as_mut_slice() else {
            unreachable!()
        };
        let StateAction::Let { action, .. } = action.as_mut() else {
            unreachable!()
        };
        let StateAction::Let { action, .. } = action.as_mut() else {
            unreachable!()
        };
        let StateAction::Sequence { actions } = action.as_mut() else {
            unreachable!()
        };
        actions
    }
    let mut unscoped_value = contract.clone();
    let StateAction::MapInsert { value, .. } = &mut actions(&mut unscoped_value)[3] else {
        unreachable!()
    };
    *value = Expr::Parameter {
        name: "grant".into(),
    };
    assert!(
        recorded(&unscoped_value).0,
        "the typed public struct is in scope"
    );

    let mut wrong_key = contract.clone();
    let StateAction::If { then, .. } = &mut actions(&mut wrong_key)[2] else {
        unreachable!()
    };
    let StateAction::Sequence { actions: update } = then.as_mut() else {
        unreachable!()
    };
    let StateAction::MapRemove { key, .. } = &mut update[1] else {
        unreachable!()
    };
    *key = Expr::Parameter {
        name: "grantId".into(),
    };
    assert!(
        recorded(&wrong_key).0,
        "the public key is typed and in scope"
    );

    let mut reordered = contract;
    actions(&mut reordered).swap(3, 4);
    assert!(
        recorded(&reordered).0,
        "typed mutation order follows the source"
    );
}

#[test]
fn typed_asset_map_write_composes_class_guards_and_typed_counts() {
    let original: serde_json::Value =
        serde_json::from_str(include_str!("../asset-record-write-schema13-ir.json")).unwrap();
    let recorded = |value: &serde_json::Value| {
        let mut contract: Contract = serde_json::from_value(value.clone()).unwrap();
        contract.schema_version = SCHEMA_VERSION;
        let Ok(rendered) = render_with_capabilities(&contract) else {
            return (false, String::new());
        };
        let capability = rendered
            .capabilities
            .circuits
            .iter()
            .find(|capability| capability.name == "setRecord")
            .unwrap();
        if capability.recorded {
            let body = super::lifecycle_recording::generated_body(
                &rendered.source,
                "recorded",
                "setRecord",
            );
            assert!(body.contains("record_insert"));
            assert!(body.contains("record_increment"));
        }
        (capability.recorded, rendered.source)
    };
    let (available, source) = recorded(&original);
    assert!(available);
    assert!(source.contains("pub fn setRecord<Private"));
    assert!(source.contains("record_increment(frame"));
    assert!(source.contains("assertRecordClassKnown("));
    assert!(source.contains("record_insert("));

    // The checked guard follows the declared enum/member, not the oracle's
    // spellings for either one. Mutation labels remain a bounded domain.
    fn rename_class(value: &mut serde_json::Value) {
        if let Some(object) = value.as_object_mut() {
            let kind = object
                .get("kind")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned);
            let name = object
                .get("name")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned);
            if kind.as_deref() == Some("enum") && name.as_deref() == Some("AssetClass") {
                object.get_mut("variants").unwrap()[0] = serde_json::json!("Pending");
            }
            if kind.as_deref() == Some("enum_variant")
                && object.get("ty").and_then(|ty| ty.get("name"))
                    == Some(&serde_json::json!("AssetClass"))
            {
                object.insert("variant".into(), serde_json::json!("Pending"));
            }
            if kind.as_deref() == Some("struct") && name.as_deref() == Some("AssetRecord") {
                for field in object.get_mut("fields").unwrap().as_array_mut().unwrap() {
                    if field["name"] == "kind" {
                        field["name"] = serde_json::json!("category");
                    }
                }
            }
            if kind.as_deref() == Some("struct_field")
                && object.get("field") == Some(&serde_json::json!("kind"))
            {
                object.insert("field".into(), serde_json::json!("category"));
            }
            for child in object.values_mut() {
                rename_class(child);
            }
        } else if let Some(items) = value.as_array_mut() {
            for child in items {
                rename_class(child);
            }
        }
    }
    let mut renamed = original.clone();
    rename_class(&mut renamed);
    assert!(recorded(&renamed).0);

    fn actions(contract: &mut serde_json::Value) -> &mut Vec<serde_json::Value> {
        contract["stateful_circuits"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|circuit| circuit["name"] == "setRecord")
            .unwrap()["actions"][0]["action"]["action"]["action"]["actions"]
            .as_array_mut()
            .unwrap()
    }
    let mut wrong_class_argument = original.clone();
    actions(&mut wrong_class_argument)[2]["arguments"][0]["value"]["name"] =
        serde_json::json!("record");
    assert!(recorded(&wrong_class_argument).0);

    let mut reordered = original.clone();
    actions(&mut reordered).swap(2, 3);
    assert!(recorded(&reordered).0);

    let mut extra_class_step = original.clone();
    extra_class_step["circuits"][0]["body"]["steps"]
        .as_array_mut()
        .unwrap()
        .push(original["circuits"][0]["body"]["steps"][0].clone());
    assert!(recorded(&extra_class_step).0);

    let mut wrong_counter_amount = original.clone();
    actions(&mut wrong_counter_amount)[3]["otherwise"]["then"]["actions"][1]["bindings"][0]["value"]
        ["value"] = serde_json::json!("2");
    assert!(recorded(&wrong_counter_amount).0);

    let mut wrong_counter_slot = original.clone();
    actions(&mut wrong_counter_slot)[3]["otherwise"]["then"]["actions"][1]["action"]["field"] =
        serde_json::json!("open");
    assert!(!recorded(&wrong_counter_slot).0);

    let mut wrong_exists_map = original.clone();
    let helper = wrong_exists_map["stateful_circuits"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|circuit| circuit["name"] == "recordExists")
        .unwrap();
    helper["return_value"]["value"]["otherwise"]["field"] = serde_json::json!("records");
    assert!(recorded(&wrong_exists_map).0);
}

#[test]
fn guarded_opaque_set_mutation_composes_typed_maps_and_branches() {
    let original: serde_json::Value =
        serde_json::from_str(include_str!("../asset-watch-write-schema13-ir.json")).unwrap();
    let recorded = |value: &serde_json::Value| {
        let mut contract: Contract = serde_json::from_value(value.clone()).unwrap();
        contract.schema_version = SCHEMA_VERSION;
        let Ok(rendered) = render_with_capabilities(&contract) else {
            return (false, String::new());
        };
        let capability = rendered
            .capabilities
            .circuits
            .iter()
            .find(|capability| capability.name == "setWatch")
            .unwrap();
        if capability.recorded {
            let body = super::lifecycle_recording::generated_body(
                &rendered.source,
                "recorded",
                "setWatch",
            );
            assert!(body.contains("record_insert"));
            assert!(body.contains("record_remove"));
        }
        (capability.recorded, rendered.source)
    };
    let (available, source) = recorded(&original);
    assert!(available);
    assert!(source.contains("pub fn setWatch<Private"));
    assert!(source.contains("watchList"));
    assert!(source.contains("record_insert("));
    assert!(source.contains("record_remove("));

    fn actions(contract: &mut serde_json::Value) -> &mut Vec<serde_json::Value> {
        contract["stateful_circuits"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|circuit| circuit["name"] == "setWatch")
            .unwrap()["actions"][0]["action"]["action"]["actions"]
            .as_array_mut()
            .unwrap()
    }
    let mut wrong_key = original.clone();
    actions(&mut wrong_key)[2]["condition"]["arguments"][0]["value"]["name"] =
        serde_json::json!("recordId");
    assert!(recorded(&wrong_key).0);

    let mut wrong_enum = original.clone();
    actions(&mut wrong_enum)[1]["condition"]["otherwise"]["right"]["variant"] =
        serde_json::json!("Add");
    assert!(recorded(&wrong_enum).0);

    let mut reordered = original.clone();
    actions(&mut reordered).swap(2, 3);
    assert!(recorded(&reordered).0);

    let mut wrong_set = original.clone();
    actions(&mut wrong_set)[3]["then"]["actions"][1]["field"] = serde_json::json!("retiredKeys");
    assert!(recorded(&wrong_set).0);

    let mut wrong_drop = original.clone();
    let absent_condition = actions(&mut wrong_drop)[3]["then"]["actions"][0]["condition"].clone();
    actions(&mut wrong_drop)[3]["otherwise"]["then"]["actions"][0]["condition"] = absent_condition;
    assert!(recorded(&wrong_drop).0);

    let mut duplicate_map = original.clone();
    let helper = duplicate_map["stateful_circuits"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|circuit| circuit["name"] == "recordExists")
        .unwrap();
    helper["return_value"]["value"]["otherwise"]["field"] = serde_json::json!("records");
    assert!(recorded(&duplicate_map).0);
    let mut missing_set = original;
    actions(&mut missing_set)[3]["then"]["actions"][1]["field"] = serde_json::json!("missing_set");
    assert!(!recorded(&missing_set).0);
}
