// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;
#[test]
fn whole_source_preserves_guards_mutations_and_witness_order() {
    let text = assert_old_positive(&contract(fixture()), "genuine whole019");
    assert!(text.find("registry is closed").unwrap() < text.find("record does not exist").unwrap());
    assert!(text.find("record_insert").unwrap() < text.rfind("record_increment").unwrap());
    assert!(text.rfind("record_increment").unwrap() < text.find("try_witness_metered").unwrap());
}
#[test]
fn lexical_keys_and_canonical_chunked_slot_permutations_preserve_old_admission() {
    for mode in ["shadow", "raw_names", "permuted_layout"] {
        let mut v = fixture();
        match mode {
            "shadow" => rename(named(&mut v, ROOT), "disclosedId", "recordId"),
            "raw_names" => {
                rename(named(&mut v, ROOT), "recordId", "r#type");
                rename(named(&mut v, ROOT), "disclosedId", "r#match");
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
        assert_old_positive(&contract(v), mode);
    }
}
#[test]
fn complete_helper_discovery_distinguishes_supported_and_unsupported_bodies() {
    for mode in ["transitive", "pure_writable", "pure_write"] {
        let mut v = fixture();
        if mode == "transitive" {
            let mut helper = named(&mut v, "assertWritable").clone();
            helper["name"] = json!("checkWritable");
            v["stateful_circuits"].as_array_mut().unwrap().push(helper);
            named(&mut v, "assertWritable")["actions"] =
                json!([{"kind":"circuit_call","name":"checkWritable","arguments":[]}]);
        } else {
            let name = if mode == "pure_writable" {
                "assertWritable"
            } else {
                "recordWrite"
            };
            named(&mut v, name)["actions"] = json!([{"kind":"assert","condition":{"kind":"boolean","value":true},"message":"pure helper"}]);
        }
        assert_old_positive(&contract(v), mode);
    }
    let mut v = isolated();
    named(&mut v, "recordWrite")["actions"] = json!([{"kind":"expression","value":{"kind":"persistent_commit","value":(serde_json::to_value(crate::recorded::test_support::unsupported_fixed_input()).unwrap()),"opening":{"kind":"bytes_literal","bytes":vec![0u8;32]}}}]);
    let c = contract(v);
    let cx = Context::new(&c);
    assert!(cx.prefix(), "the permit alone ignores helper bodies");
    assert!(!cx.old_complete().unwrap());
    assert!(cx.checked().is_err());
    let output = crate::render_with_capabilities(&c).unwrap();
    assert!(
        output
            .capabilities
            .circuits
            .iter()
            .any(|r| r.name == ROOT && !r.recorded && !r.observed_call)
    );
}
#[test]
fn key_only_maps_do_not_require_scalar_value_codecs_or_witness_view_support() {
    for (label, ty) in [
        (
            "product",
            json!({"kind":"tuple","elements":[{"kind":"field"},{"kind":"opaque_bytes"}]}),
        ),
        (
            "nested_map",
            json!({"kind":"ledger_map","key":{"kind":"field"},"value":{"kind":"field"}}),
        ),
    ] {
        let mut v = isolated();
        v["ledger_fields"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|f| f["id"] == "records")
            .unwrap()["declaration"]["value"] = ty;
        if label == "nested_map" {
            assert!(
                matches!(crate::render_with_capabilities(&contract(v.clone())), Err(RenderError::Located { error, .. }) if matches!(*error, RenderError::UnsupportedLedgerValueType(Type::LedgerMap { .. })))
            );
            // Separate complete witness-free input: retain both write counters.
            // The original witness view cannot represent this Map's value.
            v["witnesses"] = json!([]);
            named(&mut v, "recordWrite")["actions"]
                .as_array_mut()
                .unwrap()
                .truncate(2);
        }
        assert_old_positive(&contract(v), label);
    }
}
