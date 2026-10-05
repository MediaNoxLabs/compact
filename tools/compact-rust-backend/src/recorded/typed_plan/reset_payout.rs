// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//  	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//! Historical coin payout followed by one closed, literal-true reset helper.
//! Admission is separate from advance's false reset and readonly payout. The
//! shared Plan owns all evaluation, scopes, checked effects and helper calls.
use super::*;

fn optional_key(ty: &Type) -> bool {
    matches!(ty, Type::Struct { fields, .. }
        if matches!(fields.as_slice(), [tag, value]
            if tag.name == "is_some" && tag.ty == Type::Boolean
                && value.name == "value" && wrapped_bytes32(&value.ty)))
}
pub(super) fn read_type(ty: &Type) -> bool {
    matches!(ty, Type::Enum { .. })
        || *ty == crate::stateful::qualified_coin_type()
        || optional_key(ty)
}
pub(super) fn write_type(ty: &Type) -> bool {
    matches!(ty, Type::Boolean | Type::Enum { .. })
        || *ty == crate::stateful::qualified_coin_type()
        || optional_string(ty)
        || optional_key(ty)
}
pub(super) fn true_arguments(arguments: &[Expr]) -> bool {
    matches!(arguments, [value] if matches!(immediate_send::uncoerced(value), Expr::Boolean { value: true }))
}
fn reset_value(value: &Expr, pure: &HashMap<&str, &PureCircuit>) -> bool {
    match value {
        Expr::Parameter { .. } | Expr::Boolean { .. } | Expr::EnumVariant { .. } => true,
        Expr::UnsignedLiteral { value, max } => value == "1" && max == "65535",
        Expr::Default { ty } => *ty == crate::stateful::qualified_coin_type(),
        Expr::Coerce { value, .. } => reset_value(value, pure),
        Expr::Equal { left, right } => reset_value(left, pure) && reset_value(right, pure),
        Expr::If {
            condition,
            then,
            otherwise,
        } => [condition, then, otherwise]
            .iter()
            .all(|value| reset_value(value, pure)),
        Expr::Let { bindings, body } => {
            bindings
                .iter()
                .all(|binding| reset_value(&binding.value, pure))
                && reset_value(body, pure)
        }
        // Plan::pure_call audits complete declaration bodies, types and cycles.
        Expr::Call { name, arguments } => {
            pure.contains_key(name.as_str())
                && arguments.iter().all(|argument| reset_value(argument, pure))
        }
        _ => false,
    }
}
fn reset_action(action: &StateAction, pure: &HashMap<&str, &PureCircuit>) -> bool {
    match action {
        StateAction::Sequence { actions } => {
            actions.iter().all(|action| reset_action(action, pure))
        }
        StateAction::Let { bindings, action } => {
            bindings
                .iter()
                .all(|binding| reset_value(&binding.value, pure))
                && reset_action(action, pure)
        }
        StateAction::If {
            condition,
            then,
            otherwise,
        } => {
            reset_value(condition, pure)
                && reset_action(then, pure)
                && reset_action(otherwise, pure)
        }
        StateAction::CellWrite { value, .. } => reset_value(value, pure),
        StateAction::CounterReset { .. }
        | StateAction::MerkleResetToDefault { .. }
        | StateAction::SetReset { .. } => true,
        // The amount is a typed literal or scoped parameter; neither can hide an effect.
        // The shared Plan validates parameter scope/type; this audit bounds its initializer.
        StateAction::CounterIncrement {
            amount: CounterAmount::Literal { value: 1 } | CounterAmount::Parameter { .. },
            ..
        } => true,
        _ => false,
    }
}
pub(super) fn helper_shape(circuit: &StatefulCircuit, pure: &HashMap<&str, &PureCircuit>) -> bool {
    phase_reset::helper_signature(circuit)
        && circuit
            .actions
            .iter()
            .all(|action| reset_action(action, pure))
}
pub(super) fn lower<'a>(
    circuit: &StatefulCircuit,
    ledger: &'a HashMap<&'a str, &'a LedgerField>,
    witnesses: &'a HashMap<&'a str, &'a WitnessDeclaration>,
    pure: &'a HashMap<&'a str, &'a PureCircuit>,
    circuits: &'a HashMap<&'a str, &'a StatefulCircuit>,
) -> Option<TypedPlan> {
    let StateReturn::Expression { value } = &circuit.return_value else {
        return None;
    };
    if circuit.internal
        || !circuit.parameters.is_empty()
        || !circuit.actions.is_empty()
        || circuit.result != crate::stateful::shielded_coin_type()
        || shielded_payout::actionful_call_paths(
            value,
            pure,
            circuits,
            &mut HashSet::from([circuit.name.clone()]),
            true,
        )? != (1, 1)
        || !shielded_value(
            value,
            pure,
            circuits,
            &mut HashSet::from([circuit.name.clone()]),
            CompositeDomain::ResetShieldedPayout,
        )
    {
        return None;
    }
    let mut plan = shielded_plan(
        ledger,
        witnesses,
        pure,
        circuits,
        CompositeDomain::ResetShieldedPayout,
    );
    let mut steps = Vec::new();
    let result = plan.expression(value, &Scope::new(), &mut steps)?;
    // Require the effect family, not a source identity or arbitrary exact node count.
    (result.ty == circuit.result
        && plan.witness_calls == 0
        && plan.cell_reads > 0
        && plan.counter_reads > 0
        && plan.counter_comparisons > 0
        && plan.zswap_inputs > 0
        && plan.zswap_outputs > 0
        && plan.intent_queries > 0
        && plan.cell_writes > 0
        && plan.counter_writes > 0
        && plan.tree_writes > 0
        && plan.set_writes > 0)
        .then_some(TypedPlan {
            steps,
            result: result.value,
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};
    fn source() -> Value {
        serde_json::from_str(include_str!(
            "../../../tests/micro-dao-cash-out-schema20-ir.json"
        ))
        .unwrap()
    }
    fn entry<'a>(source: &'a mut Value, name: &str) -> &'a mut Value {
        source["stateful_circuits"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|c| c["name"] == name)
            .unwrap()
    }
    fn admitted(source: &Value) -> Option<String> {
        let c: crate::ir::Contract = serde_json::from_value(source.clone()).unwrap();
        let ledger = c.ledger_fields.iter().map(|f| (f.id.as_str(), f)).collect();
        let witnesses = c.witnesses.iter().map(|w| (w.name.as_str(), w)).collect();
        let pure = c.circuits.iter().map(|c| (c.name.as_str(), c)).collect();
        let circuits = c
            .stateful_circuits
            .iter()
            .map(|c| (c.name.as_str(), c))
            .collect();
        let root = c
            .stateful_circuits
            .iter()
            .find(|c| c.name.ends_with("cash_out"))?;
        let plan = lower(root, &ledger, &witnesses, &pure, &circuits)?;
        let steps = plan.steps;
        Some(quote::quote!(#(#steps)*).to_string())
    }
    fn edit(v: &mut Value, change: &mut impl FnMut(&mut Value) -> bool) -> bool {
        if change(v) {
            return true;
        }
        match v {
            Value::Object(m) => m.values_mut().any(|v| edit(v, change)),
            Value::Array(a) => a.iter_mut().any(|v| edit(v, change)),
            _ => false,
        }
    }
    #[test]
    fn original_cash_out_preserves_send_before_reset_and_saved_result() {
        let mut s = source();
        let tokens = admitted(&s).expect("original cash_out must be admitted");
        assert!(tokens.find("create_zswap_output").unwrap() < tokens.find("record_reset").unwrap());
        assert!(tokens.contains("own_coin_public_key"));
        assert!(!tokens.contains("try_witness_metered"));
        entry(&mut s, "cash_out")["name"] = json!("renamed_cash_out");
        assert!(admitted(&s).is_some());
    }
    #[test]
    fn reset_selection_and_every_executable_path_are_bounded() {
        for case in 0..4 {
            let mut s = source();
            let root = entry(&mut s, "cash_out");
            match case {
                0 => {
                    assert!(edit(root, &mut |v| {
                        if v["kind"] == "call" && v["name"] == "reset_state" {
                            v["arguments"][0] = json!({"kind":"boolean","value":false});
                            true
                        } else {
                            false
                        }
                    }));
                }
                1 => {
                    assert!(edit(root, &mut |v| {
                        if v["kind"] == "call" && v["name"] == "reset_state" {
                            v["arguments"][0] = json!({"kind":"equal","left":{"kind":"boolean","value":true},"right":{"kind":"boolean","value":true}});
                            true
                        } else {
                            false
                        }
                    }));
                }
                2 => {
                    let tail = root["return_value"]["value"].take();
                    root["return_value"]["value"] = json!({"kind":"sequence","steps":[{"kind":"call","name":"reset_state","arguments":[{"kind":"boolean","value":true}]}],"value":tail});
                }
                3 => {
                    let tail = root["return_value"]["value"].take();
                    let ty = root["result"].clone();
                    root["return_value"]["value"] = json!({"kind":"if","condition":{"kind":"boolean","value":true},"then":tail,"otherwise":{"kind":"default","ty":ty}});
                }
                _ => unreachable!(),
            }
            assert!(admitted(&s).is_none(), "case {case}");
        }
    }
    #[test]
    fn reset_body_unused_bindings_and_unselected_branches_are_audited() {
        for hidden in [
            json!({"kind":"cell_read","field":"state","index":1}),
            json!({"kind":"native_witness_call","builtin":"own_public_key"}),
            json!({"kind":"witness_call","name":"local_secret_key","arguments":[]}),
            json!({"kind":"counter_less_than","field":"no","index":5,"threshold":{"kind":"unsigned_literal","value":"1","max":"18446744073709551615"}}),
        ] {
            let mut s = source();
            let h = entry(&mut s, "reset_state");
            let old = h["actions"].take();
            h["actions"] = json!([{"kind":"let","bindings":[{"name":"unused","ty":{"kind":"boolean"},"value":hidden}],"action":{"kind":"sequence","actions":old}}]);
            assert!(admitted(&s).is_none());
        }
        let mut s = source();
        assert!(edit(entry(&mut s, "reset_state"), &mut |v| {
            if v["kind"] == "if" {
                v["otherwise"] = json!({"kind":"counter_increment","field":"round","index":6,"amount":{"kind":"literal","value":2}});
                true
            } else {
                false
            }
        }));
        assert!(admitted(&s).is_none());
    }
    #[test]
    fn malformed_slots_pure_effects_and_callee_scope_fail_closed() {
        for case in 0..5 {
            let mut s = source();
            match case {
                0 => {
                    assert!(edit(entry(&mut s, "cash_out"), &mut |v| {
                        if v["kind"] == "cell_read" && v["field"] == "beneficiary" {
                            v["index"] = json!(10);
                            true
                        } else {
                            false
                        }
                    }));
                }
                1 => {
                    entry(&mut s, "reset_state")["parameters"][0]["ty"] = json!({"kind":"field"});
                }
                2 => {
                    assert!(edit(entry(&mut s, "reset_state"), &mut |v| {
                        if v["kind"] == "cell_write" && v["field"] == "pot_has_coin" {
                            v["value"] = json!({"kind":"parameter","name":"addr"});
                            true
                        } else {
                            false
                        }
                    }));
                }
                3 => {
                    let pure = s["circuits"]
                        .as_array_mut()
                        .unwrap()
                        .iter_mut()
                        .find(|c| c["name"] == "__compact_function_none_184")
                        .unwrap();
                    let body = pure["body"].take();
                    pure["body"] = json!({"kind":"let","bindings":[{"name":"unused","ty":{"kind":"bytes","length":32},"value":{"kind":"witness_call","name":"local_secret_key","arguments":[]}}],"body":body});
                }
                4 => {
                    assert!(edit(entry(&mut s, "reset_state"), &mut |v| {
                        if v["kind"] == "merkle_reset_to_default" {
                            v["index"] = json!(8);
                            true
                        } else {
                            false
                        }
                    }));
                }
                _ => unreachable!(),
            }
            assert!(admitted(&s).is_none(), "case {case}");
        }
    }
}
