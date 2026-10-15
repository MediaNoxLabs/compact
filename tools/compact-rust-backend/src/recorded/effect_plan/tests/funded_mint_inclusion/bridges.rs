// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;
#[test]
fn pure_owned_defaults_and_nested_crypto_keep_the_complete_body_certificate() {
    let mut c = source();
    pure(
        &mut c,
        "empty_text",
        vec![],
        json!({"kind":"opaque_string"}),
        json!({"kind":"default","ty":{"kind":"opaque_string"}}),
    );
    prepend(
        &mut c,
        "buy_in",
        vec![binding(
            "saved_text",
            json!({"kind":"opaque_string"}),
            call("empty_text", vec![]),
        )],
    );
    included(c, "buy_in", "pure-owned-default");

    let mut c = source();
    pure(
        &mut c,
        "committed_fields",
        vec![],
        json!({"kind":"field"}),
        json!({"kind":"transient_commit","value":{
        "kind":"tuple","elements":[{"kind":"field_literal","value":"3"},{"kind":"field_literal","value":"7"}]
    },"opening":{"kind":"field_literal","value":"11"}}),
    );
    pure(
        &mut c,
        "nested_crypto",
        vec![],
        json!({"kind":"field"}),
        json!({"kind":"transient_hash","value":{
            "kind":"tuple","elements":[call("committed_fields",vec![]),{"kind":"degrade_to_transient","value":{
                "kind":"persistent_hash","value":{"kind":"tuple","elements":[{"kind":"bytes_literal","bytes":vec![1;32]},{"kind":"bytes_literal","bytes":vec![2;32]}]}
            }}]
        }}),
    );
    prepend(
        &mut c,
        "buy_in",
        vec![binding(
            "pure_crypto",
            json!({"kind":"field"}),
            call("nested_crypto", vec![]),
        )],
    );
    included(c, "buy_in", "pure-nested-crypto");
}
#[test]
fn pure_fold_is_not_confused_with_direct_expression_grammar() {
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
    let initial = json!({"kind":"coerce","ty":vector,"value":{"kind":"tuple","elements":[{"kind":"field_literal","value":"5"},{"kind":"field_literal","value":"7"}]}});
    prepend(
        &mut c,
        "buy_in",
        vec![binding("folded", field, call("fold_values", vec![initial]))],
    );
    included(c, "buy_in", "pure-vector-fold");
}
#[test]
fn funded_specific_pure_commit_upgrade_and_field_cast_are_not_dropped() {
    for (mode, label) in [
        (0, "pure-persistent-commit"),
        (1, "pure-upgrade-field-cast"),
        (2, "pure-owned-product-default"),
    ] {
        let mut c = source();
        let (parameters, result, body, arguments) = match mode {
            0 => (
                vec![],
                json!({"kind":"bytes","length":32}),
                json!({"kind":"persistent_commit","value":{"kind":"bytes_literal","bytes":vec![3;32]},"opening":{"kind":"bytes_literal","bytes":vec![5;32]}}),
                vec![],
            ),
            1 => (
                vec![json!({"name":"input","ty":{"kind":"unsigned","max":u64::MAX.to_string()}})],
                json!({"kind":"bytes","length":32}),
                json!({"kind":"upgrade_from_transient","value":{"kind":"field_cast","value":reference("input")}}),
                vec![reference("amount")],
            ),
            2 => {
                let product = json!({"kind":"struct","name":"OwnedMemo","fields":[{"name":"text","ty":{"kind":"opaque_string"}},{"name":"bytes","ty":{"kind":"opaque_bytes"}},{"name":"pair","ty":{"kind":"tuple","elements":[{"kind":"opaque_string"},{"kind":"opaque_bytes"}]}}]});
                (
                    vec![],
                    product.clone(),
                    json!({"kind":"default","ty":product}),
                    vec![],
                )
            }
            _ => unreachable!(),
        };
        pure(&mut c, "extra_pure", parameters, result.clone(), body);
        prepend(
            &mut c,
            "buy_in",
            vec![binding("pure_extra", result, call("extra_pure", arguments))],
        );
        included(c, "buy_in", label);
    }
}
