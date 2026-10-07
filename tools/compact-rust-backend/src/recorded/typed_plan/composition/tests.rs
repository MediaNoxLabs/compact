// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use super::*;
use crate::ir::Contract;
use crate::{RenderError, render_with_capabilities};
use serde_json::{Value, json};

fn fixture(name: &str) -> Value {
    let text = match name {
        "flat" => include_str!("../../../../tests/unit-composition/flat.json"),
        "chunked" => include_str!("../../../../tests/unit-composition/chunked.json"),
        "crypto" => include_str!("../../../../tests/unit-composition/crypto.json"),
        "guard" => include_str!("../../../../tests/set-composition/pure_unit_guard.json"),
        "zswap" => include_str!("../../../../tests/native-zswap-intents-schema18-ir.json"),
        _ => unreachable!(),
    };
    serde_json::from_str(text).unwrap()
}
fn circuit<'a>(v: &'a mut Value, name: &str) -> &'a mut Value {
    v["stateful_circuits"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|c| c["name"] == name)
        .unwrap()
}
fn attempt(value: Value, name: &str) -> ProfileAttempt<TypedPlan, CompositionRejection> {
    let c: Contract = serde_json::from_value(value).unwrap();
    let ledger = c.ledger_fields.iter().map(|x| (x.id.as_str(), x)).collect();
    let witnesses = c.witnesses.iter().map(|x| (x.name.as_str(), x)).collect();
    let pure = c.circuits.iter().map(|x| (x.name.as_str(), x)).collect();
    let circuits: HashMap<_, _> = c
        .stateful_circuits
        .iter()
        .map(|x| (x.name.as_str(), x))
        .collect();
    lower_checked(circuits[name], &ledger, &witnesses, &pure, &circuits)
}
fn rejected(value: Value, name: &str) -> CompositionRejection {
    match attempt(value, name) {
        ProfileAttempt::Rejected(r) => r,
        ProfileAttempt::Admitted(_) => panic!("incomplete/rejected domain escaped as a plan"),
        ProfileAttempt::NotApplicable => panic!("expected a matched Unit candidate"),
    }
}
fn unsupported() -> Value {
    json!({"kind":"expression","value":{"kind":"kernel_self","ty":{"kind":"struct","name":"ContractAddress","fields":[{"name":"bytes","ty":{"kind":"bytes","length":32}}]}}})
}

#[test]
fn signatures_are_unrelated_but_complete_helper_domains_are_admitted() {
    for name in ["flat", "chunked", "crypto"] {
        assert!(matches!(
            attempt(fixture(name), "close"),
            ProfileAttempt::Admitted(_)
        ));
    }
    for name in ["run", "local_control", "transitive_control"] {
        assert!(matches!(
            attempt(fixture("guard"), name),
            ProfileAttempt::Admitted(_)
        ));
    }
    for mutation in ["result", "return", "parameter"] {
        let mut v = fixture("flat");
        let c = circuit(&mut v, "close");
        match mutation {
            "result" => c["result"] = json!({"kind":"field"}),
            "return" => c["return_value"] = json!({"kind":"expression","value":{"kind":"unit"}}),
            _ => {
                c["parameters"][0]["ty"] =
                    json!({"kind":"unsigned","max":"340282366920938463463374607431768211455"})
            }
        }
        assert!(
            matches!(attempt(v, "close"), ProfileAttempt::NotApplicable),
            "{mutation}"
        );
    }
}

#[test]
fn final_obligations_distinguish_no_effect_from_missing_helper() {
    let mut v = fixture("flat");
    circuit(&mut v, "close")["actions"] = json!([]);
    let r = rejected(v, "close");
    assert_eq!(
        r.gap.code,
        super::super::super::RecordingGapCode::NoRecordedEffect
    );
    assert_eq!(r.phase, RejectionPhase::FinalObligation);
    assert_eq!(r.root_action, None);
    let mut v = fixture("flat");
    circuit(&mut v, "close")["actions"] = json!([{"kind":"cell_write","field":"active","index":0,"value":{"kind":"boolean","value":false}}]);
    let r = rejected(v, "close");
    assert_eq!(
        r.gap.code,
        super::super::super::RecordingGapCode::RecordingUnavailable
    );
    assert!(r.gap.detail.contains("Unit helper"));
}

#[test]
fn unselected_effect_retains_audit_path_without_replacing_recursive_legacy_failure() {
    for retain_earlier_guard in [false, true] {
        let mut v = fixture("flat");
        if !retain_earlier_guard {
            circuit(&mut v, "guard")["actions"] = json!([]);
        }
        circuit(&mut v, "guard")["actions"]
            .as_array_mut()
            .unwrap()
            .push(json!({
                "kind":"if","condition":{"kind":"boolean","value":true},
                "then":{"kind":"sequence","actions":[]},"otherwise":unsupported()
            }));
        let r = rejected(v.clone(), "close");
        assert_eq!(r.root_action, Some(0));
        assert_eq!(r.precision, RejectionPrecision::ConcreteNode);
        let index = usize::from(retain_earlier_guard);
        assert_eq!(
            r.gap.path,
            format!("actions[0].callee[\"guard\"].actions[{index}].otherwise")
        );
        let c: Contract = serde_json::from_value(v).unwrap();
        let result = render_with_capabilities(&c).unwrap();
        let gap = result
            .capabilities
            .circuits
            .iter()
            .find(|x| x.name == "close")
            .unwrap()
            .recording_unavailable
            .as_ref()
            .unwrap();
        if retain_earlier_guard {
            // Legacy lowering already refused its Counter-reading assertion.
            // The newer domain's later, prettier path must not replace it.
            assert_eq!(gap.path, "callee[guard].actions[0]");
        } else {
            assert_eq!(gap.path, "callee[guard].actions[0].otherwise");
        }
    }
}

#[test]
fn unused_binding_in_pure_guard_keeps_its_transitive_location() {
    let mut v = fixture("guard");
    let g = v["circuits"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|c| c["name"] == "localGuard")
        .unwrap();
    g["body"]["steps"][0]["bindings"][0]["value"] =
        json!({"kind":"cell_read","field":"active","index":0});
    let r = rejected(v, "transitive_control");
    assert_eq!(r.gap.ir_node, "Expr::CellRead");
    assert_eq!(
        r.gap.path,
        "actions[0].callee[\"transitiveGuard\"].body.steps[0].callee[\"localGuard\"].body.steps[0].bindings[0].value"
    );
}

#[test]
fn argument_is_audited_before_callee() {
    let mut v = fixture("flat");
    circuit(&mut v, "close")["actions"][0]["arguments"][0] = json!({"kind":"let","bindings":[{"name":"unused","ty":{"kind":"boolean"},"value":{"kind":"set_size","field":"unreviewed","index":4}}],"body":{"kind":"parameter","name":"expected"}});
    let r = rejected(v, "close");
    assert_eq!(r.gap.path, "actions[0].arguments[0].bindings[0].value");
}

#[test]
fn failed_suffix_cannot_publish_successful_prefix() {
    let mut v = fixture("flat");
    circuit(&mut v, "close")["actions"]
        .as_array_mut()
        .unwrap()
        .push(unsupported());
    let r = rejected(v, "close");
    assert_eq!(r.root_action, Some(4));
    assert_eq!(r.gap.path, "actions[4]");
}

#[test]
fn helper_cycle_is_a_matched_refusal_with_call_site() {
    let mut v = fixture("flat");
    circuit(&mut v,"guard")["actions"].as_array_mut().unwrap().push(json!({"kind":"circuit_call","name":"guard","arguments":[{"kind":"parameter","name":"expected"}]}));
    let r = rejected(v, "close");
    assert_eq!(r.gap.path, "actions[0].callee[\"guard\"].actions[1]");
    assert_eq!(r.gap.detail, "recursive helper call: \"guard\"");
}

#[test]
fn public_native_validation_still_precedes_defensive_typed_plan_refusal() {
    let mut v = fixture("flat");
    circuit(&mut v, "close")["actions"][2]["value"] =
        json!({"kind":"parameter","name":"outside_scope"});
    let r = rejected(v.clone(), "close");
    assert_eq!(r.phase, RejectionPhase::TypedLowering);
    assert_eq!(r.precision, RejectionPrecision::EnclosingAction);
    assert_eq!(r.root_action, Some(2));
    let c: Contract = serde_json::from_value(v).unwrap();
    assert!(matches!(
        render_with_capabilities(&c),
        Err(RenderError::Located { .. })
    ));
}

#[test]
fn local_refusal_can_become_a_complete_recorded_helper() {
    let v = fixture("flat");
    let c: Contract = serde_json::from_value(v.clone()).unwrap();
    let guard = c
        .stateful_circuits
        .iter()
        .find(|x| x.name == "guard")
        .unwrap();
    let witnesses = c.witnesses.iter().map(|x| (x.name.as_str(), x)).collect();
    let circuits = c
        .stateful_circuits
        .iter()
        .map(|x| (x.name.as_str(), x))
        .collect();
    assert!(!super::super::super::audited_local::unit_helper(
        guard, &witnesses, &circuits
    ));
    assert!(matches!(attempt(v, "close"), ProfileAttempt::Admitted(_)));
}

#[test]
fn later_zswap_domain_independently_accepts_original_ir() {
    let mut v = fixture("zswap");
    v["schema_version"] = json!(20);
    let c = circuit(&mut v, "witness_order");
    c["parameters"]
        .as_array_mut()
        .unwrap()
        .push(json!({"name":"commitment","ty":{"kind":"bytes","length":32}}));
    c["actions"].as_array_mut().unwrap().push(json!({"kind":"expression","value":{"kind":"kernel_claim","claim":"coin_receive","value":{"kind":"parameter","name":"commitment"}}}));
    assert_eq!(
        rejected(v.clone(), "witness_order").phase,
        RejectionPhase::PolicyAudit
    );
    let c: Contract = serde_json::from_value(v).unwrap();
    let ledger = c.ledger_fields.iter().map(|x| (x.id.as_str(), x)).collect();
    let witnesses = c.witnesses.iter().map(|x| (x.name.as_str(), x)).collect();
    let circuits: HashMap<_, _> = c
        .stateful_circuits
        .iter()
        .map(|x| (x.name.as_str(), x))
        .collect();
    assert!(
        super::super::super::zswap_plan::lower(
            circuits["witness_order"],
            &ledger,
            &circuits,
            &witnesses
        )
        .is_some()
    );
    let result = render_with_capabilities(&c).unwrap();
    assert!(
        result
            .capabilities
            .circuits
            .iter()
            .find(|x| x.name == "witness_order")
            .unwrap()
            .recorded
    );
}

#[test]
fn refinement_uses_typed_ordinal_phase_and_precision_not_diagnostic_strings() {
    let mut v = fixture("flat");
    circuit(&mut v, "guard")["actions"]
        .as_array_mut()
        .unwrap()
        .push(unsupported());
    let r = rejected(v, "close");
    let action: StateAction = serde_json::from_value(unsupported()).unwrap();
    let coarse = RecordingGap::action(&action, "arbitrary display string".into());
    assert_eq!(
        r.refine_action(
            0,
            super::super::super::LegacyActionPrecision::Coarse,
            coarse.clone()
        ),
        r.gap
    );
    assert_eq!(
        r.refine_action(
            1,
            super::super::super::LegacyActionPrecision::Coarse,
            coarse.clone()
        ),
        coarse
    );
    let concrete = RecordingGap::expression(&Expr::Unit, "actions[0]".into());
    assert_eq!(
        r.refine_action(
            0,
            super::super::super::LegacyActionPrecision::Coarse,
            concrete.clone()
        ),
        concrete
    );
    assert_eq!(
        r.refine_action(
            0,
            super::super::super::LegacyActionPrecision::Concrete,
            coarse.clone()
        ),
        coarse
    );
    let mut enclosing = r;
    enclosing.precision = RejectionPrecision::EnclosingAction;
    assert_eq!(
        enclosing.refine_action(
            0,
            super::super::super::LegacyActionPrecision::Coarse,
            coarse.clone()
        ),
        coarse
    );
}

#[test]
fn direct_root_call_argument_refusal_exposes_the_nested_audit_reason() {
    let mut v = fixture("flat");
    circuit(&mut v, "close")["actions"][0]["arguments"][0] = json!({
        "kind":"let", "bindings":[{"name":"unused", "ty":{"kind":"struct","name":"ContractAddress","fields":[{"name":"bytes","ty":{"kind":"bytes","length":32}}]}, "value":unsupported()["value"]}],
        "body":{"kind":"parameter","name":"expected"}
    });
    let r = rejected(v.clone(), "close");
    assert_eq!(r.gap.path, "actions[0].arguments[0].bindings[0].value");
    let c: Contract = serde_json::from_value(v).unwrap();
    let rendered = render_with_capabilities(&c).unwrap();
    let gap = rendered
        .capabilities
        .circuits
        .iter()
        .find(|x| x.name == "close")
        .unwrap()
        .recording_unavailable
        .as_ref()
        .unwrap();
    assert_eq!(gap.path, r.gap.path);
    assert_eq!(gap.ir_node, "Expr::KernelSelf");
}
