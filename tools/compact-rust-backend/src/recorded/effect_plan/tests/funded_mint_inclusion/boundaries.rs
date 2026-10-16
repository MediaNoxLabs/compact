// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Old shape refusals, typed source errors and expansions have separate premises.
use super::*;

#[test]
fn root_carrier_and_price_authority_are_not_silently_generalized() {
    for mode in 0..4 {
        let mut c = source();
        let r = circuit(&mut c, "buy_in");
        match mode {
            0 => r["parameters"]
                .as_array_mut()
                .unwrap()
                .push(json!({"name":"extra","ty":{"kind":"boolean"}})),
            1 => r["parameters"].as_array_mut().unwrap().swap(0, 1),
            2 => r["parameters"][1]["ty"] = json!({"kind":"unsigned","max":"255"}),
            3 => r["parameters"][1]["name"] = r["parameters"][0]["name"].clone(),
            _ => unreachable!(),
        }
        let c: Contract = serde_json::from_value(c).unwrap();
        let env = Environment::new(&c);
        assert!(
            env.old(env.stateful["buy_in"]).is_none(),
            "root envelope {mode}"
        );
        // The added-unused argument and reordered declaration are native-valid
        // potential expansions; duplicate/unbound signatures are source errors.
        // None are exported or counted as whole-old inclusion.
    }
}
#[test]
fn receive_store_and_formal_shadow_requirements_remain_old_refusals() {
    for mode in 0..5 {
        let mut c = source();
        let r = circuit(&mut c, "buy_in");
        match mode {
            0 => {
                r["actions"].as_array_mut().unwrap().remove(2);
            }
            1 => assert!(first(&mut r["actions"], &mut |v| {
                if v["kind"] == "cell_write_coin" {
                    *v = json!({"kind":"sequence","actions":[]});
                    true
                } else {
                    false
                }
            })),
            2 | 3 => {
                let p = r["parameters"][mode - 2].clone();
                let actions = r["actions"].take();
                r["actions"] = json!([{"kind":"let","bindings":[binding(p["name"].as_str().unwrap(),p["ty"].clone(),reference(p["name"].as_str().unwrap()))],"action":{"kind":"sequence","actions":actions}}]);
            }
            4 => {
                let receive = r["actions"][2].clone();
                r["actions"].as_array_mut().unwrap().push(receive);
            }
            _ => unreachable!(),
        }
        let c: Contract = serde_json::from_value(c).unwrap();
        let env = Environment::new(&c);
        assert!(env.old(env.stateful["buy_in"]).is_none(), "old flow {mode}");
    }
}
#[test]
fn malformed_ledger_index_is_rejected_by_both_independent_source_paths() {
    let mut c = source();
    assert!(first(&mut circuit(&mut c, "buy_in")["actions"], &mut |v| {
        if v["kind"] == "cell_read" && v["field"] == "costs" {
            v["index"] = json!(99);
            true
        } else {
            false
        }
    }));
    let c: Contract = serde_json::from_value(c).unwrap();
    let env = Environment::new(&c);
    let r = env.stateful["buy_in"];
    assert!(env.old(r).is_none());
    let Err(PlanError::Unsupported(gap)) = env.checked(r) else {
        panic!("malformed slot must be a source refusal")
    };
    assert_eq!(gap.detail, "unknown ledger field or inconsistent index");
}
#[test]
fn recursive_root_products_are_expansions_not_old_funded_return_types() {
    let mut c = source();
    let coin = circuit(&mut c, "buy_in")["result"].clone();
    let product =
        json!({"kind":"struct","name":"PublicMintReceipt","fields":[{"name":"minted","ty":coin}]});
    let original = root(&mut c, "buy_in").take();
    *root(&mut c, "buy_in") = json!({"kind":"struct_literal","ty":product,"fields":[original]});
    circuit(&mut c, "buy_in")["result"] = product;
    std::thread::Builder::new()
        .stack_size(8 * 1024 * 1024)
        .spawn(move || {
            let c: Contract = serde_json::from_value(c).unwrap();
            let env = Environment::new(&c);
            let r = env.stateful["buy_in"];
            assert!(
                env.old(r).is_none(),
                "exact old root result is ShieldedCoinInfo"
            );
            render(&env.checked(r).unwrap()).unwrap();
            current(&c, "buy_in", "root-product-expansion");
            // Kept outside raw inclusion labels and any old-family inclusion count.
        })
        .unwrap()
        .join()
        .unwrap();
}
