// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;

#[test]
fn both_complete_payout_modes_and_coherent_renaming_compose() {
    for (name, baseline, renamed, label) in [
        (
            "withdraw",
            "readonly-full-coracle",
            "renamed_withdraw",
            "readonly-coherent-names",
        ),
        (
            "concede",
            "actionful-full-coracle",
            "renamed_concede",
            "actionful-coherent-names",
        ),
    ] {
        included(source(), name, baseline);
        let mut c = source();
        for (old, new) in [
            (name, renamed),
            ("pot", "payout_pot"),
            ("red_withdraw", "red_payment"),
            ("blue_concede", "blue_concession"),
            ("is_red", "is_red_player"),
        ] {
            replace(&mut c, old, new);
        }
        included(c, renamed, label);
    }
}
#[test]
fn typed_root_wrappers_preserve_readonly_and_actionful_payouts() {
    for (mode, name, label) in [
        (0, "withdraw", "readonly-coerce"),
        (1, "concede", "actionful-saved-let"),
        (2, "withdraw", "readonly-unit-sequence"),
    ] {
        let mut c = source();
        let ty = circuit(&mut c, name)["result"].clone();
        let original = root(&mut c, name).take();
        *root(&mut c, name) = match mode {
            0 => json!({"kind":"coerce","value":original,"ty":ty}),
            1 => {
                json!({"kind":"let","bindings":[binding("saved_payout",ty,original)],"body":reference("saved_payout")})
            }
            2 => {
                json!({"kind":"sequence","steps":[{"kind":"assert","condition":{"kind":"boolean","value":true},"message":"wrapper"}],"value":original})
            }
            _ => unreachable!(),
        };
        included(c, name, label);
    }
}
#[test]
fn recursive_coin_results_reuse_one_saved_payout_instead_of_repeating_effects() {
    for (name, label) in [
        ("withdraw", "readonly-nested-result"),
        ("concede", "actionful-nested-result"),
    ] {
        let mut c = source();
        let original_ty = circuit(&mut c, name)["result"].clone();
        let inner = json!({"kind":"struct","name":"NestedPayout","fields":[{"name":"saved","ty":original_ty}]});
        let outer = json!({"kind":"struct","name":"PayoutEnvelope","fields":[{"name":"first","ty":inner},{"name":"reused","ty":inner}]});
        let original = root(&mut c, name).take();
        let wrapped =
            json!({"kind":"struct_literal","ty":inner,"fields":[reference("saved_payout")]});
        *root(&mut c, name) = json!({"kind":"let","bindings":[binding("saved_payout",original_ty,original)],"body":{
            "kind":"struct_literal","ty":outer,"fields":[wrapped,wrapped]
        }});
        circuit(&mut c, name)["result"] = outer;
        included(c, name, label);
    }
}
#[test]
fn actionless_forwarding_helpers_preserve_both_old_path_classifications() {
    for (name, label) in [
        ("withdraw", "readonly-forwarding"),
        ("concede", "actionful-forwarding"),
    ] {
        let mut c = source();
        let mut helper = circuit(&mut c, name).clone();
        helper["name"] = json!("forward_payout");
        helper["internal"] = json!(true);
        c["stateful_circuits"].as_array_mut().unwrap().push(helper);
        *root(&mut c, name) = call("forward_payout", vec![]);
        included(c, name, label);
    }
}
#[test]
fn actionful_prefix_wrappers_do_not_change_the_one_enum_write_obligation() {
    let mut c = source();
    for name in ["red_concede", "blue_concede"] {
        let helper = circuit(&mut c, name);
        let actions = helper["actions"].take();
        helper["actions"] = json!([{"kind":"let","bindings":[binding("prefix_guard",json!({"kind":"boolean"}),json!({"kind":"boolean","value":true}))],"action":{
            "kind":"sequence","actions":[{"kind":"assert","condition":reference("prefix_guard"),"message":"prefix"},{"kind":"sequence","actions":actions}]
        }}]);
    }
    included(c, "concede", "actionful-sequence-let-assert");
}
#[test]
fn repeated_native_key_values_remain_distinct_from_ordinary_witnesses() {
    let mut c = source();
    let ty =
        serde_json::to_value(crate::ir::NativeWitnessBuiltin::OwnPublicKey.result_type()).unwrap();
    prepend(
        &mut c,
        "red_concede",
        vec![binding(
            "additional_key",
            ty,
            json!({"kind":"native_witness_call","builtin":"own_public_key"}),
        )],
    );
    included(c, "concede", "actionful-key-after-write");
}
