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
//! Ordered voting-token consumption, private ballot and public commitment.
//! Admission audits the complete closure; the shared Plan alone evaluates it.
use super::immediate_send::{parameter, uncoerced};
use super::*;
use crate::coin_shapes::{shielded_coin_type, shielded_recipient_type};

enum Step<'a> {
    Binding(&'a crate::ir::LocalBinding),
    Action(&'a StateAction),
}
fn flatten<'a>(action: &'a StateAction, steps: &mut Vec<Step<'a>>) {
    match action {
        StateAction::Sequence { actions } => actions.iter().for_each(|a| flatten(a, steps)),
        StateAction::Let { bindings, action } => {
            steps.extend(bindings.iter().map(Step::Binding));
            flatten(action, steps);
        }
        _ => steps.push(Step::Action(action)),
    }
}
fn one(value: &Expr) -> bool {
    matches!(uncoerced(value), Expr::UnsignedLiteral { value, max }
        if value == "1" && max == &u128::MAX.to_string())
}
fn full_value_assert(action: &StateAction, coin: &str) -> bool {
    matches!(action, StateAction::Assert { condition: Expr::Equal { left, right }, .. }
        if one(right) && matches!(uncoerced(left), Expr::StructField { value, field, index }
            if field == "value" && *index == 2 && parameter(value, coin)))
}

fn zero_user_target(value: &Expr, pure: &HashMap<&str, &PureCircuit>) -> bool {
    let Expr::Call { name, arguments } = uncoerced(value) else {
        return false;
    };
    let [key] = arguments.as_slice() else {
        return false;
    };
    let Expr::StructLiteral { fields, .. } = uncoerced(key) else {
        return false;
    };
    if !matches!(fields.as_slice(), [Expr::BytesLiteral { bytes }] if bytes == &[0; 32]) {
        return false;
    }
    let Some(helper) = pure.get(name.as_str()) else {
        return false;
    };
    let [formal] = helper.parameters.as_slice() else {
        return false;
    };
    matches!(&helper.body, Expr::StructLiteral { ty, fields }
        if *ty == shielded_recipient_type()
        && matches!(fields.as_slice(), [Expr::Boolean { value: true }, left, Expr::Default { ty }]
            if parameter(left, &formal.name) && *ty == contract_address_type()))
}

// Root observations cannot hide intents or writes. Stateful calls are limited
// to previously audited context identity or Counter-to-bytes hash helpers.
fn observation(
    value: &Expr,
    ledger: &HashMap<&str, &LedgerField>,
    pure: &HashMap<&str, &PureCircuit>,
    circuits: &HashMap<&str, &StatefulCircuit>,
) -> bool {
    let visit = |value| observation(value, ledger, pure, circuits);
    match value {
        Expr::Parameter { .. }
        | Expr::Boolean { .. }
        | Expr::BytesLiteral { .. }
        | Expr::UnsignedLiteral { .. }
        | Expr::EnumVariant { .. }
        | Expr::CellRead { .. } => true,
        Expr::Coerce { value, .. }
        | Expr::StructField { value, .. }
        | Expr::SetMember { value, .. } => visit(value),
        Expr::Equal { left, right } => visit(left) && visit(right),
        Expr::If {
            condition,
            then,
            otherwise,
        } => visit(condition) && visit(then) && visit(otherwise),
        Expr::WitnessCall { arguments, .. } => arguments.is_empty(),
        Expr::Call { name, arguments } => {
            if pure.contains_key(name.as_str()) || !arguments.iter().all(visit) {
                return false;
            }
            let Some(callee) = circuits.get(name.as_str()) else {
                return false;
            };
            let StateReturn::Expression { value } = &callee.return_value else {
                return false;
            };
            callee.actions.is_empty()
                && (scalar_counter_hash(callee, ledger)
                    || (callee.parameters.is_empty()
                        && callee.result == (Type::Bytes { length: 32 })
                        && context_query_value(
                            value,
                            pure,
                            &mut HashSet::from([name.clone()]),
                            true,
                        )))
        }
        _ => false,
    }
}

pub(super) fn lower<'a>(
    circuit: &StatefulCircuit,
    ledger: &'a HashMap<&'a str, &'a LedgerField>,
    witnesses: &'a HashMap<&'a str, &'a WitnessDeclaration>,
    pure: &'a HashMap<&'a str, &'a PureCircuit>,
    circuits: &'a HashMap<&'a str, &'a StatefulCircuit>,
) -> Option<TypedPlan> {
    let [ballot, coin] = circuit.parameters.as_slice() else {
        return None;
    };
    if ballot.name == coin.name
        || ballot.ty != Type::Boolean
        || coin.ty != shielded_coin_type()
        || circuit.result != Type::Unit
        || !matches!(circuit.return_value, StateReturn::Unit)
    {
        return None;
    }
    let mut ordered = Vec::new();
    for action in &circuit.actions {
        flatten(action, &mut ordered);
    }
    let [
        Step::Action(StateAction::Assert {
            condition: phase_condition,
            ..
        }),
        Step::Action(StateAction::Assert {
            condition: color, ..
        }),
        Step::Action(amount),
        Step::Binding(secret),
        Step::Binding(nullifier),
        Step::Action(StateAction::Assert {
            condition: unique, ..
        }),
        Step::Action(
            receive @ StateAction::CircuitCall {
                name: receive_name,
                arguments: received,
            },
        ),
        Step::Action(StateAction::CircuitCall {
            name: send_name,
            arguments: sent,
        }),
        Step::Action(StateAction::Expression {
            value:
                Expr::WitnessCall {
                    name: record,
                    arguments: record_arguments,
                },
        }),
        Step::Binding(commitment),
        Step::Action(StateAction::MerkleInsert { value: leaf, .. }),
        Step::Action(StateAction::SetInsert {
            field: set,
            index: set_index,
            value: participant,
        }),
        Step::Action(StateAction::Expression {
            value:
                Expr::WitnessCall {
                    name: advance,
                    arguments: advance_arguments,
                },
        }),
    ] = ordered.as_slice()
    else {
        return None;
    };
    let bindings = [secret, nullifier, commitment];
    if bindings
        .iter()
        .any(|b| b.name == coin.name || b.name == ballot.name)
        || bindings
            .iter()
            .map(|b| &b.name)
            .collect::<HashSet<_>>()
            .len()
            != 3
        || bindings.iter().any(|b| {
            b.ty != (Type::Bytes { length: 32 }) || !observation(&b.value, ledger, pure, circuits)
        })
        || ![phase_condition, color, unique]
            .into_iter()
            .all(|v| observation(v, ledger, pure, circuits))
        || !full_value_assert(amount, &coin.name)
        || !matches!(received.as_slice(), [v] if parameter(v, &coin.name))
        || !matches!(record_arguments.as_slice(), [v] if parameter(v, &ballot.name))
        || !advance_arguments.is_empty()
        || !parameter(leaf, &commitment.name)
        || !parameter(participant, &nullifier.name)
    {
        return None;
    }
    // Require membership guard to protect exactly the subsequently inserted key.
    if !matches!(unique, Expr::If { condition, then, otherwise }
        if matches!(condition.as_ref(), Expr::SetMember { field, index, value }
            if field == set && index == set_index && parameter(value, &nullifier.name))
        && matches!(then.as_ref(), Expr::Boolean { value: false })
        && matches!(otherwise.as_ref(), Expr::Boolean { value: true }))
    {
        return None;
    }
    for (name, parameters) in [
        (record.as_str(), vec![Type::Boolean]),
        (advance.as_str(), vec![]),
    ] {
        let declaration = witnesses.get(name)?;
        if declaration.result != Type::Unit
            || declaration
                .parameters
                .iter()
                .map(|p| p.ty.clone())
                .collect::<Vec<_>>()
                != parameters
        {
            return None;
        }
    }
    let received_helper = circuits.get(receive_name.as_str())?;
    if !shielded_unit_signature(received_helper)
        || !received_helper.actions.iter().all(|a| {
            guarded_deposit::forwards_received_coin(a, &received_helper.parameters[0].name)
        })
        || !shielded_receive_action(receive, pure, circuits, &mut HashSet::new())
    {
        return None;
    }
    let [sent_coin, recipient, sent_amount] = sent.as_slice() else {
        return None;
    };
    if !parameter(sent_coin, &coin.name) || !one(sent_amount) {
        return None;
    }
    immediate_send::audit_bridge(send_name, ledger, witnesses, pure, circuits)?;
    let tail = Expr::Call {
        name: send_name.clone(),
        arguments: sent.clone(),
    };
    if !shielded_send_value(
        &tail,
        pure,
        circuits,
        &mut HashSet::from([circuit.name.clone()]),
    ) {
        return None;
    }
    // The target is a closed pure value. Its exact types and all helper bodies
    // are checked by Plan; no witness, query or hidden intent is permitted here.
    let mut plan = shielded_plan(
        ledger,
        witnesses,
        pure,
        circuits,
        CompositeDomain::VotingCommit,
    );
    if !zero_user_target(recipient, pure) || !plan.pure_value(recipient, &mut HashSet::new()) {
        return None;
    }
    plan.counter_hash_helpers = true;
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
    for action in &circuit.actions {
        plan.action(action, &scope, &mut steps)?;
    }
    (plan.witness_calls == 4
        && plan.cell_reads == 1
        && plan.cell_writes == 0
        && plan.counter_reads == 2
        && plan.scalar_counter_reads == 2
        && plan.counter_writes == 0
        && plan.counter_comparisons == 0
        && plan.tree_writes == 1
        && plan.set_writes == 1
        && plan.qualified_cell_writes == 0
        && plan.qualified_set_writes == 0
        && plan.zswap_inputs == 1
        && plan.zswap_outputs == 3
        && plan.intent_queries == 6
        && plan.kernel_self_reads == 3)
        .then_some(TypedPlan {
            steps,
            result: syn::parse_quote!(()),
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    fn source() -> Value {
        serde_json::from_str(include_str!(
            "../../../tests/micro-dao-vote-commit-schema20-ir.json"
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
        let circuits = source
            .stateful_circuits
            .iter()
            .map(|c| (c.name.as_str(), c))
            .collect();
        lower(
            source
                .stateful_circuits
                .iter()
                .find(|c| c.name.ends_with("vote_commit"))
                .unwrap(),
            &ledger,
            &witnesses,
            &pure,
            &circuits,
        )
        .is_some()
    }
    #[test]
    fn original_vote_commit_is_a_typed_ordered_plan() {
        assert!(admitted(&source()));
    }
    fn root(value: &mut Value) -> &mut Value {
        value["stateful_circuits"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|c| c["name"] == "vote_commit")
            .unwrap()
    }
    fn edit(value: &mut Value, change: &mut impl FnMut(&mut Value) -> bool) -> bool {
        if change(value) {
            return true;
        }
        match value {
            Value::Object(m) => m.values_mut().any(|v| edit(v, change)),
            Value::Array(a) => a.iter_mut().any(|v| edit(v, change)),
            _ => false,
        }
    }
    #[test]
    fn token_provenance_order_and_exact_discarded_amount_are_required() {
        use serde_json::json;
        for case in 0..10 {
            let mut v = source();
            match case {
                0 => {
                    root(&mut v)["actions"][0]["actions"][2]["condition"] =
                        json!({"kind":"boolean","value":true})
                }
                1..=3 => {
                    assert!(edit(root(&mut v), &mut |n| {
                        if n["kind"] == "circuit_call" && n["name"] == "sendImmediateShielded" {
                            n["arguments"][case - 1] = match case {
                                1 => json!({"kind":"parameter","name":"other_coin"}),
                                2 => json!({"kind":"parameter","name":"ballot"}),
                                _ => {
                                    json!({"kind":"unsigned_literal","value":"2","max":u128::MAX.to_string()})
                                }
                            };
                            true
                        } else {
                            false
                        }
                    }));
                }
                4 => {
                    assert!(edit(root(&mut v), &mut |n| {
                        if n["kind"] == "circuit_call" && n["name"] == "receiveShielded" {
                            n["arguments"][0] = json!({"kind":"parameter","name":"other_coin"});
                            true
                        } else {
                            false
                        }
                    }));
                }
                5 => {
                    assert!(edit(root(&mut v), &mut |n| {
                        if n["kind"] == "let" {
                            n["bindings"][0]["name"] = json!("votingCoin");
                            true
                        } else {
                            false
                        }
                    }));
                }
                6 => {
                    assert!(edit(root(&mut v), &mut |n| {
                        if n["kind"] == "sequence" && n["actions"][1]["kind"] == "circuit_call" {
                            n["actions"].as_array_mut().unwrap().swap(1, 2);
                            true
                        } else {
                            false
                        }
                    }));
                }
                7 => {
                    assert!(edit(root(&mut v), &mut |n| {
                        if n["kind"] == "sequence" && n["actions"][0]["kind"] == "merkle_insert" {
                            n["actions"].as_array_mut().unwrap().swap(0, 1);
                            true
                        } else {
                            false
                        }
                    }));
                }
                8 => root(&mut v)["actions"]
                    .as_array_mut()
                    .unwrap()
                    .push(json!({"kind":"expression","value":{"kind":"unit"}})),
                9 => {
                    assert!(edit(root(&mut v), &mut |n| {
                        if n["kind"] == "merkle_insert" {
                            n["value"] = json!({"kind":"parameter","name":"escaped"});
                            true
                        } else {
                            false
                        }
                    }));
                }
                _ => unreachable!(),
            }
            assert!(!admitted(&v), "case {case}");
        }
    }
    #[test]
    fn full_helpers_types_and_unselected_effects_are_audited() {
        use serde_json::json;
        for case in 0..9 {
            let mut v = source();
            match case {
                0 => {
                    v["circuits"]
                        .as_array_mut()
                        .unwrap()
                        .iter_mut()
                        .find(|c| c["name"] == "upcastQualifiedCoin")
                        .unwrap()["body"]["fields"][3]["value"] = json!("1")
                }
                1 => root(&mut v)["parameters"][0]["ty"] = json!({"kind":"field"}),
                2 => root(&mut v)["result"] = json!({"kind":"field"}),
                3 => {
                    v["ledger_fields"]
                        .as_array_mut()
                        .unwrap()
                        .retain(|f| f["id"] != "round");
                }
                4 => {
                    let w = v["witnesses"]
                        .as_array_mut()
                        .unwrap()
                        .iter_mut()
                        .find(|w| w["name"] == "local_record_vote")
                        .unwrap();
                    w["parameters"][0]["ty"] = json!({"kind":"field"});
                }
                5..=7 => {
                    let p = v["circuits"]
                        .as_array_mut()
                        .unwrap()
                        .iter_mut()
                        .find(|c| c["name"] == "coinNullifier")
                        .unwrap();
                    let original = p["body"].take();
                    p["body"] = match case {
                        5 => {
                            json!({"kind":"if","condition":{"kind":"boolean","value":true},"then":original,"otherwise":{"kind":"witness_call","name":"local_secret_key","arguments":[]}})
                        }
                        6 => {
                            json!({"kind":"let","bindings":[{"name":"unused","ty":{"kind":"boolean"},"value":{"kind":"cell_read","field":"state","index":1}}],"body":original})
                        }
                        _ => json!({"kind":"call","name":"coinNullifier","arguments":[]}),
                    };
                }
                8 => {
                    let h = v["stateful_circuits"]
                        .as_array_mut()
                        .unwrap()
                        .iter_mut()
                        .find(|c| c["name"] == "commitment_nullifier")
                        .unwrap();
                    h["return_value"]["value"]["value"]["elements"][1] =
                        json!({"kind":"bytes_literal","bytes":vec![0;32]});
                }
                _ => unreachable!(),
            }
            assert!(!admitted(&v), "case {case}");
        }
    }
    #[test]
    fn declarations_are_data_not_profile_names() {
        fn rename(v: &mut Value) {
            match v {
                Value::Object(m) => {
                    if let Some(Value::String(n)) = m.get_mut("name")
                        && [
                            "vote_commit",
                            "sendImmediateShielded",
                            "receiveShielded",
                            "dao_voting_token",
                            "commit_with_sk",
                        ]
                        .contains(&n.as_str())
                    {
                        n.insert_str(0, "renamed_");
                    }
                    for v in m.values_mut() {
                        rename(v)
                    }
                }
                Value::Array(a) => {
                    for v in a {
                        rename(v)
                    }
                }
                _ => (),
            }
        }
        let mut v = source();
        rename(&mut v);
        assert!(admitted(&v));
    }
}
