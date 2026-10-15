// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;
#[test]
fn complete_old_sequence_includes_unit_and_discarded_field_on_same_or_distinct_sets() {
    for field_result in [false, true] {
        for same_set in [false, true] {
            let mut v = fixture();
            if field_result {
                named(&mut v, "witnesses", "set_local_id")["result"] = json!({"kind":"field"});
            }
            if same_set {
                root(&mut v)["actions"][1]["field"] = json!("eligible_participants");
                root(&mut v)["actions"][1]["index"] = json!(1);
            }
            let c = contract(v);
            included(&c, "check_in");
            let text = current(&c, "check_in");
            // A discarded Field still has a typed witness result in the generated
            // frame operation. Runtime tests qualify its exact private FAB.
            assert_eq!(text.matches("witnesses . set_local_id").count(), 1);
            let block: syn::Block = syn::parse_str(&format!("{{ {text} }}")).unwrap();
            let result_type = block
                .stmts
                .iter()
                .find_map(|statement| {
                    let syn::Stmt::Local(local) = statement else {
                        return None;
                    };
                    let syn::Expr::Try(call) = local.init.as_ref()?.expr.as_ref() else {
                        return None;
                    };
                    let syn::Expr::MethodCall(call) = call.expr.as_ref() else {
                        return None;
                    };
                    if call.method != "try_witness_metered" {
                        return None;
                    }
                    let syn::Pat::Type(pattern) = &local.pat else {
                        panic!("typed witness binding");
                    };
                    let syn::Type::Tuple(types) = pattern.ty.as_ref() else {
                        panic!("frame/result pair");
                    };
                    assert_eq!(types.elems.len(), 2);
                    types.elems.iter().nth(1)
                })
                .expect("one typed witness call");
            if field_result {
                let syn::Type::Path(ty) = result_type else {
                    panic!("Field path");
                };
                assert!(ty.qself.is_none());
                let path: Vec<_> = ty
                    .path
                    .segments
                    .iter()
                    .map(|s| s.ident.to_string())
                    .collect();
                assert_eq!(path, ["runtime", "Field"]);
            } else {
                assert!(matches!(result_type, syn::Type::Tuple(ty) if ty.elems.is_empty()));
            }
        }
    }
}
#[test]
fn names_layout_and_reachable_internal_roots_do_not_change_complete_old_authority() {
    for mode in ["raw_names", "chunked", "internal"] {
        let mut v = fixture();
        let public_name = match mode {
            "raw_names" => {
                rename(&mut v, "participant", "r#match");
                rename(&mut v, "eligible_participants", "r#type");
                rename(&mut v, "checked_in_participants", "r#loop");
                rename(&mut v, "set_local_id", "r#yield");
                rename(&mut v, "check_in", "arrive");
                "arrive"
            }
            "chunked" => {
                let fields = v["ledger_fields"].as_array_mut().unwrap();
                for (i, f) in fields.iter_mut().enumerate() {
                    f["index"] = json!(0);
                    f["path"] = json!([0, i]);
                }
                // Eighteen declarations have the canonical [3,15] chunk layout.
                for i in 0..15 {
                    fields.push(json!({"id":format!("padding{i}"),"index":1,"path":[1,i],"declaration":{"kind":"counter"}}));
                }
                edit(&mut v, &mut |n| {
                    if n.get("field").and_then(Value::as_str).is_some_and(|f| {
                        [
                            "organizer_pks",
                            "eligible_participants",
                            "checked_in_participants",
                        ]
                        .contains(&f)
                    }) && n.contains_key("index")
                    {
                        n.insert("index".into(), json!(0));
                    }
                });
                "check_in"
            }
            "internal" => {
                root(&mut v)["internal"] = json!(true);
                let mut wrapper = root(&mut v).clone();
                wrapper["name"] = json!("arrive");
                wrapper["internal"] = json!(false);
                wrapper["actions"] = json!([{"kind":"circuit_call","name":"check_in","arguments":[{"kind":"parameter","name":"participant"}]}]);
                v["stateful_circuits"].as_array_mut().unwrap().push(wrapper);
                "arrive"
            }
            _ => unreachable!(),
        };
        let c = contract(v);
        if mode == "internal" {
            let cx = Context::new(&c);
            assert!(cx.old(cx.circuits["check_in"]).unwrap());
            assert!(cx.checked(cx.circuits["check_in"]).is_ok());
            current(&c, public_name);
        } else {
            included(&c, public_name);
        }
    }
}
#[test]
fn equivalent_argument_and_ordered_composition_are_expansions_not_old_predicates() {
    for direct_argument in [false, true] {
        let mut v = fixture();
        if direct_argument {
            root(&mut v)["actions"][2]["value"]["arguments"][0] =
                json!({"kind":"parameter","name":"participant"});
        } else {
            root(&mut v)["actions"].as_array_mut().unwrap().insert(1,json!({"kind":"assert","condition":{"kind":"boolean","value":true},"message":"extra check before insertion"}));
        }
        let c = contract(v);
        let cx = Context::new(&c);
        assert!(!cx.old(cx.circuits["check_in"]).unwrap());
        let text = current(&c, "check_in");
        if !direct_argument {
            assert!(
                text.find("extra check before insertion").unwrap()
                    < text.find("record_insert").unwrap()
            );
        }
    }
}
