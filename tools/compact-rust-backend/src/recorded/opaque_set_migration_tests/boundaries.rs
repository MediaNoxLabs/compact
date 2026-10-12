// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;
#[test]
fn malformed_slot_declaration_and_lexical_authority_remain_refused() {
    for case in [
        "key_type",
        "root_result",
        "map_index",
        "set_index",
        "set_type",
        "missing_key",
        "helper_arity",
        "helper_result",
    ] {
        let mut v = fixture();
        match case {
            "key_type" => {
                named(&mut v, "setWatch")["parameters"][0]["ty"] = json!({"kind":"field"})
            }
            "root_result" => named(&mut v, "setWatch")["result"] = json!({"kind":"boolean"}),
            "map_index" => {
                named(&mut v, "recordExists")["return_value"]["value"]["condition"]["index"] =
                    json!(99)
            }
            "set_index" => actions(&mut v)[3]["then"]["actions"][1]["index"] = json!(99),
            "set_type" => {
                let field = v["ledger_fields"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|f| f["id"] == "watchList")
                    .unwrap();
                field["declaration"]["ty"] = json!({"kind":"field"});
            }
            "missing_key" => {
                actions(&mut v)[3]["then"]["actions"][1]["value"]["name"] = json!("escaped")
            }
            "helper_arity" => {
                named(&mut v, "assertWritable")["parameters"] =
                    json!([{"name":"extra","ty":{"kind":"field"}}])
            }
            "helper_result" => {
                named(&mut v, "assertWritable")["result"] = json!({"kind":"boolean"})
            }
            _ => unreachable!(),
        }
        let c = contract(v);
        let cx = Context::new(&c);
        assert!(!matches!(cx.old(), Ok(Some(_))), "complete old {case}");
        assert!(cx.checked().is_err(), "checked {case}");
        assert!(
            crate::render_with_capabilities(&c).is_err(),
            "native {case}"
        );
    }
}
#[test]
fn native_errors_precede_recorded_routing_in_former_fallible_guard_sites() {
    for (from, to) in [
        ("records", "bad first map"),
        ("custodyGrants", "bad second map"),
        ("watchList", "bad set"),
        ("ListMutation", "bad enum"),
        ("assertWritable", "bad helper"),
    ] {
        let mut v = fixture();
        rename(&mut v, &[(from, to)]);
        let c = contract(v);
        assert!(
            Context::new(&c).old().is_err(),
            "actual helper/old emitter error {from}"
        );
        let (result, rows) = trace::capture_json(|| crate::render_with_capabilities(&c));
        assert!(result.is_err(), "native {from}");
        assert!(
            rows.as_array().unwrap().is_empty(),
            "native validation precedes recorded routing {from}"
        );
    }
    // The old key-only Map check alone imposes no scalar value-codec rule.
    // Complete native rendering rejects this ledger-container value instead.
    let mut v = fixture();
    for f in v["ledger_fields"].as_array_mut().unwrap() {
        if f["id"] == "records" || f["id"] == "custodyGrants" {
            f["declaration"]["value"] =
                json!({"kind":"ledger_map","key":{"kind":"field"},"value":{"kind":"field"}});
        }
    }
    let c = contract(v);
    assert!(
        crate::render_with_capabilities(&c).is_err(),
        "nested ledger value is not counted as source-valid inclusion"
    );
}
#[test]
fn complete_helper_qualification_excludes_unsupported_children_and_recursion() {
    let mut v = fixture();
    named(&mut v,"assertWritable")["actions"].as_array_mut().unwrap().push(json!({
        "kind":"if","condition":{"kind":"boolean","value":true},
        "then":{"kind":"sequence","actions":[]},
        "otherwise":{"kind":"expression","value":{"kind":"persistent_commit","value":(serde_json::to_value(crate::recorded::test_support::unsupported_fixed_input()).unwrap()),"opening":{"kind":"bytes_literal","bytes":vec![0u8;32]}}}
    }));
    let c = crate::recorded::test_support::with_unsupported_fold(contract(v));
    let cx = Context::new(&c);
    let names = cx.discovered().unwrap();
    assert!(!names.contains("assertWritable"));
    assert!(
        cx.old().unwrap().is_none(),
        "unsupported helper is not a whole-old positive"
    );
    assert!(
        cx.checked().is_err(),
        "unselected helper effect must be checked"
    );
    let output = crate::render_with_capabilities(&c).unwrap();
    let root = output
        .capabilities
        .circuits
        .iter()
        .find(|c| c.name == "setWatch")
        .unwrap();
    assert!(!root.recorded && !root.observed_call);

    let mut v = fixture();
    named(&mut v, "assertWritable")["actions"] =
        json!([{"kind":"circuit_call","name":"assertWritable","arguments":[]}]);
    let c = contract(v);
    let cx = Context::new(&c);
    assert!(
        !matches!(cx.old(), Ok(Some(_))),
        "recursive discovery cannot forge shared authority"
    );
    assert!(cx.checked().is_err());
    assert!(
        crate::render_with_capabilities(&c).is_err(),
        "recursive native source is invalid"
    );
}
