// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Pure bodies use their certified native bridge, not the direct reset visitor.
use super::*;

#[test]
fn owned_pure_values_and_stateful_identity_reuse_remain_native_valid() {
    let mut c = source();
    pure(
        &mut c,
        "empty_owned",
        vec![],
        json!({"kind":"opaque_string"}),
        json!({"kind":"default","ty":{"kind":"opaque_string"}}),
    );
    prepend(
        &mut c,
        vec![binding(
            "owned_text",
            json!({"kind":"opaque_string"}),
            call("empty_owned", vec![]),
        )],
    );
    included(c, "cash_out", "pure-owned-string");

    let mut c = source();
    let text = json!({"kind":"opaque_string"});
    let vector = json!({"kind":"vector","element":text,"length":2});
    pure(
        &mut c,
        "empty_owned",
        vec![],
        text.clone(),
        json!({"kind":"default","ty":text}),
    );
    c["stateful_circuits"].as_array_mut().unwrap().push(json!({
        "name":"owned_identity","internal":true,"parameters":[{"name":"item","ty":vector}],
        "result":vector,"actions":[],"return_value":{"kind":"expression","value":reference("item")}
    }));
    let initial = json!({"kind":"coerce","ty":vector,"value":{"kind":"tuple","elements":[call("empty_owned",vec![]),call("empty_owned",vec![])]}});
    prepend(
        &mut c,
        vec![
            binding(
                "owned_first",
                vector.clone(),
                call("owned_identity", vec![initial]),
            ),
            binding(
                "owned_second",
                vector.clone(),
                call("owned_identity", vec![reference("owned_first")]),
            ),
            binding(
                "owned_reused",
                vector,
                call("owned_identity", vec![reference("owned_first")]),
            ),
        ],
    );
    included(c, "cash_out", "owned-vector-helper-reuse");
}
#[test]
fn reset_pure_owned_optional_result_can_use_an_identity_chain() {
    let mut c = source();
    let declaration = c["circuits"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["name"] == "__compact_function_none_184")
        .unwrap()
        .clone();
    let ty = declaration["result"].clone();
    pure(
        &mut c,
        "optional_identity",
        vec![json!({"name":"value","ty":ty})],
        ty,
        reference("value"),
    );
    let helper = circuit(&mut c, "reset_state");
    assert!(first(helper, &mut |v| {
        if v["kind"] == "call" && v["name"] == "__compact_function_none_184" {
            *v = call("optional_identity", vec![v.clone()]);
            true
        } else {
            false
        }
    }));
    included(c, "cash_out", "reset-optional-pure-chain");
}
#[test]
fn fold_is_admitted_through_a_complete_pure_body() {
    let mut c = source();
    let field = json!({"kind":"field"});
    let vector = json!({"kind":"vector","element":field,"length":2});
    pure(
        &mut c,
        "keep_accumulator",
        vec![
            json!({"name":"acc","ty":field}),
            json!({"name":"item","ty":field}),
        ],
        field.clone(),
        reference("acc"),
    );
    pure(
        &mut c,
        "fold_values",
        vec![json!({"name":"items","ty":vector})],
        field.clone(),
        json!({
            "kind":"vector_fold_call","name":"keep_accumulator","initial":{"kind":"field_literal","value":"3"},
            "source":reference("items"),"accumulator":field,"element":field,"length":2
        }),
    );
    let vector_value = json!({"kind":"coerce","ty":vector,"value":{"kind":"tuple","elements":[{"kind":"field_literal","value":"7"},{"kind":"field_literal","value":"11"}]}});
    prepend(
        &mut c,
        vec![binding(
            "folded",
            field,
            call("fold_values", vec![vector_value]),
        )],
    );
    included(c, "cash_out", "pure-vector-fold");
}
