// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;

#[test]
fn complete_old_named_declarations_and_flat_slot_identity_compose() {
    for case in ["original", "renamed", "raw_names", "flat_reorder"] {
        let mut v = fixture();
        match case {
            "original" => {}
            "renamed" => {
                v = renamed(
                    v,
                    &[
                        (PATH_WITNESS, "lookup_path"),
                        ("eligible_voters", "registry"),
                        ("private$secret_key", "load_credential"),
                        ("public_key", "derive_authority"),
                        ("authority", "controller"),
                        ("PublicState", "Phase"),
                        ("setup", "Draft"),
                        ("MaybeCompact1", "OptionalPath"),
                        ("is_some", "present"),
                    ],
                )
            }
            "raw_names" => {
                v = renamed(
                    v,
                    &[
                        (PATH_WITNESS, "r#match"),
                        ("eligible_voters", "r#loop"),
                        ("is_some", "r#type"),
                    ],
                )
            }
            "flat_reorder" => {
                let fields = v["ledger_fields"].as_array_mut().unwrap();
                fields.swap(5, 6);
                let mut slots = HashMap::new();
                for (i, f) in fields.iter_mut().enumerate() {
                    f["index"] = json!(i);
                    f["path"] = json!([i]);
                    slots.insert(f["id"].as_str().unwrap().to_owned(), i);
                }
                edit(&mut v, &mut |n| {
                    if let Some(i) = n
                        .get("field")
                        .and_then(Value::as_str)
                        .and_then(|f| slots.get(f))
                        && n.contains_key("index")
                    {
                        n.insert("index".into(), json!(i));
                    }
                });
            }
            _ => unreachable!(),
        }
        let c = contract(v);
        assert!(old(&c).unwrap().is_some(), "complete old {case}");
        assert!(checked(&c).is_ok(), "checked {case}");
        assert_actual(&c);
    }
}

fn depth(mut v: Value, depth: u8) -> Value {
    // Keep every use of the same named path type canonical in the whole source,
    // including the second tree and path-consuming exports.
    edit(&mut v, &mut |n| {
        if n.get("kind") == Some(&json!("merkle_tree")) {
            n.insert("depth".into(), json!(depth));
        }
        // The standard-library fold carries its own source-vector length.
        // Preserve this expression-level annotation alongside named types.
        if matches!(
            n.get("kind").and_then(Value::as_str),
            Some("vector" | "vector_fold_call")
        ) && n.get("element").and_then(|e| e.get("name")) == Some(&json!("MerkleTreePathEntry"))
        {
            n.insert("length".into(), json!(depth));
        }
    });
    v
}
#[test]
fn complete_old_depth_bounds_are_separate_from_native_validity() {
    for d in [2, 10, 32] {
        let c = contract(depth(fixture(), d));
        assert!(old(&c).unwrap().is_some(), "complete old depth {d}");
        assert!(checked(&c).is_ok(), "checked depth {d}");
        assert_actual(&c);
    }
    for d in [1, 33] {
        let c = contract(depth(fixture(), d));
        assert!(
            old(&c).unwrap().is_some(),
            "old grammar lacks native depth bound {d}"
        );
        let (result, rows) = trace::capture_json(|| crate::render_with_capabilities(&c));
        assert!(result.is_err());
        assert!(rows.as_array().unwrap().is_empty());
    }
}

#[test]
fn independent_composition_preserves_changed_order_and_literal_argument() {
    let mut v = fixture();
    root_actions(&mut v).as_array_mut().unwrap().swap(0, 1);
    let c = contract(v);
    assert!(old(&c).unwrap().is_none());
    let plan = checked(&c).unwrap();
    let steps = &plan.steps;
    let tokens = quote::quote!(#(#steps)*).to_string();
    assert!(
        tokens.find("authority . record_read").unwrap()
            < tokens.find("context_eligible_voters_path_of").unwrap()
    );
    assert_actual(&c);

    let mut v = fixture();
    root_actions(&mut v)[0]["condition"]["condition"]["value"]["arguments"][0]["value"] =
        json!({"kind":"bytes_literal","bytes":vec![0u8;32]});
    let c = contract(v);
    assert!(old(&c).unwrap().is_none());
    assert!(checked(&c).is_ok());
    assert_actual(&c);
}
