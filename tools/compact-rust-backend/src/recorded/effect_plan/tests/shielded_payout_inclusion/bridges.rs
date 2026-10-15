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
        "withdraw",
        vec![binding(
            "saved_text",
            json!({"kind":"opaque_string"}),
            call("empty_text", vec![]),
        )],
    );
    included(c, "withdraw", "pure-owned-default");

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
        "concede",
        vec![binding(
            "pure_crypto",
            json!({"kind":"field"}),
            call("nested_crypto", vec![]),
        )],
    );
    included(c, "concede", "pure-nested-crypto");
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
        "withdraw",
        vec![binding("folded", field, call("fold_values", vec![initial]))],
    );
    included(c, "withdraw", "pure-vector-fold");
}
