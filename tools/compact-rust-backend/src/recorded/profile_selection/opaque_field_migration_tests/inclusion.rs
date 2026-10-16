// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;
#[test]
fn whole_source_preserves_complete_put_and_ensure_semantics() {
    let c = contract(fixture());
    for name in ["put", "ensure"] {
        included(&c, name);
    }
}
#[test]
fn old_forms_preserve_names_shadowed_binding_physical_layout_and_internal_calls() {
    for mode in [
        "raw_names",
        "shadow_key",
        "nonzero",
        "chunked",
        "internal_put",
    ] {
        let mut v = fixture();
        match mode {
            "raw_names" => {
                rename(&mut v, "entries", "r#type");
                rename(&mut v, "key", "r#match");
                rename(&mut v, "value", "r#loop");
            }
            "shadow_key" => {
                let e = named(&mut v, "ensure");
                e["return_value"]["value"]["bindings"][0]["name"] = json!("key");
                e["return_value"]["value"]["body"]["name"] = json!("key");
            }
            "nonzero" => {
                v["ledger_fields"][0]["index"] = json!(1);
                v["ledger_fields"][0]["path"] = json!([1]);
                v["ledger_fields"].as_array_mut().unwrap().insert(
                    0,
                    json!({"id":"padding","index":0,"path":[0],"declaration":{"kind":"counter"}}),
                );
                edit(&mut v, &mut |n| {
                    if n.get("field") == Some(&json!("entries")) && n.contains_key("index") {
                        n.insert("index".into(), json!(1));
                    }
                });
            }
            "chunked" => {
                v["ledger_fields"][0]["path"] = json!([0, 1]);
                v["ledger_fields"].as_array_mut().unwrap().insert(0,json!({"id":"padding0","index":0,"path":[0,0],"declaration":{"kind":"counter"}}));
                for i in 0..15 {
                    v["ledger_fields"].as_array_mut().unwrap().push(json!({"id":format!("padding{}",i+1),"index":1,"path":[1,i],"declaration":{"kind":"counter"}}));
                }
            }
            "internal_put" => {
                named(&mut v, "put")["internal"] = json!(true);
                let mut wrapper = named(&mut v, "put").clone();
                wrapper["internal"] = json!(false);
                wrapper["name"] = json!("writeViaHelper");
                wrapper["actions"] = json!([{"kind":"circuit_call","name":"put","arguments":[{"kind":"parameter","name":"key"},{"kind":"parameter","name":"value"}]}]);
                v["stateful_circuits"].as_array_mut().unwrap().push(wrapper);
            }
            _ => unreachable!(),
        }
        let c = contract(v);
        if mode == "internal_put" {
            let cx = Context::new(&c);
            assert!(cx.old(cx.circuits["put"]).unwrap());
            assert!(cx.checked(cx.circuits["put"]).is_ok());
            assert!(current(&c, "writeViaHelper", "algebra.ordinary").contains("record_insert"));
        } else {
            included(&c, "put");
        }
        included(&c, "ensure");
    }
}
#[test]
fn field_interface_supports_direct_expression_indirect_and_non_map_returns() {
    for mode in ["direct", "expression", "indirect", "non_map"] {
        let mut v = fixture();
        if mode == "indirect" {
            let mut helper = named(&mut v, "ensure").clone();
            helper["name"] = json!("readChecked");
            helper["internal"] = json!(true);
            v["stateful_circuits"].as_array_mut().unwrap().push(helper);
            let r = named(&mut v, "ensure");
            r["actions"] = json!([]);
            r["return_value"] = json!({"kind":"expression","value":{"kind":"call","name":"readChecked","arguments":[{"kind":"parameter","name":"key"}]}});
        } else {
            let r = named(&mut v, "ensure");
            r["return_value"] = match mode {
                "direct" => {
                    json!({"kind":"map_lookup","field":"entries","index":0,"key":{"kind":"parameter","name":"key"}})
                }
                "expression" => {
                    json!({"kind":"expression","value":{"kind":"map_lookup","field":"entries","index":0,"key":{"kind":"parameter","name":"key"}}})
                }
                "non_map" => {
                    json!({"kind":"expression","value":{"kind":"field_literal","value":"42"}})
                }
                _ => unreachable!(),
            };
        }
        let c = contract(v);
        let cx = Context::new(&c);
        assert!(!cx.old(cx.circuits["ensure"]).unwrap());
        let text = current(&c, "ensure", "algebra.ordinary");
        if mode != "non_map" {
            assert!(text.find("missing key").unwrap() < text.find("record_lookup").unwrap());
        }
    }
    let c: Contract = serde_json::from_str(include_str!(
        "../../../../tests/effect-plan-maps-schema20-ir.json"
    ))
    .unwrap();
    assert!(current(&c, "direct_lookup", "algebra.ordinary").contains("record_lookup"));
    assert!(current(&c, "direct_member", "algebra.ordinary").contains("record_member"));
}
