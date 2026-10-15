// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;

#[test]
fn state_action_operands_and_control_children_are_complete() {
    for kind in [
        "cell_write",
        "set_insert",
        "set_remove",
        "list_push_front",
        "historic_merkle_insert",
        "merkle_insert",
    ] {
        assert_targets(
            &root(
                vec![json!({"kind":kind, "field":"f", "index":0, "value":call("value")})],
                json!({"kind":"unit"}),
            ),
            &["value"],
        );
    }
    for kind in ["map_insert_default", "map_remove"] {
        assert_targets(
            &root(
                vec![json!({"kind":kind, "field":"f", "index":0, "key":call("key")})],
                json!({"kind":"unit"}),
            ),
            &["key"],
        );
    }
    for kind in ["historic_merkle_insert_hash", "merkle_insert_hash"] {
        assert_targets(
            &root(
                vec![json!({"kind":kind, "field":"f", "index":0, "hash":call("hash")})],
                json!({"kind":"unit"}),
            ),
            &["hash"],
        );
    }
    for kind in [
        "historic_merkle_insert_index_default",
        "merkle_insert_index_default",
    ] {
        assert_targets(
            &root(
                vec![json!({"kind":kind, "field":"f", "index":0, "position":call("position")})],
                json!({"kind":"unit"}),
            ),
            &["position"],
        );
    }
    for kind in ["historic_merkle_insert_index", "merkle_insert_index"] {
        assert_targets(
            &root(
                vec![
                    json!({"kind":kind, "field":"f", "index":0, "value":call("value"), "position":call("position")}),
                ],
                json!({"kind":"unit"}),
            ),
            &["value", "position"],
        );
    }
    for kind in [
        "historic_merkle_insert_hash_index",
        "merkle_insert_hash_index",
    ] {
        assert_targets(
            &root(
                vec![
                    json!({"kind":kind, "field":"f", "index":0, "hash":call("hash"), "position":call("position")}),
                ],
                json!({"kind":"unit"}),
            ),
            &["hash", "position"],
        );
    }
    for kind in ["cell_write_coin", "set_insert_coin"] {
        assert_targets(
            &root(
                vec![
                    json!({"kind":kind, "field":"f", "index":0, "coin":call("coin"), "recipient":call("recipient")}),
                ],
                json!({"kind":"unit"}),
            ),
            &["coin", "recipient"],
        );
    }
    let rows = [
        (
            json!({"kind":"sequence", "actions":[{"kind":"circuit_call", "name":"a", "arguments":[]}, {"kind":"circuit_call", "name":"b", "arguments":[]}]}),
            vec!["a", "b"],
        ),
        (
            json!({"kind":"if", "condition":call("condition"), "then":{"kind":"circuit_call", "name":"then", "arguments":[]}, "otherwise":{"kind":"circuit_call", "name":"otherwise", "arguments":[]}}),
            vec!["condition", "then", "otherwise"],
        ),
        (
            json!({"kind":"expression", "value":call("value")}),
            vec!["value"],
        ),
        (
            json!({"kind":"assert", "condition":call("condition"), "message":"ok"}),
            vec!["condition"],
        ),
        (
            json!({"kind":"let", "bindings":[{"name":"x", "ty":{"kind":"boolean"}, "value":call("binding")}], "action":{"kind":"circuit_call", "name":"body", "arguments":[]}}),
            vec!["binding", "body"],
        ),
        (
            json!({"kind":"map_insert", "field":"f", "index":0, "key":call("key"), "value":call("value")}),
            vec!["key", "value"],
        ),
    ];
    for (action, expected) in rows {
        assert_targets(&root(vec![action], json!({"kind":"unit"})), &expected);
    }
}

#[test]
fn ordinary_return_operands_are_discovered() {
    let rows = [
        json!({"kind":"expression", "value":call("target")}),
        json!({"kind":"set_member", "field":"f", "index":0, "value":call("target")}),
        json!({"kind":"map_member", "field":"f", "index":0, "key":call("target")}),
        json!({"kind":"map_lookup", "field":"f", "index":0, "key":call("target")}),
        json!({"kind":"historic_merkle_check_root", "field":"f", "index":0, "root":call("target")}),
        json!({"kind":"merkle_check_root", "field":"f", "index":0, "root":call("target")}),
    ];
    for return_value in rows {
        assert_targets(&root(vec![], return_value), &["target"]);
    }
}

#[test]
fn every_effectful_return_plan_child_is_discovered() {
    let circuit = root(
        vec![],
        json!({"kind":"effectful", "body":{
            "kind":"sequence", "actions":[{
                "kind":"circuit_call", "name":"action", "arguments":[call("argument")]
            }], "result":{
                "kind":"let", "bindings":[{"name":"x", "ty":{"kind":"boolean"}, "value":call("binding")}],
                "result":{
                    "kind":"conditional", "condition":call("condition"),
                    "then":{"kind":"value", "value":call("then")},
                    "otherwise":{"kind":"value", "value":call("otherwise")}
                }
            }
        }}),
    );
    assert_targets(
        &circuit,
        &[
            "action",
            "argument",
            "binding",
            "condition",
            "then",
            "otherwise",
        ],
    );
}

#[test]
fn names_in_non_call_carriers_are_not_targets() {
    let circuit = root(
        vec![
            json!({"kind":"counter_increment", "field":"field_name", "index":0, "amount":{"kind":"parameter", "name":"parameter_name"}}),
        ],
        json!({"kind":"cell_read", "field":"field_name", "index":0}),
    );
    assert_targets(&circuit, &[]);
}
