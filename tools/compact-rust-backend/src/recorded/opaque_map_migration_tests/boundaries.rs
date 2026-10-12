// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;
#[test]
fn map_write_declaration_argument_and_class_authority_remain_independent() {
    for mode in [
        "key_type",
        "key_source",
        "value_binding",
        "insert_index",
        "insert_source",
        "helper_arity",
        "class_index",
        "class_arity",
        "counter_index",
        "exists_argument",
    ] {
        let mut v = isolated("setRecord");
        match mode {
            "key_type" => {
                named(&mut v, "setRecord")["parameters"][0]["ty"] = json!({"kind":"boolean"})
            }
            "key_source" => {
                named(&mut v, "setRecord")["actions"][0]["bindings"][0]["value"]["name"] =
                    json!("record")
            }
            "value_binding" => {
                named(&mut v, "setRecord")["actions"][0]["action"]["bindings"][0]["ty"] =
                    json!({"kind":"boolean"})
            }
            "insert_index" => insertion(&mut v, "setRecord")["index"] = json!(99),
            "insert_source" => insertion(&mut v, "setRecord")["value"]["name"] = json!("escaped"),
            "helper_arity" => {
                actions(&mut v, "setRecord")[0]["arguments"] =
                    json!([{"kind":"boolean","value":true}])
            }
            "class_index" => {
                v["circuits"][0]["body"]["steps"][0]["condition"]["left"]["index"] = json!(99)
            }
            "class_arity" => actions(&mut v, "setRecord")[2]["arguments"] = json!([]),
            "counter_index" => {
                branch(&mut v, "setRecord")["otherwise"]["then"]["actions"][1]["action"]["index"] =
                    json!(99)
            }
            "exists_argument" => {
                branch(&mut v, "setRecord")["otherwise"]["then"]["actions"][0]["condition"]["condition"]
                    ["arguments"][0]["value"]["name"] = json!("escaped")
            }
            _ => unreachable!(),
        }
        let c = contract(v);
        let cx = Context::new(&c, "setRecord");
        assert!(!matches!(cx.old(), Ok(Some(_))), "old authority {mode}");
        assert!(cx.checked().is_err(), "checked {mode}");
        assert!(
            crate::render_with_capabilities(&c).is_err(),
            "native {mode}"
        );
    }
}
#[test]
fn unsupported_or_recursive_helpers_never_count_as_complete_old_admission() {
    let mut v = fixture();
    named(&mut v, "recordWrite")["actions"] = json!([{"kind":"expression","value":{"kind":"persistent_commit","value":{"kind":"field_literal","value":"0"},"opening":{"kind":"bytes_literal","bytes":vec![0u8;32]}}}]);
    let c = contract(v);
    for name in ROOTS {
        let cx = Context::new(&c, name);
        assert!(!cx.discovered().unwrap().contains("recordWrite"));
        assert!(cx.old().unwrap().is_none());
        assert!(cx.checked().is_err());
    }
    let native = crate::render_with_capabilities(&c).unwrap();
    for name in ROOTS {
        let row = native
            .capabilities
            .circuits
            .iter()
            .find(|r| r.name == name)
            .unwrap();
        assert!(!row.recorded && !row.observed_call);
    }
    let mut v = fixture();
    named(&mut v, "recordWrite")["actions"] =
        json!([{"kind":"circuit_call","name":"recordWrite","arguments":[]}]);
    let c = contract(v);
    let (result, rows) = trace::capture_json(|| crate::render_with_capabilities(&c));
    assert!(result.is_err());
    assert!(
        rows.as_array().unwrap().is_empty(),
        "native cycle refusal precedes helper discovery"
    );
}
#[test]
fn native_errors_precede_retired_guard_and_remaining_recording_routes() {
    for (from, to) in [
        ("records", "bad map"),
        ("assertWritable", "bad helper"),
        ("currentTimestamp", "bad witness"),
        ("assertRecordClassKnown", "bad pure"),
        ("AssetRecord", "bad struct"),
        ("kind", "bad member"),
    ] {
        let mut v = isolated("setRecord");
        rename(&mut v, from, to);
        let c = contract(v);
        let (result, rows) = trace::capture_json(|| crate::render_with_capabilities(&c));
        assert!(result.is_err(), "native {from}");
        assert!(
            rows.as_array().unwrap().is_empty(),
            "native precedence {from}"
        );
    }
}
