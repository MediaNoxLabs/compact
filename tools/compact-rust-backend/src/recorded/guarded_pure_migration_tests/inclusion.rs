// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;

#[test]
fn complete_source_guards_keep_earlier_checked_owner_and_execution_order() {
    assert_old_positive(
        &contract(fixture()),
        "recordFreshEnough",
        "record_increment",
    );
    let asset: Contract = serde_json::from_str(include_str!(
        "../../../tests/asset-registry-schema20-ir.json"
    ))
    .unwrap();
    assert_old_positive(&asset, "acceptIfFresh", "record_increment");
}

#[test]
fn canonical_products_raw_names_and_parameter_authority_remain_distinct() {
    for case in ["nested_owned", "raw_names", "shadowing"] {
        let mut v = fixture();
        match case {
            "nested_owned" => edit(&mut v, &mut |m| {
                if m.get("kind").and_then(Value::as_str) == Some("struct")
                    && m.get("name").and_then(Value::as_str) == Some("VerifierPolicy")
                {
                    m.get_mut("fields").unwrap().as_array_mut().unwrap().push(json!({"name":"label","ty":{"kind":"struct","name":"PolicyLabel","fields":[{"name":"text","ty":{"kind":"opaque_string"}}]}}));
                }
            }),
            "raw_names" => rename(
                &mut v,
                &[
                    ("policy", "r#type"),
                    ("assertFreshEnough", "r#match"),
                    ("accepted", "r#loop"),
                ],
            ),
            "shadowing" => rename(pure(&mut v), &[("t_19", "currentTime")]),
            _ => unreachable!(),
        }
        assert_old_positive(&contract(v), "recordFreshEnough", "record_increment");
    }
    for case in ["arity", "formal", "coercion", "argument_order"] {
        let mut v = fixture();
        match case {
            "arity" => {
                root(&mut v)["actions"][0]["arguments"]
                    .as_array_mut()
                    .unwrap()
                    .pop();
            }
            "formal" => pure(&mut v)["parameters"][0]["ty"] = json!({"kind":"boolean"}),
            "coercion" => {
                root(&mut v)["actions"][0]["arguments"][0]["ty"] = json!({"kind":"boolean"})
            }
            "argument_order" => root(&mut v)["actions"][0]["arguments"]
                .as_array_mut()
                .unwrap()
                .swap(0, 1),
            _ => unreachable!(),
        }
        let c = contract(v);
        assert!(
            old_prefix(&c, "recordFreshEnough").unwrap().is_none(),
            "{case}"
        );
        assert!(checked(&c, "recordFreshEnough").is_err(), "{case}");
        assert!(crate::render_with_capabilities(&c).is_err(), "{case}");
    }
}

#[test]
fn certified_pure_bodies_preserve_native_authority_beyond_outer_shape() {
    for case in ["boolean", "comparison", "nested_projection"] {
        let mut v = fixture();
        match case {
            "boolean" => {
                pure(&mut v)["body"]["steps"][0]["condition"] =
                    json!({"kind":"boolean","value":true})
            }
            "comparison" => {
                pure(&mut v)["body"]["steps"][0]["condition"]["operator"] = json!("greater")
            }
            "nested_projection" => {
                pure(&mut v)["body"]["steps"][0]["condition"]["right"]["field"] = json!("issuer")
            }
            _ => unreachable!(),
        }
        if case == "nested_projection" {
            pure(&mut v)["body"]["steps"][0]["condition"]["right"]["index"] = json!(1);
        }
        assert_old_positive(&contract(v), "recordFreshEnough", "record_increment");
    }
    let mut v = fixture();
    pure(&mut v)["body"]["steps"].as_array_mut().unwrap().push(json!({"kind":"assert","condition":{"kind":"boolean","value":true},"message":"additional pure assertion"}));
    let c = contract(v);
    assert!(old_prefix(&c, "recordFreshEnough").unwrap().is_none());
    let text = assert_current(&c, "recordFreshEnough");
    assert!(text.find("pure_circuits").unwrap() < text.find("record_increment").unwrap());
}

#[test]
fn whole_helpers_and_empty_private_execution_are_qualified_beyond_prefix() {
    for transitive in [false, true] {
        let mut v = fixture();
        let continuation = root(&mut v)["actions"][1].clone();
        helper(&mut v, "finish", json!([continuation]));
        if transitive {
            helper(&mut v, "forward", json!([call("finish")]));
        }
        root(&mut v)["actions"][1] = call(if transitive { "forward" } else { "finish" });
        assert_old_positive(&contract(v), "recordFreshEnough", "record_increment");
    }
    let synthetic: Contract = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests-rust-backend/empty-recording-ir/contract-ir.json"
    )))
    .unwrap();
    assert_old_positive(&synthetic, "recordFreshEnough", "pure continuation");
    let genuine: Contract = serde_json::from_str(include_str!(
        "../../../tests/empty-witness-guard-schema20-ir.json"
    ))
    .unwrap();
    assert_old_positive(&genuine, "recordFreshEnough", "try_witness_metered");
    let mut v = fixture();
    helper(
        &mut v,
        "finish",
        json!([{"kind":"expression","value":{"kind":"persistent_commit","value":(serde_json::to_value(crate::recorded::test_support::unsupported_fixed_input()).unwrap()),"opening":{"kind":"bytes_literal","bytes":vec![0u8;32]}}}]),
    );
    root(&mut v)["actions"][1] = call("finish");
    let c = contract(v);
    assert!(old_prefix(&c, "recordFreshEnough").unwrap().is_some());
    assert!(
        !old_complete(&c, "recordFreshEnough").unwrap(),
        "unsupported continuation is not an old positive"
    );
    assert!(checked(&c, "recordFreshEnough").is_err());
    let output = crate::render_with_capabilities(&c).unwrap();
    assert!(
        output
            .capabilities
            .circuits
            .iter()
            .any(|r| r.name == "recordFreshEnough" && !r.recorded && !r.observed_call)
    );
}
