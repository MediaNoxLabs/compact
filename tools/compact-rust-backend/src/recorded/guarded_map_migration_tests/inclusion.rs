// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;
#[test]
fn whole_source_and_closed_pure_guard_grammar_keep_map_order() {
    assert_old_positive(&contract(fixture()), "complete source019");
    for case in [
        "assert",
        "branch",
        "let_coerce",
        "nested_sequence",
        "original",
    ] {
        let mut v = isolated();
        let assertion =
            json!({"kind":"assert","condition":{"kind":"boolean","value":true},"message":"guard"});
        let body = match case {
            "assert" => assertion,
            "branch" => {
                json!({"kind":"if","condition":{"kind":"boolean","value":false},"then":assertion,"otherwise":{"kind":"unit"}})
            }
            "let_coerce" => {
                json!({"kind":"let","bindings":[{"name":"local_guard","ty":{"kind":"boolean"},"value":{"kind":"coerce","ty":{"kind":"boolean"},"value":{"kind":"boolean","value":true}}}],"body":{"kind":"assert","condition":{"kind":"parameter","name":"local_guard"},"message":"local guard"}})
            }
            "nested_sequence" => {
                json!({"kind":"sequence","steps":[{"kind":"sequence","steps":[assertion],"value":{"kind":"unit"}}],"value":{"kind":"unit"}})
            }
            _ => pure(&mut v)["body"].clone(),
        };
        pure(&mut v)["body"] = body;
        assert_old_positive(&contract(v), case);
    }
}
#[test]
fn canonical_public_products_and_owned_map_members_preserve_old_domain() {
    let domains = [
        json!({"kind":"unit"}),
        json!({"kind":"opaque_bytes"}),
        json!({"kind":"unsigned","max":"340282366920938463463374607431768211455"}),
        json!({"kind":"tuple","elements":vec![json!({"kind":"field"});11]}),
    ];
    for ty in domains {
        let mut v = isolated();
        root(&mut v)["parameters"]
            .as_array_mut()
            .unwrap()
            .push(json!({"name":"unused","ty":ty}));
        assert_old_positive(&contract(v), "extra public parameter");
    }
    for ty in [
        json!({"kind":"unit"}),
        json!({"kind":"opaque_bytes"}),
        json!({"kind":"struct","name":"EmptyMember","fields":[]}),
        json!({"kind":"vector","length":2,"element":{"kind":"tuple","elements":vec![json!({"kind":"field"});8]}}),
    ] {
        let mut v = isolated();
        edit(&mut v, &mut |n| {
            if n.get("kind") == Some(&json!("struct"))
                && n.get("name") == Some(&json!("AssetRecord"))
            {
                n["fields"]
                    .as_array_mut()
                    .unwrap()
                    .push(json!({"name":"unused","ty":ty}));
            }
        });
        assert_old_positive(&contract(v), "canonical owned Map member");
    }
    // The old guard admits a zero-field Map value too. This synthetic complete
    // contract still performs membership and lookup; only its pure assertion
    // stops projecting fields that no longer exist.
    let mut empty = isolated();
    edit(&mut empty, &mut |n| {
        if n.get("kind") == Some(&json!("struct")) && n.get("name") == Some(&json!("AssetRecord")) {
            n.insert("fields".into(), json!([]));
        }
    });
    pure(&mut empty)["body"] = json!({"kind":"assert","condition":{"kind":"boolean","value":true},"message":"empty stored product"});
    assert_old_positive(&contract(empty), "empty named Map value");

    // An extra owned argument reaches the pure callee, not merely the public ABI.
    let mut v = isolated();
    let param = json!({"name":"owned","ty":{"kind":"opaque_bytes"}});
    root(&mut v)["parameters"]
        .as_array_mut()
        .unwrap()
        .push(param.clone());
    pure(&mut v)["parameters"]
        .as_array_mut()
        .unwrap()
        .push(param);
    call(&mut v)["arguments"].as_array_mut().unwrap().push(json!({"kind":"coerce","ty":{"kind":"opaque_bytes"},"value":{"kind":"parameter","name":"owned"}}));
    assert_old_positive(&contract(v), "owned pure argument");
}
#[test]
fn physical_slot_authority_and_lexical_shadowing_do_not_depend_on_spelling() {
    for case in ["raw_names", "key_shadow", "value_shadow", "reverse_layout"] {
        let mut v = isolated();
        match case {
            "raw_names" => {
                rename(&mut v, "records", "r#type");
                rename(&mut v, "disclosedId", "r#match");
            }
            "key_shadow" => rename(&mut v, "disclosedId", "recordId"),
            "value_shadow" => {
                let ty = sequence(&mut v)[1]["bindings"][0]["ty"].clone();
                root(&mut v)["parameters"]
                    .as_array_mut()
                    .unwrap()
                    .push(json!({"name":"record","ty":ty}));
            }
            "reverse_layout" => {
                let fields = v["ledger_fields"].as_array_mut().unwrap();
                // Permute declarations over the existing canonical physical
                // layout; arbitrary equal-sized chunks are not ledger paths.
                let layout: Vec<_> = fields
                    .iter()
                    .map(|f| (f["index"].clone(), f["path"].clone()))
                    .collect();
                fields.reverse();
                let mut indices = HashMap::new();
                for (f, (index, path)) in fields.iter_mut().zip(layout) {
                    f["index"] = index.clone();
                    f["path"] = path;
                    indices.insert(f["id"].as_str().unwrap().to_owned(), index);
                }
                edit(&mut v, &mut |n| {
                    if let Some(i) = n
                        .get("field")
                        .and_then(Value::as_str)
                        .and_then(|f| indices.get(f))
                        && n.contains_key("index")
                    {
                        n.insert("index".into(), json!(i));
                    }
                });
            }
            _ => unreachable!(),
        }
        assert_old_positive(&contract(v), case);
    }
}
