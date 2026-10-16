// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;

#[test]
fn whole_old_successor_grammar_composes_before_actual_owner_selection() {
    for case in ["leaf", "coerce", "if", "nested_both_arms", "original"] {
        let mut v = fixture();
        let helper = named_mut(&mut v, "circuits", "successor");
        let ty = helper["result"].clone();
        let leaf = json!({"kind":"enum_variant", "ty":ty, "variant":"commit"});
        let condition = helper["body"]["condition"].clone();
        let branch = json!({"kind":"if", "condition":condition,"then":leaf,"otherwise":leaf});
        helper["body"] = match case {
            "leaf" => leaf,
            "coerce" => json!({"kind":"coerce", "ty":ty,"value":leaf}),
            "if" => branch,
            "nested_both_arms" => {
                json!({"kind":"if", "condition":condition,"then":branch,"otherwise":branch})
            }
            "original" => helper["body"].clone(),
            _ => unreachable!(),
        };
        let c = contract(v);
        assert!(old(&c).unwrap().is_some(), "whole old {case}");
        assert!(checked(&c).is_ok(), "checked {case}");
        assert_actual_owner(&c);
    }
}

#[test]
fn whole_old_declarations_names_shadowing_and_physical_slots_are_preserved() {
    for case in [
        "renamed",
        "raw_names",
        "enum_domain",
        "shadowing",
        "chunked",
    ] {
        let mut v = fixture();
        match case {
            "renamed" => rename(
                &mut v,
                &[
                    ("successor", "next_phase"),
                    ("PublicState", "Phase"),
                    ("setup", "Draft"),
                    ("topic", "caption"),
                    ("is_some", "present"),
                    ("Maybe", "Optional"),
                ],
            ),
            "raw_names" => rename(
                &mut v,
                &[
                    ("successor", "r#match"),
                    ("topic", "r#return"),
                    ("is_some", "r#type"),
                    ("state", "r#loop"),
                ],
            ),
            "enum_domain" => edit(&mut v, &mut |node| {
                if node.get("kind") == Some(&json!("enum"))
                    && node.get("name") == Some(&json!("PublicState"))
                {
                    node.insert(
                        "variants".into(),
                        json!(["final", "reveal", "commit", "setup", "extra"]),
                    );
                }
            }),
            "shadowing" => rename(
                named_mut(&mut v, "stateful_circuits", "advance"),
                &[("tmp_134", "apk")],
            ),
            "chunked" => chunked(&mut v),
            _ => unreachable!(),
        }
        let c = contract(v);
        assert!(old(&c).unwrap().is_some(), "whole old {case}");
        assert!(checked(&c).is_ok(), "checked {case}");
        assert_actual_owner(&c);
    }
}
fn chunked(v: &mut Value) {
    let fields = v["ledger_fields"].as_array_mut().unwrap();
    while fields.len() < 17 {
        fields.push(json!({"id":format!("padding{}",fields.len()),"index":0,"path":[],"declaration":{"kind":"counter"}}));
    }
    let mut indices = HashMap::new();
    for (pos, field) in fields.iter_mut().enumerate() {
        let (outer, inner) = if pos < 2 { (0, pos) } else { (1, pos - 2) };
        field["index"] = json!(outer);
        field["path"] = json!([outer, inner]);
        indices.insert(field["id"].as_str().unwrap().to_owned(), outer);
    }
    edit(v, &mut |node| {
        if let Some(index) = node
            .get("field")
            .and_then(Value::as_str)
            .and_then(|f| indices.get(f))
            && node.contains_key("index")
        {
            node.insert("index".into(), json!(index));
        }
    });
}

#[test]
fn reordered_guards_and_asserted_successors_remain_valid_composition() {
    let mut v = fixture();
    actions(&mut v).as_array_mut().unwrap().swap(0, 1);
    let helper = named_mut(&mut v, "circuits", "successor");
    let body = helper["body"].take();
    helper["body"] = json!({"kind":"sequence","steps":[{"kind":"assert","condition":{"kind":"boolean","value":true},"message":"pure guard"}],"value":body});
    let c = contract(v);
    assert!(old(&c).unwrap().is_none());
    let plan = checked(&c).unwrap();
    let steps = &plan.steps;
    let tokens = quote::quote!(#(#steps)*).to_string();
    assert!(
        tokens.find("topic . record_read").unwrap()
            < tokens.find("authority . record_read").unwrap()
    );
    assert!(
        tokens.find("authority . record_read").unwrap()
            < tokens.find("state . record_write").unwrap()
    );
    assert_actual_owner(&c);
}
