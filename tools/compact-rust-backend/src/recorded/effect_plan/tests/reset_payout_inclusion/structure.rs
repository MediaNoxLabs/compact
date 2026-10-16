// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;

#[test]
fn complete_retained_reset_payout_and_coherent_renaming_compose() {
    included(source(), "cash_out", "retained-full-dao");
    let mut c = source();
    for (old, new) in [
        ("cash_out", "renamed_cash_out"),
        ("reset_state", "reset_everything"),
        ("reset_pot", "clear_pot"),
        ("sendShielded", "send_coin"),
        ("beneficiary", "beneficiary_slot"),
        ("round", "round_slot"),
        ("tmp_186", "increment_amount"),
    ] {
        replace(&mut c, old, new);
    }
    included(c, "renamed_cash_out", "coherent-names");
}
#[test]
fn root_wrappers_keep_one_actionful_helper_per_executable_path() {
    for (mode, label) in [
        "identity-coerce",
        "saved-result-let",
        "unit-sequence",
        "both-root-branches",
    ]
    .into_iter()
    .enumerate()
    {
        let mut c = source();
        let ty = circuit(&mut c, "cash_out")["result"].clone();
        let body = root(&mut c).take();
        *root(&mut c) = match mode {
            0 => json!({"kind":"coerce","value":body,"ty":ty}),
            1 => {
                json!({"kind":"let","bindings":[binding("saved_result",ty,body)],"body":reference("saved_result")})
            }
            2 => {
                json!({"kind":"sequence","steps":[{"kind":"assert","condition":{"kind":"boolean","value":true},"message":"wrapper"}],"value":body})
            }
            3 => {
                json!({"kind":"if","condition":{"kind":"boolean","value":true},"then":body,"otherwise":body})
            }
            _ => unreachable!(),
        };
        included(c, "cash_out", label);
    }
}
#[test]
fn actionless_forwarding_and_distinct_branch_helpers_keep_old_path_bounds() {
    let mut c = source();
    let mut helper = circuit(&mut c, "cash_out").clone();
    helper["name"] = json!("forward_payout");
    helper["internal"] = json!(true);
    c["stateful_circuits"].as_array_mut().unwrap().push(helper);
    *root(&mut c) = call("forward_payout", vec![]);
    included(c, "cash_out", "actionless-forwarding");

    let mut c = source();
    let mut helper = circuit(&mut c, "reset_state").clone();
    helper["name"] = json!("alternate_reset");
    c["stateful_circuits"].as_array_mut().unwrap().push(helper);
    let first = root(&mut c).take();
    let mut second = first.clone();
    replace(&mut second, "reset_state", "alternate_reset");
    *root(&mut c) = json!({"kind":"if","condition":{"kind":"boolean","value":false},"then":first,"otherwise":second});
    included(c, "cash_out", "distinct-branch-reset-helpers");
}
#[test]
fn structural_optional_names_and_enum_names_are_not_source_identity() {
    let mut c = source();
    for (old, new) in [
        ("Maybe", "OptionalTopic"),
        ("MaybeCompact1", "OptionalBeneficiary"),
        ("LedgerState", "LifecycleStage"),
        ("setup", "initial_stage"),
    ] {
        replace(&mut c, old, new);
    }
    included(c, "cash_out", "structural-optional-and-enum-names");
}
#[test]
fn repeated_wallet_keys_and_canonical_wide_casts_remain_ordered_values() {
    let mut c = source();
    let key =
        serde_json::to_value(crate::ir::NativeWitnessBuiltin::OwnPublicKey.result_type()).unwrap();
    prepend(
        &mut c,
        vec![binding(
            "additional_key",
            key,
            json!({"kind":"native_witness_call","builtin":"own_public_key"}),
        )],
    );
    included(c, "cash_out", "repeated-wallet-key");
    let mut c = source();
    let max = "680564733841876926926749214863536422911";
    prepend(
        &mut c,
        vec![binding(
            "wide_local",
            json!({"kind":"unsigned","max":max}),
            json!({
                "kind":"unsigned_cast","max":max,"value":{"kind":"unsigned_literal","value":"7","max":"255"}
            }),
        )],
    );
    included(c, "cash_out", "canonical-wide-cast");
}
