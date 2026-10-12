// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;

#[test]
fn malformed_declaration_and_slot_controls_remain_refused() {
    for case in [
        "formal",
        "result",
        "member",
        "optional_index",
        "write_index",
        "write_slot",
        "binding_type",
        "write_local",
    ] {
        let mut v = fixture();
        match case {
            "formal" => {
                named_mut(&mut v, "circuits", "successor")["parameters"][0]["ty"] =
                    json!({"kind":"boolean"})
            }
            "result" => {
                named_mut(&mut v, "circuits", "successor")["result"] = json!({"kind":"boolean"})
            }
            "member" => actions(&mut v)[1]["condition"]["field"] = json!("missing"),
            "optional_index" => actions(&mut v)[1]["condition"]["value"]["index"] = json!(99),
            "write_index" => actions(&mut v)[2]["action"]["index"] = json!(99),
            "write_slot" => actions(&mut v)[2]["action"]["field"] = json!("authority"),
            "binding_type" => actions(&mut v)[2]["bindings"][0]["ty"] = json!({"kind":"boolean"}),
            "write_local" => actions(&mut v)[2]["action"]["value"]["name"] = json!("missing"),
            _ => unreachable!(),
        }
        let c = contract(v);
        assert!(old(&c).unwrap().is_none(), "whole old {case}");
        assert!(checked(&c).is_err(), "checked {case}");
        assert!(
            crate::render_with_capabilities(&c).is_err(),
            "native {case}"
        );
    }
}

#[test]
fn old_fallible_sites_and_unchecked_variant_are_native_owned() {
    for (from, to) in [
        ("topic", "bad optional"),
        ("state", "bad phase"),
        ("is_some", "bad member"),
        ("successor", "bad successor"),
        ("PublicState", "bad enum"),
        ("Maybe", "bad struct"),
    ] {
        let mut v = fixture();
        rename(&mut v, &[(from, to)]);
        let c = contract(v);
        assert!(old(&c).is_err(), "old fallible {from}");
        assert_native_error_before_recording(&c);
    }
    let mut v = fixture();
    let helper = named_mut(&mut v, "circuits", "successor");
    helper["body"] = json!({"kind":"enum_variant","ty":helper["result"],"variant":"Missing"});
    let c = contract(v);
    // The old grammar checks type equality but not membership of the variant.
    assert!(
        old(&c).unwrap().is_some(),
        "old syntactic acceptance is not source validity"
    );
    assert!(checked(&c).is_err());
    assert_native_error_before_recording(&c);
}
fn assert_native_error_before_recording(c: &Contract) {
    let (result, rows) = trace::capture_json(|| crate::render_with_capabilities(c));
    assert!(result.is_err());
    assert!(rows.as_array().unwrap().is_empty());
}

#[test]
fn interface_and_helper_boundaries_are_not_replaced_with_enum_tree_recognition() {
    let mut v = fixture();
    named_mut(&mut v, "circuits", "successor")["internal"] = json!(false);
    let c = contract(v);
    assert!(old(&c).unwrap().is_none());
    assert!(
        checked(&c).is_ok(),
        "public pure helper is already a supported declaration"
    );
    assert_actual_owner(&c);

    let mut v = fixture();
    named_mut(&mut v, "stateful_circuits", "advance")["parameters"] =
        json!([{"name":"extra","ty":{"kind":"boolean"}}]);
    let c = contract(v);
    assert!(old(&c).unwrap().is_none());
    assert!(
        checked(&c).is_ok(),
        "existing membership interface owns Boolean parameters"
    );
    assert_actual_owner(&c);

    let mut v = fixture();
    let helper = named_mut(&mut v, "circuits", "successor");
    let body = helper["body"].take();
    helper["body"] = json!({"kind":"sequence","steps":[{"kind":"assert","condition":{"kind":"struct_field","value":{"kind":"cell_read","field":"topic","index":2},"field":"is_some","index":0},"message":"impure presence"}],"value":body});
    let c = contract(v);
    assert!(old(&c).unwrap().is_none());
    assert_native_error_before_recording(&c);

    let mut v = fixture();
    let helper = named_mut(&mut v, "circuits", "successor");
    let body = helper["body"].take();
    // A pure sequence step must be Unit. Compare the unsupported commitment
    // inside an assertion so this control remains native-valid.
    let rejected_branch = json!({"kind":"sequence","steps":[{"kind":"assert","condition":{"kind":"equal","left":{"kind":"persistent_commit","value":{"kind":"field_literal","value":"0"},"opening":{"kind":"bytes_literal","bytes":vec![0u8;32]}},"right":{"kind":"bytes_literal","bytes":vec![0u8;32]}},"message":"hidden commitment"}],"value":body});
    helper["body"] = json!({"kind":"if","condition":{"kind":"boolean","value":true},"then":body,"otherwise":rejected_branch});
    let c = contract(v);
    assert!(old(&c).unwrap().is_none());
    // Certified pure calls preserve native context-free operations. Their domain
    // is deliberately broader than direct effect-plan expression lowering.
    let plan = checked(&c).unwrap();
    let steps = &plan.steps;
    assert!(quote::quote!(#(#steps)*).to_string().contains("successor"));
    assert_actual_owner(&c);
    let rendered = crate::render_with_capabilities(&c).unwrap();
    assert!(rendered.source.contains("hidden commitment"));
    assert!(rendered.source.contains("persistent_commit"));

    // In contrast, a direct unsupported effect child cannot hide in an
    // unselected action branch. This is independently native-valid.
    let mut v = fixture();
    actions(&mut v).as_array_mut().unwrap().push(json!({
        "kind":"if", "condition":{"kind":"boolean","value":true},
        "then":{"kind":"sequence","actions":[]},
        "otherwise":{"kind":"expression","value":{"kind":"persistent_commit",
            "value":(serde_json::to_value(crate::recorded::test_support::unsupported_fixed_input()).unwrap()),
            "opening":{"kind":"bytes_literal","bytes":vec![0u8;32]}}}
    }));
    let c = crate::recorded::test_support::with_unsupported_fold(contract(v));
    assert!(old(&c).unwrap().is_none());
    assert!(
        checked(&c).is_err(),
        "unselected effect children are checked"
    );
    let rendered = crate::render_with_capabilities(&c).unwrap();
    let capability = rendered
        .capabilities
        .circuits
        .iter()
        .find(|c| c.name == "advance")
        .unwrap();
    assert!(!capability.recorded && !capability.observed_call);
}
