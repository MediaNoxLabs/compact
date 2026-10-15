// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;

#[test]
fn complete_old_domain_composes_names_enum_values_shadowing_and_physical_paths() {
    for case in [
        "original",
        "renamed",
        "raw_names",
        "enum_domain",
        "domain_separator",
        "shadowing",
        "chunked_slots",
    ] {
        let mut value = fixture();
        match case {
            "renamed" => replace_strings(
                &mut value,
                &[
                    ("private$secret_key", "load_credential"),
                    ("public_key", "derive_authority"),
                    ("authority", "controller"),
                    ("state", "phase"),
                    ("topic", "caption"),
                    ("PublicState", "Phase"),
                    ("setup", "Draft"),
                    ("Maybe", "Optional"),
                    ("is_some", "present"),
                    ("value", "payload"),
                    ("sk", "credential"),
                    ("apk", "digest"),
                ],
            ),
            "raw_names" => replace_strings(
                &mut value,
                &[
                    ("private$secret_key", "r#type"),
                    ("public_key", "r#match"),
                    ("authority", "r#loop"),
                    ("topic", "r#return"),
                    ("value", "r#ref"),
                ],
            ),
            "enum_domain" => edit_nodes(&mut value, &mut |node| {
                if node.get("kind") == Some(&json!("enum"))
                    && node.get("name") == Some(&json!("PublicState"))
                {
                    node.insert(
                        "variants".into(),
                        json!(["final", "reveal", "commit", "setup", "extra"]),
                    );
                }
            }),
            "domain_separator" => {
                circuit_mut(&mut value, "circuits", "public_key")["body"]["value"]["elements"][0]
                    ["bytes"] = json!(vec![19u8; 32]);
            }
            "shadowing" => {
                // Initializer reads the root argument; only the continuation sees the new local.
                let name = continuation(&mut value)[2]["bindings"][0]["name"]
                    .as_str()
                    .unwrap()
                    .to_owned();
                replace_strings(root_mut(&mut value), &[(name.as_str(), "t")]);
            }
            "chunked_slots" => chunked(&mut value),
            _ => {}
        }
        let c = contract(value);
        assert!(old(&c).unwrap().is_some(), "whole-old {case}");
        assert!(checked(&c).is_ok(), "checked {case}");
        let rendered = crate::render_with_capabilities(&c).unwrap();
        assert!(
            rendered
                .capabilities
                .circuits
                .iter()
                .any(|c| c.name == "set_topic" && c.recorded && c.observed_call),
            "public {case}"
        );
    }
}

fn chunked(value: &mut Value) {
    let fields = value["ledger_fields"].as_array_mut().unwrap();
    while fields.len() < 17 {
        fields.push(json!({"id":format!("padding{}", fields.len()), "index":0,
            "declaration":{"kind":"counter"}, "path":[]}));
    }
    let mut indices = HashMap::new();
    // The native 17-field layout is [2, 15], not an arbitrary two-byte path.
    for (position, field) in fields.iter_mut().enumerate() {
        let (outer, inner) = if position < 2 {
            (0, position)
        } else {
            (1, position - 2)
        };
        field["index"] = json!(outer);
        field["path"] = json!([outer, inner]);
        indices.insert(field["id"].as_str().unwrap().to_owned(), outer);
    }
    edit_nodes(value, &mut |node| {
        if let Some(index) = node
            .get("field")
            .and_then(Value::as_str)
            .and_then(|field| indices.get(field))
            && node.contains_key("index")
        {
            node.insert("index".into(), json!(index));
        }
    });
}

#[test]
fn reordered_assertions_are_valid_composition_not_a_replacement_shape_gate() {
    let mut value = fixture();
    continuation(&mut value).as_array_mut().unwrap().swap(0, 1);
    let c = contract(value);
    assert!(old(&c).unwrap().is_none(), "historical order gate");
    let plan = checked(&c).unwrap();
    let steps = plan.steps;
    let syntax = quote::quote!(#(#steps)*).to_string();
    assert!(
        syntax.find("state . record_read").unwrap()
            < syntax.find("authority . record_read").unwrap()
    );
    assert!(
        syntax.find("after setup phase").unwrap() < syntax.find("without authorization").unwrap()
    );
    assert!(
        syntax.find("without authorization").unwrap()
            < syntax.find("topic . record_write").unwrap()
    );
}
