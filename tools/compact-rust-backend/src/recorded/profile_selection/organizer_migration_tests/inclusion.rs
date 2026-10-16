// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;

#[test]
fn complete_old_both_interfaces_compose_names_prefixes_and_local_shadowing() {
    for case in [
        "original",
        "renamed",
        "raw_names",
        "prefix",
        "internal_hash",
        "shadowing",
    ] {
        let mut v = fixture();
        match case {
            "original" => {}
            "renamed" => rename(
                &mut v,
                &[
                    ("local_sk", "load_secret"),
                    ("local_sk_or_error", "require_secret"),
                    ("public_key", "derive_key"),
                    ("organizer_pks", "controllers"),
                    ("eligible_participants", "members"),
                    ("MaybeCompact1", "OptionalSecret"),
                    ("maybe_sk", "credential"),
                ],
            ),
            "raw_names" => rename(
                &mut v,
                &[
                    ("local_sk", "r#type"),
                    ("public_key", "r#match"),
                    ("organizer_pks", "r#loop"),
                ],
            ),
            "prefix" => {
                named(&mut v, "circuits", "public_key")["body"]["value"]["elements"][0]["bytes"] =
                    json!(vec![19u8; 32])
            }
            "internal_hash" => named(&mut v, "circuits", "public_key")["internal"] = json!(true),
            "shadowing" => {
                rename(
                    named(&mut v, "stateful_circuits", "add_participant"),
                    &[("tmp_8", "participant")],
                );
                rename(
                    named(&mut v, "stateful_circuits", "local_sk_or_error"),
                    &[("maybe_sk", "sk")],
                );
            }
            _ => unreachable!(),
        }
        assert_old_positive(&contract(v), case);
    }
}

#[test]
fn complete_old_same_distinct_and_chunked_set_slots_preserve_identity() {
    let c = contract(fixture());
    let root = c
        .stateful_circuits
        .iter()
        .find(|c| c.name == "add_organizer")
        .unwrap();
    let crate::ir::StateAction::SetInsert { field, .. } = &root.actions[1] else {
        unreachable!()
    };
    assert_eq!(
        field, "organizer_pks",
        "original same-slot authority and mutation"
    );
    assert_old_positive(&c, "same-slot");

    let mut v = fixture();
    v["ledger_fields"].as_array_mut().unwrap().push(json!({"id":"new_organizers","index":3,"path":[3],"declaration":{"kind":"set","ty":{"kind":"bytes","length":32}}}));
    let root = named(&mut v, "stateful_circuits", "add_organizer");
    root["actions"][1]["field"] = json!("new_organizers");
    root["actions"][1]["index"] = json!(3);
    assert_old_positive(&contract(v), "distinct-bytes-slot");

    let mut v = fixture();
    let fields = v["ledger_fields"].as_array_mut().unwrap();
    while fields.len() < 17 {
        fields.push(json!({"id":format!("padding{}",fields.len()),"index":0,"path":[],"declaration":{"kind":"counter"}}));
    }
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
    assert_old_positive(&contract(v), "native-valid-chunked-slots");
}

#[test]
fn legitimate_helper_shape_departures_keep_source_order() {
    let mut v = fixture();
    let hash = named(&mut v, "circuits", "public_key");
    let formal = hash["parameters"][0]["name"].clone();
    hash["body"] = json!({"kind":"parameter","name":formal});
    let helper = named(&mut v, "stateful_circuits", "local_sk_or_error");
    helper["return_value"]["value"]["body"]["steps"].as_array_mut().unwrap().push(json!({"kind":"assert","condition":{"kind":"boolean","value":true},"message":"additional secret guard"}));
    let c = contract(v);
    for name in ROOTS {
        assert!(old(&c, name).unwrap().is_none());
        let p = checked(&c, name).unwrap();
        let steps = &p.steps;
        let t = quote::quote!(#(#steps)*).to_string();
        assert!(
            t.find("No secret key found").unwrap() < t.find("additional secret guard").unwrap()
        );
        assert!(t.find("additional secret guard").unwrap() < t.find("record_member").unwrap());
        assert!(t.find("Not an organizer").unwrap() < t.find("record_insert").unwrap());
    }
    assert_actual(&c);
}
