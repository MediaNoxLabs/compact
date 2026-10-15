// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;
#[test]
fn complete_old_set_admission_includes_real_helper_discovery_and_transitivity() {
    assert_old_positive(&contract(full_fixture()), "complete genuine019");
    for case in [
        "only_watch",
        "transitive",
        "raw_helpers",
        "same_helper",
        "pure_guard_helper",
        "public_helpers",
    ] {
        let mut v = fixture();
        match case {
            "only_watch" => {}
            "transitive" => {
                let mut inner = named(&mut v, "assertWritable").clone();
                inner["name"] = json!("checkWritable");
                named(&mut v, "assertWritable")["actions"] =
                    json!([{"kind":"circuit_call","name":"checkWritable","arguments":[]}]);
                v["stateful_circuits"].as_array_mut().unwrap().push(inner);
            }
            "raw_helpers" => rename(
                &mut v,
                &[
                    ("assertWritable", "r#type"),
                    ("recordWrite", "r#match"),
                    ("recordExists", "r#loop"),
                ],
            ),
            "same_helper" => actions(&mut v)[4]["name"] = json!("assertWritable"),
            "pure_guard_helper" => {
                named(&mut v, "assertWritable")["actions"] = json!([{"kind":"assert","condition":{"kind":"boolean","value":true},"message":"pure helper guard"}])
            }
            "public_helpers" => {
                named(&mut v, "assertWritable")["internal"] = json!(false);
                named(&mut v, "recordWrite")["internal"] = json!(false);
            }
            _ => unreachable!(),
        }
        assert_old_positive(&contract(v), case);
    }
}
#[test]
fn complete_old_key_only_maps_and_chunked_slots_keep_declared_identity() {
    for case in [
        "field_values",
        "opaque_values",
        "renamed_shadowing",
        "reordered_slots",
    ] {
        let mut v = fixture();
        match case {
            "field_values" | "opaque_values" => {
                let ty = if case == "field_values" {
                    json!({"kind":"field"})
                } else {
                    json!({"kind":"opaque_bytes"})
                };
                for f in v["ledger_fields"].as_array_mut().unwrap() {
                    if f["id"] == "records" || f["id"] == "custodyGrants" {
                        f["declaration"]["value"] = ty.clone();
                    }
                }
            }
            "renamed_shadowing" => rename(
                &mut v,
                &[
                    ("records", "firstRecords"),
                    ("custodyGrants", "secondRecords"),
                    ("watchList", "r#type"),
                    ("ListMutation", "WatchAction"),
                    ("disclosedId", "recordId"),
                    ("disclosedMutation", "mutation"),
                ],
            ),
            "reordered_slots" => {
                let fields = v["ledger_fields"].as_array_mut().unwrap();
                fields.reverse();
                let mut slots = HashMap::new();
                for (n, f) in fields.iter_mut().enumerate() {
                    let (outer, inner) = if n < 5 { (0, n) } else { (1, n - 5) };
                    f["index"] = json!(outer);
                    f["path"] = json!([outer, inner]);
                    slots.insert(f["id"].as_str().unwrap().to_owned(), outer);
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
        assert_old_positive(&contract(v), case);
    }
}
#[test]
fn legitimate_old_shape_departures_preserve_composed_mutations() {
    for case in ["same_map", "extra_assertion"] {
        let mut v = fixture();
        match case {
            "same_map"=>{
                let value=&mut named(&mut v,"recordExists")["return_value"]["value"];
                value["otherwise"]=value["condition"].clone();
            },
            "extra_assertion"=>actions(&mut v).as_array_mut().unwrap().insert(3,json!({"kind":"assert","condition":{"kind":"boolean","value":true},"message":"additional guard before mutation"})),
            _=>unreachable!()
        }
        let c = contract(v);
        let cx = Context::new(&c);
        assert!(cx.old().unwrap().is_none());
        let plan = cx.checked().unwrap();
        let steps = &plan.steps;
        let t = quote::quote!(#(#steps)*).to_string();
        assert!(t.contains("record_insert") && t.contains("record_remove"));
        if case == "extra_assertion" {
            assert!(
                t.find("additional guard before mutation").unwrap()
                    < t.find("record_insert").unwrap()
            );
        }
        assert_current(&c);
    }
}
