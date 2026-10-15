// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! The old root has no formal whitelist; native representation limits still apply.
use super::*;

fn tuple(length: usize) -> Json {
    let mut elements = vec![json!({"kind":"opaque_bytes"})];
    elements.extend((1..length).map(|_| json!({"kind":"field"})));
    json!({"kind":"tuple","elements":elements})
}
#[test]
fn native_encodable_public_formals_are_not_narrowed_to_fixture_signatures() {
    for (name, label, ty) in [
        (
            "withdraw",
            "public-owned-string",
            json!({"kind":"opaque_string"}),
        ),
        ("concede", "public-tuple11", tuple(11)),
        (
            "withdraw",
            "public-vector-tuple9",
            json!({"kind":"vector","element":tuple(9),"length":2}),
        ),
        (
            "concede",
            "public-wide-empty-product",
            json!({"kind":"struct","name":"PublicExtra","fields":[
                {"name":"wide","ty":{"kind":"unsigned","max":"680564733841876926926749214863536422911"}},
                {"name":"empty","ty":{"kind":"struct","name":"EmptyExtra","fields":[]}}
            ]}),
        ),
    ] {
        let mut c = source();
        circuit(&mut c, name)["parameters"]
            .as_array_mut()
            .unwrap()
            .push(json!({"name":"unused_owned","ty":ty}));
        included(c, name, label);
    }
}
#[test]
fn internal_local_interfaces_are_checked_directly_as_well_as_through_public_calls() {
    for (name, arity, label) in [
        ("withdraw", 12, "internal-tuple12"),
        ("concede", 13, "internal-tuple13"),
    ] {
        let mut c = source();
        let mut helper = circuit(&mut c, name).clone();
        helper["name"] = json!("internal_payout");
        helper["internal"] = json!(true);
        helper["parameters"] = json!([{"name":"unused_local","ty":tuple(arity)}]);
        c["stateful_circuits"].as_array_mut().unwrap().push(helper);
        let carrier = json!({"kind":"struct","name":"LocalBytesCarrier","fields":[{"name":"bytes","ty":{"kind":"opaque_bytes"}}]});
        let mut elements = vec![
            json!({"kind":"struct_field","value":{"kind":"default","ty":carrier},"field":"bytes","index":0}),
        ];
        elements.extend((1..arity).map(|n| json!({"kind":"field_literal","value":n.to_string()})));
        *root(&mut c, name) = call(
            "internal_payout",
            vec![json!({"kind":"tuple","elements":elements})],
        );
        included_with_internal(c, name, label, Some("internal_payout"));
    }
}
#[test]
fn owned_witness_arguments_can_be_reused_without_narrowing_the_old_result_domain() {
    let mut c = source();
    let ty = json!({"kind":"struct","name":"WitnessPayload","fields":[
        {"name":"text","ty":{"kind":"opaque_string"}},
        {"name":"bytes","ty":{"kind":"opaque_bytes"}},
        {"name":"pair","ty":{"kind":"tuple","elements":[{"kind":"opaque_string"},{"kind":"opaque_bytes"}]}}
    ]});
    pure(
        &mut c,
        "empty_payload",
        vec![],
        ty.clone(),
        json!({"kind":"default","ty":ty}),
    );
    let w = c["witnesses"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|w| w["name"] == "local_secret_key")
        .unwrap();
    w["parameters"] = json!([{"name":"payload","ty":ty}]);
    visit(&mut c, &mut |v| {
        if v["kind"] == "witness_call" && v["name"] == "local_secret_key" {
            v["arguments"] = json!([call("empty_payload", vec![])]);
        }
    });
    let invoke = json!({"kind":"witness_call","name":"local_secret_key","arguments":[reference("saved_payload")]});
    prepend(
        &mut c,
        "withdraw",
        vec![
            binding("saved_payload", ty, call("empty_payload", vec![])),
            binding(
                "first_key",
                json!({"kind":"bytes","length":32}),
                invoke.clone(),
            ),
            binding("reused_key", json!({"kind":"bytes","length":32}), invoke),
        ],
    );
    included(c, "withdraw", "owned-witness-argument-reuse");
}
#[test]
fn actionful_opening_witness_uses_structure_not_product_names() {
    let mut c = source();
    replace(&mut c, "Committable", "OpeningPayload");
    replace(&mut c, "Board", "OpenedPosition");
    included(c, "concede", "actionful-opening-product-names");
}
