// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;

#[test]
fn expression_operands_are_visited_across_every_carrier_family() {
    // Each operand has a distinct target: finding one sibling cannot mask an
    // omitted child. These are syntactic facts, intentionally not typed programs.
    for kind in [
        "field_cast",
        "field_to_bytes32",
        "transient_hash",
        "persistent_hash",
        "keccak256",
        "degrade_to_transient",
        "upgrade_from_transient",
        "hash_to_curve",
        "jubjub_point_x",
        "jubjub_point_y",
        "ec_neg",
        "jubjub_scalar_from_native",
    ] {
        assert_targets(
            &expression(json!({"kind":kind, "value":call("value")})),
            &["value"],
        );
    }
    for kind in [
        "equal",
        "not_equal",
        "ec_add",
        "add",
        "subtract",
        "multiply",
    ] {
        assert_targets(
            &expression(json!({"kind":kind, "left":call("left"), "right":call("right")})),
            &["left", "right"],
        );
    }
    for kind in ["unsigned_add", "unsigned_subtract", "unsigned_multiply"] {
        assert_targets(
            &expression(
                json!({"kind":kind, "max":"255", "left":call("left"), "right":call("right")}),
            ),
            &["left", "right"],
        );
    }
    for kind in ["transient_commit", "persistent_commit"] {
        assert_targets(
            &expression(json!({"kind":kind, "value":call("value"), "opening":call("opening")})),
            &["value", "opening"],
        );
    }
    for kind in ["map_member", "map_lookup"] {
        assert_targets(
            &expression(json!({"kind":kind, "field":"m", "index":0, "key":call("key")})),
            &["key"],
        );
    }
    for kind in ["merkle_check_root", "historic_merkle_check_root"] {
        assert_targets(
            &expression(json!({"kind":kind, "field":"m", "index":0, "root":call("root")})),
            &["root"],
        );
    }
    let rows = [
        (
            json!({"kind":"unsigned_cast", "max":"255", "value":call("value")}),
            vec!["value"],
        ),
        (
            json!({"kind":"coerce", "ty":{"kind":"field"}, "value":call("value")}),
            vec!["value"],
        ),
        (
            json!({"kind":"struct_field", "field":"x", "index":0, "value":call("value")}),
            vec!["value"],
        ),
        (
            json!({"kind":"tuple_index", "index":0, "value":call("value")}),
            vec!["value"],
        ),
        (
            json!({"kind":"struct_literal", "ty":{"kind":"struct", "name":"S", "fields":[]}, "fields":[call("a"),call("b")]}),
            vec!["a", "b"],
        ),
        (
            json!({"kind":"set_member", "field":"s", "index":0, "value":call("value")}),
            vec!["value"],
        ),
        (
            json!({"kind":"counter_less_than", "field":"c", "index":0, "threshold":call("threshold")}),
            vec!["threshold"],
        ),
        (
            json!({"kind":"tuple", "elements":[call("a"),call("b")]}),
            vec!["a", "b"],
        ),
        (
            json!({"kind":"vector", "element":{"kind":"field"}, "elements":[call("a"),call("b")]}),
            vec!["a", "b"],
        ),
        (
            json!({"kind":"vector_map", "parameter":{"name":"x", "ty":{"kind":"field"}}, "source":call("source"), "body":call("body"), "result":{"kind":"field"}, "length":1}),
            vec!["source", "body"],
        ),
        (
            json!({"kind":"if", "condition":call("condition"), "then":call("then"), "otherwise":call("otherwise")}),
            vec!["condition", "then", "otherwise"],
        ),
        (
            json!({"kind":"let", "bindings":[{"name":"x", "ty":{"kind":"boolean"}, "value":call("binding")}], "body":call("body")}),
            vec!["binding", "body"],
        ),
        (
            json!({"kind":"sequence", "steps":[call("a"),call("b")], "value":call("value")}),
            vec!["a", "b", "value"],
        ),
        (
            json!({"kind":"assert", "condition":call("condition"), "message":"ok"}),
            vec!["condition"],
        ),
        (
            json!({"kind":"compare", "operator":"less", "left":call("left"), "right":call("right")}),
            vec!["left", "right"],
        ),
        (
            json!({"kind":"call", "name":"outer", "arguments":[call("a"),call("b")]}),
            vec!["outer", "a", "b"],
        ),
        (
            json!({"kind":"construct_jubjub_point", "x":call("x"), "y":call("y")}),
            vec!["x", "y"],
        ),
        (
            json!({"kind":"ec_mul", "point":call("point"), "scalar":call("scalar")}),
            vec!["point", "scalar"],
        ),
        (
            json!({"kind":"ec_mul_generator", "scalar":call("scalar")}),
            vec!["scalar"],
        ),
        (
            json!({"kind":"kernel_claim", "claim":"nullifier", "value":call("value")}),
            vec!["value"],
        ),
        (
            json!({"kind":"kernel_mint_shielded", "domain":call("domain"), "amount":call("amount")}),
            vec!["domain", "amount"],
        ),
        (
            json!({"kind":"create_zswap_input", "coin":call("coin")}),
            vec!["coin"],
        ),
        (
            json!({"kind":"create_zswap_output", "coin":call("coin"), "recipient":call("recipient")}),
            vec!["coin", "recipient"],
        ),
    ];
    for (value, expected) in rows {
        assert_targets(&expression(value), &expected);
    }
}
