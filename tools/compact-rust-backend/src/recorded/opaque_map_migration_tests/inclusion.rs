// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;
#[test]
fn complete_old_map_write_admission_uses_real_transitive_helpers() {
    let original = contract(fixture());
    for name in ROOTS {
        let text = assert_old_positive(&original, name, "complete genuine019");
        assert!(
            text.find("registry is closed").unwrap()
                < text.find("mutation must be Insert or Update").unwrap()
        );
        assert!(
            text.find("record_insert").unwrap() < text.rfind("record_increment").unwrap(),
            "write helper follows insertion"
        );
        assert_eq!(
            text.contains("pure_circuits"),
            name == "setRecord",
            "optional class guard"
        );
    }
    for mode in ["pure_writable", "pure_write", "transitive", "same_helper"] {
        let mut v = fixture();
        match mode {
            "pure_writable" | "pure_write" => {
                let name = if mode == "pure_writable" {
                    "assertWritable"
                } else {
                    "recordWrite"
                };
                named(&mut v, name)["actions"] = json!([{"kind":"assert","condition":{"kind":"boolean","value":true},"message":"pure helper"}]);
            }
            "transitive" => {
                let mut helper = named(&mut v, "assertWritable").clone();
                helper["name"] = json!("checkWritable");
                v["stateful_circuits"].as_array_mut().unwrap().push(helper);
                named(&mut v, "assertWritable")["actions"] =
                    json!([{"kind":"circuit_call","name":"checkWritable","arguments":[]}]);
            }
            "same_helper" => {
                for name in ROOTS {
                    actions(&mut v, name)
                        .as_array_mut()
                        .unwrap()
                        .last_mut()
                        .unwrap()["name"] = json!("assertWritable");
                }
            }
            _ => unreachable!(),
        }
        let c = contract(v);
        for name in ROOTS {
            assert_old_positive(&c, name, mode);
        }
    }
}
#[test]
fn typed_products_map_topology_and_binding_names_preserve_complete_old_writes() {
    for mode in [
        "nested_product",
        "empty_product",
        "other_nested_map",
        "reversed_exists",
        "raw_shadow_names",
        "permuted_layout",
    ] {
        let name = if mode == "empty_product" {
            "setCustodyGrant"
        } else {
            "setRecord"
        };
        let mut v = isolated(name);
        match mode {
            "nested_product" => edit(&mut v, &mut |n| {
                if n.get("kind") == Some(&json!("struct"))
                    && n.get("name") == Some(&json!("AssetRecord"))
                {
                    n["fields"].as_array_mut().unwrap().push(json!({"name":"codec","ty":{"kind":"vector","length":2,"element":{"kind":"tuple","elements":[{"kind":"field"},{"kind":"unsigned","max":"680564733841876926926749214863536422911"}]}}}));
                }
            }),
            "empty_product" => edit(&mut v, &mut |n| {
                if n.get("kind") == Some(&json!("struct"))
                    && n.get("name") == Some(&json!("CustodyGrant"))
                {
                    n.insert("fields".into(), json!([]));
                }
            }),
            "other_nested_map" => {
                let slot = v["ledger_fields"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|x| x["id"] == "custodyGrants")
                    .unwrap();
                slot["declaration"]["value"] =
                    json!({"kind":"ledger_map","key":{"kind":"field"},"value":{"kind":"field"}});
                // The original unrelated witness ledger view exposes every Map
                // value and cannot express a nested Map. Keep that native refusal
                // explicit before isolating the key-only complete contract.
                assert!(matches!(
                    crate::render_with_capabilities(&contract(v.clone())),
                    Err(RenderError::Located { error, .. })
                        if matches!(*error, RenderError::UnsupportedLedgerValueType(Type::LedgerMap { .. }))
                ));
                v["witnesses"] = json!([]);
                // Preserve both public Counter effects of the actual write helper;
                // remove only its timestamp witness and dependent Cell assignment.
                named(&mut v, "recordWrite")["actions"]
                    .as_array_mut()
                    .unwrap()
                    .truncate(2);
            }
            "reversed_exists" => {
                let value = &mut named(&mut v, "recordExists")["return_value"]["value"];
                let first = value["condition"].clone();
                value["condition"] = value["otherwise"].clone();
                value["otherwise"] = first;
            }
            "raw_shadow_names" => {
                for (from, to) in [
                    ("records", "r#type"),
                    ("assertWritable", "r#match"),
                    ("disclosedId", "recordId"),
                    ("disclosedRecord", "record"),
                    ("disclosedMutation", "mutation"),
                ] {
                    rename(&mut v, from, to);
                }
            }
            "permuted_layout" => {
                let fields = v["ledger_fields"].as_array_mut().unwrap();
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
                        n.insert("index".into(), i.clone());
                    }
                });
            }
            _ => unreachable!(),
        }
        assert_old_positive(&contract(v), name, mode);
    }
}
#[test]
fn legitimate_class_counter_and_typed_grammar_departures_remain_compositional() {
    for mode in [
        "class_without_count",
        "count_without_class",
        "extra_guard",
        "extra_variant",
    ] {
        let mut v = isolated("setRecord");
        match mode {
            "class_without_count"=>{
                let arm=&mut branch(&mut v,"setRecord")["otherwise"]["then"];
                *arm=arm["actions"][0].clone();
            },
            "count_without_class"=>{actions(&mut v,"setRecord").as_array_mut().unwrap().remove(2);},
            "extra_guard"=>actions(&mut v,"setRecord").as_array_mut().unwrap().insert(2,json!({"kind":"assert","condition":{"kind":"boolean","value":true},"message":"extra composed guard"})),
            "extra_variant"=>edit(&mut v,&mut |n|{if n.get("kind")==Some(&json!("enum"))&&n.get("name")==Some(&json!("RecordMutation")){n["variants"].as_array_mut().unwrap().push(json!("Reserved"));}}),
            _=>unreachable!()
        }
        let c = contract(v);
        assert!(
            Context::new(&c, "setRecord").old().unwrap().is_none(),
            "{mode}"
        );
        let text = assert_current(&c, "setRecord");
        if mode == "extra_guard" {
            assert!(
                text.find("extra composed guard").unwrap() < text.find("record_remove").unwrap()
            );
        }
        if mode == "count_without_class" {
            assert!(!text.contains("pure_circuits"));
        }
    }
}
