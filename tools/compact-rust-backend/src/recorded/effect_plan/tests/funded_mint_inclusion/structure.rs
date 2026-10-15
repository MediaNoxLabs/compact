// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;

#[test]
fn original_and_renamed_funding_are_complete_old_inclusions() {
    included(source(), "buy_in", "original-full-dao");
    let mut c = source();
    for (old, new) in [
        ("buy_in", "purchase"),
        ("pot", "reserve"),
        ("pot_has_coin", "occupied"),
        ("costs", "prices"),
        ("Costs", "Tariff"),
        ("seed_dust", "seed_price"),
        ("buy_in_dust", "unit_price"),
        ("receiveShielded", "receive_payment"),
        ("mergeCoinImmediate", "combine_received"),
        ("mintShieldedToken", "mint_purchase"),
    ] {
        replace(&mut c, old, new);
    }
    let r = circuit(&mut c, "purchase");
    for (index, old, new) in [(0, "coin", "payment"), (1, "amount", "quantity")] {
        r["parameters"][index]["name"] = json!(new);
        visit(r, &mut |v| {
            if v["kind"] == "parameter" && v["name"] == old {
                v["name"] = json!(new);
            }
        });
    }
    included(c, "purchase", "coherent-all-symbols");
}
#[test]
fn internal_complete_root_has_the_same_primitives_and_a_real_helper_owner() {
    let mut c = source();
    let mut helper = circuit(&mut c, "buy_in").clone();
    helper["name"] = json!("internal_purchase");
    helper["internal"] = json!(true);
    c["stateful_circuits"].as_array_mut().unwrap().push(helper);
    included_with_internal(c, "buy_in", "internal-root", Some("internal_purchase"));
}
#[test]
fn prefix_wrappers_and_lazy_arm_orientation_preserve_received_provenance() {
    for (mode, label) in [
        (0, "prefix-sequence"),
        (1, "prefix-let-assert"),
        (2, "phase-arm-swap"),
        (3, "receiver-forwarding-scope"),
    ] {
        let mut c = source();
        match mode {
            0 => {
                let actions = circuit(&mut c, "buy_in")["actions"].take();
                circuit(&mut c, "buy_in")["actions"] =
                    json!([{"kind":"sequence","actions":actions}]);
            }
            1 => {
                let actions = circuit(&mut c, "buy_in")["actions"].take();
                circuit(&mut c, "buy_in")["actions"] = json!([{"kind":"let","bindings":[binding("prefix_ok",json!({"kind":"boolean"}),json!({"kind":"boolean","value":true}))],"action":{"kind":"sequence","actions":[{"kind":"assert","condition":reference("prefix_ok"),"message":"prefix"},{"kind":"sequence","actions":actions}]}}]);
            }
            2 => {
                let branch = &mut circuit(&mut c, "buy_in")["actions"][3];
                assert_eq!(branch["kind"], "if");
                let condition = branch["condition"].take();
                let then = branch["then"].take();
                branch["condition"] = json!({"kind":"equal","left":condition,"right":{"kind":"boolean","value":false}});
                branch["then"] = branch["otherwise"].take();
                branch["otherwise"] = then;
            }
            3 => {
                let helper = circuit(&mut c, "receiveShielded");
                helper["parameters"][0]["name"] = json!("incoming");
                visit(helper, &mut |v| {
                    if v["kind"] == "parameter" && v["name"] == "coin" {
                        v["name"] = json!("incoming");
                    }
                });
                let actions = helper["actions"].take();
                helper["actions"] = json!([{"kind":"let","bindings":[binding("forwarding_ok",json!({"kind":"boolean"}),json!({"kind":"boolean","value":true}))],"action":{"kind":"sequence","actions":actions}}]);
            }
            _ => unreachable!(),
        }
        included(c, "buy_in", label);
    }
}
#[test]
fn result_wrappers_preserve_the_exact_coin_interface_and_effect_count() {
    for (mode, label) in [
        (0, "result-coerce"),
        (1, "result-saved-let"),
        (2, "result-unit-sequence"),
        (3, "return-local-shadow"),
        (4, "coin-product-projection"),
    ] {
        let mut c = source();
        let ty = circuit(&mut c, "buy_in")["result"].clone();
        let original = root(&mut c, "buy_in").take();
        *root(&mut c, "buy_in") = match mode {
            0 => json!({"kind":"coerce","value":original,"ty":ty}),
            1 => {
                json!({"kind":"let","bindings":[binding("minted",ty,original)],"body":reference("minted")})
            }
            2 => json!({"kind":"sequence","steps":[{"kind":"unit"}],"value":original}),
            3 => {
                json!({"kind":"let","bindings":[binding("amount",json!({"kind":"unsigned","max":u64::MAX.to_string()}),reference("amount"))],"body":original})
            }
            4 => {
                let product = json!({"kind":"struct","name":"MintedPair","fields":[{"name":"first","ty":ty},{"name":"again","ty":ty}]});
                json!({"kind":"let","bindings":[binding("minted",ty,original)],"body":{"kind":"struct_field","value":{"kind":"struct_literal","ty":product,"fields":[reference("minted"),reference("minted")]},"field":"first","index":0}})
            }
            _ => unreachable!(),
        };
        included(c, "buy_in", label);
    }
}
#[test]
fn actionless_helpers_keep_fresh_scopes_and_product_returns() {
    for (product_result, label) in [
        (false, "actionless-mint-forwarder"),
        (true, "helper-owned-product-result"),
    ] {
        let mut c = source();
        let ty = circuit(&mut c, "buy_in")["result"].clone();
        let original = root(&mut c, "buy_in").take();
        let product = json!({"kind":"struct","name":"MintReceipt","fields":[{"name":"coin","ty":ty},{"name":"note","ty":{"kind":"opaque_string"}}]});
        pure(
            &mut c,
            "empty_note",
            vec![],
            json!({"kind":"opaque_string"}),
            json!({"kind":"default","ty":{"kind":"opaque_string"}}),
        );
        let result = if product_result {
            product.clone()
        } else {
            ty.clone()
        };
        let body = if product_result {
            json!({"kind":"struct_literal","ty":product,"fields":[reference("amount"),call("empty_note",vec![])]})
        } else {
            reference("amount")
        };
        c["stateful_circuits"].as_array_mut().unwrap().push(json!({"name":"finish_purchase","internal":true,"parameters":[{"name":"amount","ty":ty}],"result":result,"actions":[],"return_value":{"kind":"expression","value":body}}));
        let invocation = call("finish_purchase", vec![original]);
        *root(&mut c, "buy_in") = if product_result {
            json!({"kind":"struct_field","value":invocation,"field":"coin","index":0})
        } else {
            invocation
        };
        included(c, "buy_in", label);
    }
}
#[test]
fn canonical_flat_slots_can_move_without_names_or_indices_becoming_profiles() {
    let mut c = source();
    visit(&mut c, &mut |v| {
        let id = v["field"].as_str().or_else(|| v["id"].as_str());
        let index = match id {
            Some("pot") => Some(12),
            Some("costs") => Some(10),
            _ => None,
        };
        if let Some(index) = index {
            if v.get("index").is_some() {
                v["index"] = json!(index);
            }
            if v.get("path").is_some() {
                v["path"] = json!([index]);
            }
        }
    });
    c["ledger_fields"].as_array_mut().unwrap().swap(10, 12);
    included(c, "buy_in", "flat-slot-permutation");
}
