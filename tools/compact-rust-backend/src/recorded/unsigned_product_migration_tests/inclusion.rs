// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;

#[test]
fn complete_old_product_preserves_names_flags_shadowing_and_assertion_order() {
    for case in [
        "original",
        "renamed",
        "raw_names",
        "pure_internal",
        "shadowing",
        "message",
    ] {
        let mut v = fixture();
        match case {
            "original" => {}
            "renamed" => rename(
                &mut v,
                &[
                    ("assertProductLE", "check_bound"),
                    ("q", "factor"),
                    ("y", "ceiling"),
                    ("t_13", "product"),
                    ("mixedOps", "accepted"),
                    ("tmp_15", "one"),
                ],
            ),
            "raw_names" => rename(
                &mut v,
                &[
                    ("assertProductLE", "r#type"),
                    ("q", "r#match"),
                    ("mixedOps", "r#loop"),
                ],
            ),
            "pure_internal" => pure(&mut v)["internal"] = json!(true),
            "shadowing" => {
                rename(pure(&mut v), &[("t_13", "q")]);
                rename(root(&mut v), &[("tmp_15", "q")]);
            }
            "message" => {
                pure(&mut v)["body"]["steps"][0]["message"] = json!("caller-defined guard message")
            }
            _ => unreachable!(),
        }
        assert_old_positive(&contract(v), case);
    }
    // Internal roots have no public facade, but the old rule places no flag
    // restriction on them. Qualify their body and native source separately.
    let mut v = fixture();
    root(&mut v)["internal"] = json!(true);
    let c = contract(v);
    assert!(old(&c).unwrap().is_some());
    let plan = checked(&c).unwrap();
    let steps = &plan.steps;
    assert!(
        quote::quote!(#(#steps)*)
            .to_string()
            .contains("record_increment")
    );
    crate::render_with_capabilities(&c).unwrap();
}

#[test]
fn complete_old_counter_uses_declared_nonzero_and_chunked_paths() {
    assert_old_positive(&contract(fixture()), "original nonzero slot");
    let mut v = fixture();
    let fields = v["ledger_fields"].as_array_mut().unwrap();
    while fields.len() < 17 {
        fields.push(json!({"id":format!("padding{}",fields.len()),"index":0,"path":[],"declaration":{"kind":"counter"}}));
    }
    fields.swap(1, 16);
    let mut slots = HashMap::new();
    for (position, field) in fields.iter_mut().enumerate() {
        let (outer, inner) = if position < 2 {
            (0, position)
        } else {
            (1, position - 2)
        };
        field["index"] = json!(outer);
        field["path"] = json!([outer, inner]);
        slots.insert(field["id"].as_str().unwrap().to_owned(), outer);
    }
    edit(&mut v, &mut |n| {
        if let Some(index) = n
            .get("field")
            .and_then(Value::as_str)
            .and_then(|f| slots.get(f))
            && n.contains_key("index")
        {
            n.insert("index".into(), json!(index));
        }
    });
    assert_old_positive(&contract(v), "complete chunked layout");
}

#[test]
fn changed_valid_arithmetic_is_composed_without_the_old_shape_constants() {
    for (case, factor, operator) in [("factor", "3", "less_equal"), ("comparison", "4", "less")] {
        let mut v = fixture();
        condition(&mut v)["bindings"][0]["value"]["right"]["value"] = json!(factor);
        condition(&mut v)["body"]["operator"] = json!(operator);
        let c = contract(v);
        assert!(old(&c).unwrap().is_none(), "old {case}");
        let plan = checked(&c).unwrap();
        let steps = &plan.steps;
        let text = quote::quote!(#(#steps)*).to_string();
        assert!(text.find("assertProductLE").unwrap() < text.find("record_increment").unwrap());
        // The native pure function, including the modified operation, remains
        // the authority of this PureCall, rather than the retired recognizer.
        assert_current(&c);
    }
}
