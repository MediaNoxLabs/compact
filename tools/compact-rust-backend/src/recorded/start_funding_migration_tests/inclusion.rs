// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;

#[test]
fn complete_original_sources_preserve_funding_and_branch_witness_order() {
    let original = parse(source());
    assert_witness_order(&included(&original, "start"));
    let whole: Contract = serde_json::from_str(include_str!(
        "../../../tests/coracle-source-schema20-ir.json"
    ))
    .unwrap();
    assert_witness_order(&included(&whole, "start"));
    // These are static branch totals. Dynamic red/blue traces are qualified by
    // the maintained27-row native/recorded/replay consumer, not inferred here.
}

#[test]
fn coherent_identifiers_and_authoritative_flat_slots_preserve_whole_old_domain() {
    let mut value = source();
    for (from, to) in [
        ("start", "type"),
        ("receiveShielded", "receive_renamed"),
        ("mergeCoinImmediate", "merge_received"),
        ("mergeCoin", "merge_historical"),
        ("coinCommitment", "commitment_renamed"),
        ("pos", "position_renamed"),
        ("wager", "wager_renamed"),
        ("deposit", "deposit_renamed"),
        ("red_board", "board_first"),
        ("Player", "Participant"),
    ] {
        rename(&mut value, from, to);
    }
    included(&parse(value.clone()), "type");
    let indices: HashMap<String, u64> = value["ledger_fields"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| {
            (
                v["id"].as_str().unwrap().to_owned(),
                8 - v["index"].as_u64().unwrap(),
            )
        })
        .collect();
    for field in value["ledger_fields"].as_array_mut().unwrap() {
        field["index"] = json!(indices[field["id"].as_str().unwrap()]);
    }
    edit(&mut value, &mut |node| {
        if let Some(index) = node
            .get("field")
            .and_then(Value::as_str)
            .and_then(|id| indices.get(id))
            && node.contains_key("index")
        {
            node.insert("index".into(), json!(index));
        }
    });
    // The frontend records explicit physical paths even for flat slots. Merely
    // permuting `index` leaves stale authority: old qualified-cell resolution
    // and mandatory native layout validation correctly reject that fixture.
    let stale_paths = parse(value.clone());
    assert!(old(&stale_paths, "type").is_none());
    assert!(crate::render_with_capabilities(&stale_paths).is_err());
    for field in value["ledger_fields"].as_array_mut().unwrap() {
        field["path"] = json!([field["index"].as_u64().unwrap()]);
    }
    let renamed = parse(value);
    included(&renamed, "type");
    let mut internal = parse(source());
    internal
        .stateful_circuits
        .iter_mut()
        .find(|c| c.name == "start")
        .unwrap()
        .internal = true;
    assert!(old(&internal, "start").is_some());
    assert!(checked(&internal, "start").is_ok());
    let output = crate::render_with_capabilities(&internal).unwrap();
    assert!(
        output
            .capabilities
            .circuits
            .iter()
            .all(|c| c.name != "start")
    );
}

#[test]
fn branch_swap_and_effectless_scaffolding_preserve_complete_funding_provenance() {
    let mut value = source();
    let mut swapped = 0;
    edit(&mut root(&mut value)["return_value"], &mut |node| {
        if node.get("kind") == Some(&json!("conditional")) {
            let then = node.remove("then").unwrap();
            let otherwise = node.remove("otherwise").unwrap();
            node.insert("then".into(), otherwise);
            node.insert("otherwise".into(), then);
            swapped += 1;
        }
    });
    assert_eq!(swapped, 1);
    prefix_binding(
        &mut value,
        "position_alias",
        json!({"kind":"field"}),
        json!({"kind":"parameter","name":"pos"}),
    );
    let body = root(&mut value)["return_value"]["body"].take();
    root(&mut value)["return_value"]["body"] = json!({"kind":"sequence","actions":[{
        "kind":"assert","condition":{"kind":"boolean","value":true},"message":"prefix"}],"result":body});
    assert_witness_order(&included(&parse(value), "start"));
    // Key/board destinations need not be distinct across the principal paths.
    // Only the funded pot/deposit relation is constrained by the old grammar.
    let mut same_ordinary_slots = source();
    edit(
        &mut root(&mut same_ordinary_slots)["return_value"],
        &mut |node| {
            if node.get("kind") == Some(&json!("cell_write")) {
                let replacement = match node.get("field").and_then(Value::as_str) {
                    Some("blue") => Some(("red", 0)),
                    Some("blue_board") => Some(("red_board", 2)),
                    _ => None,
                };
                if let Some((field, index)) = replacement {
                    node.insert("field".into(), json!(field));
                    node.insert("index".into(), json!(index));
                }
            }
        },
    );
    included(&parse(same_ordinary_slots), "start");
    // Same valid received coin twice is an old funding-grammar rejection; it is
    // still a typed checked program. Current admission must not reintroduce the
    // recognizer's funding policy as a global operation restriction.
    let mut repeated = source();
    let mut calls = 0;
    edit(&mut root(&mut repeated)["return_value"], &mut |node| {
        if node.get("kind") == Some(&json!("circuit_call"))
            && node.get("name") == Some(&json!("receiveShielded"))
        {
            calls += 1;
            if calls == 2 {
                node.insert(
                    "arguments".into(),
                    json!([{"kind":"parameter","name":"wager"}]),
                );
            }
        }
    });
    assert_eq!(calls, 2);
    let repeated = parse(repeated);
    assert!(old(&repeated, "start").is_none());
    current(&repeated, "start"); // Native-valid expansion, not runtime funding success.
}

#[test]
fn transitive_pure_fold_and_bounded_product_values_are_complete_old_positives() {
    let mut value = source();
    value["circuits"].as_array_mut().unwrap().extend([
        json!({"name":"funding_fold_step","internal":true,"parameters":[
            {"name":"acc","ty":{"kind":"field"}},{"name":"item","ty":{"kind":"field"}}],
            "result":{"kind":"field"},"body":{"kind":"parameter","name":"acc"}}),
        json!({"name":"funding_fold","internal":true,"parameters":[],"result":{"kind":"field"},
            "body":{"kind":"vector_fold_call","name":"funding_fold_step","initial":{"kind":"field_literal","value":"0"},
            "source":{"kind":"tuple","elements":[{"kind":"field_literal","value":"1"}]},
            "accumulator":{"kind":"field"},"element":{"kind":"field"},"length":1}}),
        json!({"name":"funding_wrapper","internal":true,"parameters":[],"result":{"kind":"field"},
            "body":{"kind":"call","name":"funding_fold","arguments":[]}}),
    ]);
    prefix_binding(
        &mut value,
        "pure_result",
        json!({"kind":"field"}),
        json!({"kind":"call","name":"funding_wrapper","arguments":[]}),
    );
    let ty = json!({"kind":"struct","name":"FundingProduct","fields":[
        {"name":"nested","ty":{"kind":"vector","length":2,"element":{"kind":"tuple","elements":vec![json!({"kind":"field"});8]}}}]});
    prefix_binding(
        &mut value,
        "product",
        ty.clone(),
        json!({"kind":"default","ty":ty}),
    );
    let c = parse(value);
    assert_witness_order(&included(&c, "start"));
}
