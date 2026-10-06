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
//! Structural policy for receive followed by a full immediate shielded send.
//! This audits every branch and binding; the shared Plan owns evaluation,
//! concrete types, lexical scope, argument ordering and same-frame calls.
use super::*;
use crate::coin_shapes::{qualified_coin_type, shielded_coin_type, shielded_recipient_type};

pub(super) fn uncoerced(value: &Expr) -> &Expr {
    match value {
        Expr::Coerce { value, .. } => uncoerced(value),
        _ => value,
    }
}
pub(super) fn parameter(value: &Expr, name: &str) -> bool {
    matches!(uncoerced(value), Expr::Parameter { name: actual } if actual == name)
}
fn member(value: &Expr, name: &str, field: &str, index: usize) -> bool {
    matches!(uncoerced(value), Expr::StructField { value, field: actual, index: position }
        if actual == field && *position == index && parameter(value, name))
}

// Qualification of this exact coin into the upstream singleton proof tree.
// Neither helper names nor a caller-provided index confer transient ownership.
pub(super) fn singleton_bridge(callee: &PureCircuit) -> bool {
    let [coin] = callee.parameters.as_slice() else {
        return false;
    };
    if coin.ty != shielded_coin_type() || callee.result != qualified_coin_type() {
        return false;
    }
    let Expr::StructLiteral { ty, fields } = &callee.body else {
        return false;
    };
    let [nonce, color, value, index] = fields.as_slice() else {
        return false;
    };
    *ty == callee.result
        && member(nonce, &coin.name, "nonce", 0)
        && member(color, &coin.name, "color", 1)
        && member(value, &coin.name, "value", 2)
        && matches!(index, Expr::UnsignedLiteral { value, max } if value == "0" && max == &u64::MAX.to_string())
}

// Shared exact singleton wrapper audit; caller policies own amount/provenance.
pub(super) fn audit_bridge(
    name: &str,
    ledger: &HashMap<&str, &LedgerField>,
    witnesses: &HashMap<&str, &WitnessDeclaration>,
    pure: &HashMap<&str, &PureCircuit>,
    circuits: &HashMap<&str, &StatefulCircuit>,
) -> Option<()> {
    let bridge = circuits.get(name)?;
    let [input, target, value] = bridge.parameters.as_slice() else {
        return None;
    };
    if !bridge.actions.is_empty()
        || !shielded_send_result(&bridge.result)
        || input.ty != shielded_coin_type()
        || target.ty != shielded_recipient_type()
        || value.ty
            != (Type::Unsigned {
                max: u128::MAX.to_string(),
            })
    {
        return None;
    }
    let StateReturn::Expression {
        value: Expr::Call {
            name: send_name,
            arguments,
        },
    } = &bridge.return_value
    else {
        return None;
    };
    let [qualified, forward_target, forward_value] = arguments.as_slice() else {
        return None;
    };
    if !parameter(forward_target, &target.name)
        || !parameter(forward_value, &value.name)
        || pure.contains_key(send_name.as_str())
    {
        return None;
    }
    let Expr::Call {
        name: qualify_name,
        arguments,
    } = uncoerced(qualified)
    else {
        return None;
    };
    if !matches!(arguments.as_slice(), [value] if parameter(value,&input.name))
        || circuits.contains_key(qualify_name.as_str())
        || !singleton_bridge(pure.get(qualify_name.as_str())?)
    {
        return None;
    }
    let send = circuits.get(send_name.as_str())?;
    // Reuse the already audited qualified-send domain; no merge/arbitrary
    // actionful helper becomes admitted by this composition.
    lower_shielded_send(send, ledger, witnesses, pure, circuits)?;
    Some(())
}

pub(super) fn lower<'a>(
    circuit: &StatefulCircuit,
    ledger: &'a HashMap<&'a str, &'a LedgerField>,
    witnesses: &'a HashMap<&'a str, &'a WitnessDeclaration>,
    pure: &'a HashMap<&'a str, &'a PureCircuit>,
    circuits: &'a HashMap<&'a str, &'a StatefulCircuit>,
) -> Option<TypedPlan> {
    let [coin, recipient] = circuit.parameters.as_slice() else {
        return None;
    };
    if coin.name == recipient.name
        || coin.ty != shielded_coin_type()
        || recipient.ty != shielded_recipient_type()
        || !shielded_send_result(&circuit.result)
    {
        return None;
    }
    let [
        prefix @ StateAction::CircuitCall {
            arguments: received,
            ..
        },
    ] = circuit.actions.as_slice()
    else {
        return None;
    };
    if !matches!(received.as_slice(), [value] if parameter(value,&coin.name))
        || !shielded_receive_action(
            prefix,
            pure,
            circuits,
            &mut HashSet::from([circuit.name.clone()]),
        )
    {
        return None;
    }
    let StateReturn::Expression { value: tail } = &circuit.return_value else {
        return None;
    };
    let Expr::Call { name, arguments } = tail else {
        return None;
    };
    let [sent, target, amount] = arguments.as_slice() else {
        return None;
    };
    if !parameter(sent, &coin.name)
        || !parameter(target, &recipient.name)
        || !member(amount, &coin.name, "value", 2)
        || pure.contains_key(name.as_str())
    {
        return None;
    }
    audit_bridge(name, ledger, witnesses, pure, circuits)?;
    if !shielded_send_value(
        tail,
        pure,
        circuits,
        &mut HashSet::from([circuit.name.clone()]),
    ) {
        return None;
    }
    let mut plan = shielded_plan(
        ledger,
        witnesses,
        pure,
        circuits,
        CompositeDomain::ImmediateShieldedSend,
    );
    let scope = circuit
        .parameters
        .iter()
        .enumerate()
        .map(|(index, p)| {
            let id = syn::Ident::new(&format!("__compact_param_{index}"), Span::call_site());
            (
                p.name.clone(),
                TypedValue {
                    ty: p.ty.clone(),
                    value: syn::parse_quote!(#id),
                },
            )
        })
        .collect();
    let mut steps = Vec::new();
    plan.action(prefix, &scope, &mut steps)?;
    // Check the prefix before adding send's branch counts to this same Plan.
    if plan.kernel_self_reads != 1
        || plan.zswap_inputs != 0
        || plan.zswap_outputs != 1
        || plan.intent_queries != 1
        || plan.witness_calls != 0
    {
        return None;
    }
    let result = plan.expression(tail, &scope, &mut steps)?;
    (result.ty == circuit.result
        && plan.witness_calls == 0
        && plan.cell_reads == 0
        && plan.cell_writes == 0
        && plan.counter_reads == 0
        && plan.counter_writes == 0
        && plan.tree_writes == 0
        && plan.set_writes == 0
        && plan.zswap_inputs == 1
        && plan.zswap_outputs >= 2)
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
            "../../../tests/transient-receive-send-schema20-ir.json"
        ))
        .unwrap()
    }
    fn admitted(value: &Value) -> bool {
        let source: crate::ir::Contract = serde_json::from_value(value.clone()).unwrap();
        let ledger = source
            .ledger_fields
            .iter()
            .map(|f| (f.id.as_str(), f))
            .collect();
        let witnesses = source
            .witnesses
            .iter()
            .map(|w| (w.name.as_str(), w))
            .collect();
        let pure = source
            .circuits
            .iter()
            .map(|c| (c.name.as_str(), c))
            .collect();
        let stateful = source
            .stateful_circuits
            .iter()
            .map(|c| (c.name.as_str(), c))
            .collect();
        lower(
            source.stateful_circuits.last().unwrap(),
            &ledger,
            &witnesses,
            &pure,
            &stateful,
        )
        .is_some()
    }
    fn root(value: &mut Value) -> &mut Value {
        value["stateful_circuits"]
            .as_array_mut()
            .unwrap()
            .last_mut()
            .unwrap()
    }
    #[test]
    fn names_do_not_select_transient_policy_and_existing_send_stays_separate() {
        let mut value = source();
        assert!(admitted(&value));
        fn rename(value: &mut Value) {
            match value {
                Value::Object(fields) => {
                    if let Some(Value::String(name)) = fields.get_mut("name")
                        && [
                            "receive_then_send",
                            "receiveShielded",
                            "sendImmediateShielded",
                            "sendShielded",
                            "upcastQualifiedCoin",
                        ]
                        .contains(&name.as_str())
                    {
                        *name = format!("renamed_{name}");
                    }
                    for child in fields.values_mut() {
                        rename(child);
                    }
                }
                Value::Array(values) => {
                    for child in values {
                        rename(child);
                    }
                }
                _ => (),
            }
        }
        rename(&mut value);
        assert!(admitted(&value));
    }
    #[test]
    fn full_same_coin_target_and_exact_receive_prefix_are_required() {
        for argument in 0..3 {
            let mut value = source();
            root(&mut value)["return_value"]["value"]["arguments"][argument] = if argument == 2 {
                json!({"kind":"unsigned_literal","value":"1","max":u128::MAX.to_string()})
            } else {
                json!({"kind":"parameter","name":"different"})
            };
            assert!(!admitted(&value));
        }
        let mut changed = source();
        root(&mut changed)["actions"][0]["arguments"][0] =
            json!({"kind":"parameter","name":"different"});
        assert!(!admitted(&changed));
        let mut extra = source();
        let prefix = root(&mut extra)["actions"][0].clone();
        root(&mut extra)["actions"]
            .as_array_mut()
            .unwrap()
            .push(prefix);
        assert!(!admitted(&extra));
    }
    #[test]
    fn singleton_bridge_and_whole_helper_graph_are_audited() {
        let mut wrong = source();
        wrong["circuits"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|c| c["name"] == "upcastQualifiedCoin")
            .unwrap()["body"]["fields"][3]["value"] = json!("1");
        assert!(!admitted(&wrong));
        let mut changed_value = source();
        changed_value["circuits"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|c| c["name"] == "upcastQualifiedCoin")
            .unwrap()["body"]["fields"][2] =
            json!({"kind":"unsigned_literal","value":"1","max":u128::MAX.to_string()});
        assert!(!admitted(&changed_value));
        let mut hidden = source();
        let helper = hidden["circuits"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|c| c["name"] == "coinNullifier")
            .unwrap();
        let original = helper["body"].take();
        helper["body"] = json!({"kind":"if","condition":{"kind":"boolean","value":true},"then":original,"otherwise":{"kind":"cell_read","field":"hidden","index":0}});
        assert!(!admitted(&hidden));
        let mut cycle = source();
        cycle["stateful_circuits"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|c| c["name"] == "sendImmediateShielded")
            .unwrap()["return_value"]["value"]["name"] = json!("sendImmediateShielded");
        assert!(!admitted(&cycle));
    }
}
