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
//! Original two-player funding: two received coins, ordered branch-local
//! writes, and an audited historical-plus-received merge on the second path.
//! The shared Plan owns evaluation, types, scope, and transcript order.
use super::*;

pub(super) fn cell_type(ty: &Type) -> bool {
    matches!(ty, Type::Enum { .. } | Type::Bytes { length: 32 })
        || *ty == crate::stateful::qualified_coin_type()
        || matches!(ty, Type::Struct { fields, .. } if matches!(fields.as_slice(), [member] if member.ty == Type::Field))
}

pub(super) fn witness_type(ty: &Type) -> bool {
    matches!(ty, Type::Unit | Type::Field | Type::Bytes { length: 32 })
}

fn value(
    expression: &Expr,
    pure: &HashMap<&str, &PureCircuit>,
    circuits: &HashMap<&str, &StatefulCircuit>,
) -> bool {
    value_with_reads(expression, pure, circuits, false, true)
}

fn no_merge_value(
    expression: &Expr,
    pure: &HashMap<&str, &PureCircuit>,
    circuits: &HashMap<&str, &StatefulCircuit>,
) -> bool {
    value_with_reads(expression, pure, circuits, false, false)
}

fn predicate(
    expression: &Expr,
    pure: &HashMap<&str, &PureCircuit>,
    circuits: &HashMap<&str, &StatefulCircuit>,
) -> bool {
    value_with_reads(expression, pure, circuits, true, false)
}

fn value_with_reads(
    expression: &Expr,
    pure: &HashMap<&str, &PureCircuit>,
    circuits: &HashMap<&str, &StatefulCircuit>,
    allow_read: bool,
    allow_merge: bool,
) -> bool {
    let visit = |part: &Expr| value_with_reads(part, pure, circuits, allow_read, allow_merge);
    match expression {
        Expr::Parameter { .. }
        | Expr::Boolean { .. }
        | Expr::BytesLiteral { .. }
        | Expr::FieldLiteral { .. }
        | Expr::UnsignedLiteral { .. }
        | Expr::EnumVariant { .. }
        | Expr::Unit
        | Expr::Default {
            ty: Type::Struct { .. },
        } => true,
        Expr::CellRead { .. } => allow_read,
        Expr::KernelSelf { ty } => *ty == contract_address_type(),
        Expr::StructLiteral { fields, .. } | Expr::Tuple { elements: fields } => {
            fields.iter().all(visit)
        }
        Expr::StructField { value: part, .. } | Expr::Coerce { value: part, .. } => visit(part),
        Expr::Equal { left, right } => visit(left) && visit(right),
        Expr::If {
            condition,
            then,
            otherwise,
        } => [condition, then, otherwise].iter().all(|part| visit(part)),
        Expr::Let { bindings, body } => bindings.iter().all(|b| visit(&b.value)) && visit(body),
        Expr::WitnessCall { arguments, .. } => arguments.iter().all(visit),
        Expr::Call { name, arguments } => {
            if pure.contains_key(name.as_str()) {
                return !circuits.contains_key(name.as_str()) && arguments.iter().all(visit);
            }
            let [left, right] = arguments.as_slice() else {
                return false;
            };
            allow_merge
                && matches!(immediate_send::uncoerced(left), Expr::CellRead { .. })
                && matches!(immediate_send::uncoerced(right), Expr::Parameter { .. })
                && shielded_merge::received_helper(name, pure, circuits)
                && circuits.get(name.as_str()).is_some_and(|callee| {
                    matches!(&callee.return_value, StateReturn::Expression { value: tail }
                        if shielded_value(tail, pure, circuits, &mut HashSet::from([name.clone()]), CompositeDomain::ShieldedMerge(shielded_merge::Inputs::ReceivedRight)))
                })
        }
        // Intent creation is admitted only inside the audited receive/merge
        // helpers. A root binding, argument, or unselected branch cannot hide it.
        _ => false,
    }
}

fn action(
    action: &StateAction,
    pure: &HashMap<&str, &PureCircuit>,
    circuits: &HashMap<&str, &StatefulCircuit>,
) -> bool {
    match action {
        StateAction::Sequence { actions } => {
            actions.iter().all(|a| self::action(a, pure, circuits))
        }
        StateAction::Let { bindings, action } => {
            bindings.iter().all(|b| value(&b.value, pure, circuits))
                && self::action(action, pure, circuits)
        }
        StateAction::If {
            condition,
            then,
            otherwise,
        } => {
            predicate(condition, pure, circuits)
                && self::action(then, pure, circuits)
                && self::action(otherwise, pure, circuits)
        }
        StateAction::Assert { condition, .. } => predicate(condition, pure, circuits),
        StateAction::CellWrite { value: written, .. } => value(written, pure, circuits),
        StateAction::CellWriteCoin {
            coin, recipient, ..
        } => value(coin, pure, circuits) && value(recipient, pure, circuits),
        StateAction::Expression {
            value: witness @ Expr::WitnessCall { .. },
        } => value(witness, pure, circuits),
        StateAction::CircuitCall { name, arguments } => {
            let Some(callee) = circuits.get(name.as_str()) else {
                return false;
            };
            shielded_unit_signature(callee)
                && callee
                    .actions
                    .iter()
                    .all(|a| guarded_deposit::forwards_received_coin(a, &callee.parameters[0].name))
                && arguments.iter().all(|v| value(v, pure, circuits))
                && shielded_receive_action(action, pure, circuits, &mut HashSet::new())
        }
        // All other writes/queries/intents require their own domain.
        _ => false,
    }
}

fn body(
    return_body: &ReturnPlan,
    pure: &HashMap<&str, &PureCircuit>,
    circuits: &HashMap<&str, &StatefulCircuit>,
) -> bool {
    match return_body {
        ReturnPlan::Value { value: result } => value(result, pure, circuits),
        ReturnPlan::Sequence { actions, result } => {
            actions.iter().all(|a| action(a, pure, circuits)) && body(result, pure, circuits)
        }
        ReturnPlan::Let { bindings, result } => {
            bindings.iter().all(|b| value(&b.value, pure, circuits)) && body(result, pure, circuits)
        }
        ReturnPlan::Conditional {
            condition,
            then,
            otherwise,
        } => {
            predicate(condition, pure, circuits)
                && body(then, pure, circuits)
                && body(otherwise, pure, circuits)
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CoinOrigin {
    Wager,
    Deposit,
    Merged { historical: u8 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BranchStep {
    Merge { historical: u8 },
    Coin { index: u8, origin: CoinOrigin },
    Key,
    Board,
    Phase { index: u8 },
    Witness,
}

fn uncoerced(value: &Expr) -> &Expr {
    immediate_send::uncoerced(value)
}

fn coin_origin(
    value: &Expr,
    scope: &HashMap<String, CoinOrigin>,
    wager: &str,
    deposit: &str,
    pure: &HashMap<&str, &PureCircuit>,
    circuits: &HashMap<&str, &StatefulCircuit>,
) -> Option<CoinOrigin> {
    match uncoerced(value) {
        Expr::Parameter { name } if name == wager => Some(CoinOrigin::Wager),
        Expr::Parameter { name } if name == deposit => Some(CoinOrigin::Deposit),
        Expr::Parameter { name } => scope.get(name).copied(),
        Expr::Call { name, arguments } if shielded_merge::received_helper(name, pure, circuits) => {
            let [left, right] = arguments.as_slice() else {
                return None;
            };
            let Expr::CellRead { index, .. } = uncoerced(left) else {
                return None;
            };
            if !immediate_send::parameter(right, wager) {
                return None;
            }
            Some(CoinOrigin::Merged { historical: *index })
        }
        _ => None,
    }
}

fn no_funding_call(
    action: &StateAction,
    funding_names: &[&str; 3],
    pure: &HashMap<&str, &PureCircuit>,
    circuits: &HashMap<&str, &StatefulCircuit>,
) -> bool {
    match action {
        StateAction::Sequence { actions } => actions
            .iter()
            .all(|a| no_funding_call(a, funding_names, pure, circuits)),
        StateAction::If {
            then, otherwise, ..
        } => {
            no_funding_call(then, funding_names, pure, circuits)
                && no_funding_call(otherwise, funding_names, pure, circuits)
        }
        StateAction::Let { bindings, action } => {
            bindings.iter().all(|b| {
                !funding_names.contains(&b.name.as_str())
                    && no_merge_value(&b.value, pure, circuits)
            }) && no_funding_call(action, funding_names, pure, circuits)
        }
        StateAction::CircuitCall { .. }
        | StateAction::CellWriteCoin { .. }
        | StateAction::CellWrite { .. } => false,
        StateAction::Assert { condition, .. } => predicate(condition, pure, circuits),
        StateAction::Expression { value } => no_merge_value(value, pure, circuits),
        _ => false,
    }
}

fn branch_action(
    action: &StateAction,
    scope: &HashMap<String, CoinOrigin>,
    funding_names: &[&str; 3],
    ledger: &HashMap<&str, &LedgerField>,
    pure: &HashMap<&str, &PureCircuit>,
    circuits: &HashMap<&str, &StatefulCircuit>,
    steps: &mut Vec<BranchStep>,
) -> bool {
    match action {
        StateAction::Sequence { actions } => actions
            .iter()
            .all(|a| branch_action(a, scope, funding_names, ledger, pure, circuits, steps)),
        StateAction::Let { bindings, action } => {
            let mut scoped = scope.clone();
            for binding in bindings {
                if funding_names.contains(&binding.name.as_str()) {
                    return false;
                }
                if binding.ty == crate::stateful::shielded_coin_type() {
                    let Some(origin) = coin_origin(
                        &binding.value,
                        &scoped,
                        funding_names[1],
                        funding_names[2],
                        pure,
                        circuits,
                    ) else {
                        return false;
                    };
                    scoped.insert(binding.name.clone(), origin);
                    if matches!(uncoerced(&binding.value), Expr::Call { .. }) {
                        let CoinOrigin::Merged { historical } = origin else {
                            return false;
                        };
                        steps.push(BranchStep::Merge { historical });
                    }
                } else if !no_merge_value(&binding.value, pure, circuits) {
                    return false;
                }
            }
            branch_action(
                action,
                &scoped,
                funding_names,
                ledger,
                pure,
                circuits,
                steps,
            )
        }
        StateAction::CellWriteCoin {
            index,
            coin,
            recipient,
            ..
        } => {
            if !no_merge_value(coin, pure, circuits) || !no_merge_value(recipient, pure, circuits) {
                return false;
            }
            let Some(origin) = coin_origin(
                coin,
                scope,
                funding_names[1],
                funding_names[2],
                pure,
                circuits,
            ) else {
                return false;
            };
            steps.push(BranchStep::Coin {
                index: *index,
                origin,
            });
            true
        }
        StateAction::CellWrite {
            field,
            index,
            value: written,
        } => {
            if !no_merge_value(written, pure, circuits) {
                return false;
            }
            let Some(declaration) = ledger.get(field.as_str()) else {
                return false;
            };
            if declaration.index != *index {
                return false;
            }
            let LedgerFieldKind::Cell { ty } = &declaration.declaration else {
                return false;
            };
            let step = match ty {
                Type::Bytes { length: 32 } => BranchStep::Key,
                Type::Enum { .. } => BranchStep::Phase { index: *index },
                Type::Struct { fields, .. } if matches!(fields.as_slice(), [member] if member.ty == Type::Field) => {
                    BranchStep::Board
                }
                _ => return false,
            };
            steps.push(step);
            true
        }
        StateAction::Expression {
            value: Expr::WitnessCall { .. },
        } => {
            steps.push(BranchStep::Witness);
            true
        }
        // The branch may assert before writing, but cannot receive, write an
        // unrelated ADT, or move a funded effect behind another conditional.
        StateAction::Assert { condition, .. } => predicate(condition, pure, circuits),
        _ => false,
    }
}

fn branch(
    body: &ReturnPlan,
    funding_names: &[&str; 3],
    ledger: &HashMap<&str, &LedgerField>,
    pure: &HashMap<&str, &PureCircuit>,
    circuits: &HashMap<&str, &StatefulCircuit>,
) -> Option<Vec<BranchStep>> {
    let mut steps = Vec::new();
    fn walk(
        body: &ReturnPlan,
        scope: &HashMap<String, CoinOrigin>,
        funding_names: &[&str; 3],
        ledger: &HashMap<&str, &LedgerField>,
        pure: &HashMap<&str, &PureCircuit>,
        circuits: &HashMap<&str, &StatefulCircuit>,
        steps: &mut Vec<BranchStep>,
    ) -> bool {
        match body {
            ReturnPlan::Sequence { actions, result } => {
                actions
                    .iter()
                    .all(|a| branch_action(a, scope, funding_names, ledger, pure, circuits, steps))
                    && walk(result, scope, funding_names, ledger, pure, circuits, steps)
            }
            ReturnPlan::Let { bindings, result } => {
                let mut scoped = scope.clone();
                for binding in bindings {
                    if funding_names.contains(&binding.name.as_str()) {
                        return false;
                    }
                    if binding.ty == crate::stateful::shielded_coin_type() {
                        let Some(origin) = coin_origin(
                            &binding.value,
                            &scoped,
                            funding_names[1],
                            funding_names[2],
                            pure,
                            circuits,
                        ) else {
                            return false;
                        };
                        scoped.insert(binding.name.clone(), origin);
                        if matches!(uncoerced(&binding.value), Expr::Call { .. }) {
                            let CoinOrigin::Merged { historical } = origin else {
                                return false;
                            };
                            steps.push(BranchStep::Merge { historical });
                        }
                    } else if !no_merge_value(&binding.value, pure, circuits) {
                        return false;
                    }
                }
                walk(
                    result,
                    &scoped,
                    funding_names,
                    ledger,
                    pure,
                    circuits,
                    steps,
                )
            }
            ReturnPlan::Value { value: result } => no_merge_value(result, pure, circuits),
            ReturnPlan::Conditional { .. } => false,
        }
    }
    walk(
        body,
        &HashMap::new(),
        funding_names,
        ledger,
        pure,
        circuits,
        &mut steps,
    )
    .then_some(steps)
}

fn funding_paths(
    body: &ReturnPlan,
    funding_names: &[&str; 3],
    ledger: &HashMap<&str, &LedgerField>,
    pure: &HashMap<&str, &PureCircuit>,
    circuits: &HashMap<&str, &StatefulCircuit>,
) -> bool {
    fn prefix<'a>(
        body: &'a ReturnPlan,
        funding_names: &[&str; 3],
        receives: &mut Vec<String>,
        pure: &HashMap<&str, &PureCircuit>,
        circuits: &HashMap<&str, &StatefulCircuit>,
    ) -> Option<(&'a ReturnPlan, &'a ReturnPlan)> {
        match body {
            ReturnPlan::Sequence { actions, result } => {
                for action in actions {
                    if let StateAction::CircuitCall { arguments, .. } = action {
                        let [coin] = arguments.as_slice() else {
                            return None;
                        };
                        let Expr::Parameter { name } = uncoerced(coin) else {
                            return None;
                        };
                        if receives.len() >= 2 || name != funding_names[receives.len() + 1] {
                            return None;
                        }
                        receives.push(name.clone());
                    } else if !no_funding_call(action, funding_names, pure, circuits) {
                        return None;
                    }
                }
                prefix(result, funding_names, receives, pure, circuits)
            }
            ReturnPlan::Let { bindings, result } => {
                if bindings.iter().any(|b| {
                    funding_names.contains(&b.name.as_str())
                        || !no_merge_value(&b.value, pure, circuits)
                }) {
                    return None;
                }
                prefix(result, funding_names, receives, pure, circuits)
            }
            ReturnPlan::Conditional {
                then, otherwise, ..
            } if receives.len() == 2 => Some((then, otherwise)),
            _ => None,
        }
    }
    let Some((first, second)) = prefix(body, funding_names, &mut Vec::new(), pure, circuits) else {
        return false;
    };
    let Some(first) = branch(first, funding_names, ledger, pure, circuits) else {
        return false;
    };
    let Some(second) = branch(second, funding_names, ledger, pure, circuits) else {
        return false;
    };
    let parse = |steps: &[BranchStep]| {
        let (merge, tail) = match steps.split_first() {
            Some((BranchStep::Merge { historical }, tail)) => (Some(*historical), tail),
            _ => (None, steps),
        };
        let [
            BranchStep::Coin { index: pot, origin },
            BranchStep::Coin {
                index: deposit,
                origin: CoinOrigin::Deposit,
            },
            BranchStep::Key,
            BranchStep::Board,
            BranchStep::Phase { .. },
            BranchStep::Witness,
        ] = tail
        else {
            return None;
        };
        match (merge, origin) {
            (None, CoinOrigin::Wager) | (Some(_), CoinOrigin::Merged { .. }) => {}
            _ => return None,
        }
        if matches!((merge, origin), (Some(merge_index), CoinOrigin::Merged { historical }) if merge_index != *historical)
        {
            return None;
        }
        Some((*pot, *deposit, *origin))
    };
    let (
        Some((first_pot, first_deposit, first_origin)),
        Some((second_pot, second_deposit, second_origin)),
    ) = (parse(&first), parse(&second))
    else {
        return false;
    };
    if !matches!((first_origin, second_origin),
        (CoinOrigin::Wager, CoinOrigin::Merged { historical })
            | (CoinOrigin::Merged { historical }, CoinOrigin::Wager) if historical == first_pot)
    {
        return false;
    }
    first_pot == second_pot
        && first_deposit != second_deposit
        && first_deposit != first_pot
        && second_deposit != first_pot
}

pub(super) fn lower<'a>(
    circuit: &StatefulCircuit,
    ledger: &'a HashMap<&'a str, &'a LedgerField>,
    witnesses: &'a HashMap<&'a str, &'a WitnessDeclaration>,
    pure: &'a HashMap<&'a str, &'a PureCircuit>,
    circuits: &'a HashMap<&'a str, &'a StatefulCircuit>,
) -> Option<TypedPlan> {
    let StateReturn::Effectful { body: return_body } = &circuit.return_value else {
        return None;
    };
    if !circuit.actions.is_empty()
        || !matches!(circuit.result, Type::Enum { .. })
        || !matches!(circuit.parameters.as_slice(), [position, wager, deposit]
            if position.ty == Type::Field
                && wager.ty == crate::stateful::shielded_coin_type()
                && deposit.ty == wager.ty)
        || circuit
            .parameters
            .iter()
            .map(|p| &p.name)
            .collect::<HashSet<_>>()
            .len()
            != circuit.parameters.len()
        || !body(return_body, pure, circuits)
        || !funding_paths(
            return_body,
            &[
                &circuit.parameters[0].name,
                &circuit.parameters[1].name,
                &circuit.parameters[2].name,
            ],
            ledger,
            pure,
            circuits,
        )
    {
        return None;
    }
    let mut plan = shielded_plan(
        ledger,
        witnesses,
        pure,
        circuits,
        CompositeDomain::StartFunding,
    );
    let scope = circuit
        .parameters
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let name = syn::Ident::new(&format!("__compact_param_{i}"), Span::call_site());
            (
                p.name.clone(),
                TypedValue {
                    ty: p.ty.clone(),
                    value: syn::parse_quote!(#name),
                },
            )
        })
        .collect();
    let mut steps = Vec::new();
    let result = plan.return_plan(return_body, &scope, &mut steps);
    let result = result?;
    (result.ty == circuit.result
        && plan.zswap_inputs == 2
        && plan.zswap_outputs == 3
        && plan.qualified_cell_writes == 4
        && plan.cell_writes == 6
        && plan.witness_calls == 4
        && plan.counter_reads == 0
        && plan.counter_comparisons == 0
        && plan.counter_writes == 0
        && plan.root_observations == 0
        && plan.tree_writes == 0
        && plan.set_writes == 0
        && plan.qualified_set_reads == 0
        && plan.qualified_set_writes == 0
        && plan.historic_roots == 0
        && plan.historic_writes == 0)
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
            "../../../tests/coracle-start-schema20-ir.json"
        ))
        .unwrap()
    }

    fn admitted(value: &Value, name: &str) -> bool {
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
        let circuits: HashMap<_, _> = source
            .stateful_circuits
            .iter()
            .map(|c| (c.name.as_str(), c))
            .collect();
        lower(
            circuits.get(name).unwrap(),
            &ledger,
            &witnesses,
            &pure,
            &circuits,
        )
        .is_some()
    }

    fn visit_kind(value: &mut Value, kind: &str, change: &mut impl FnMut(&mut Value)) {
        match value {
            Value::Object(fields) => {
                if fields.get("kind").and_then(Value::as_str) == Some(kind) {
                    change(value);
                }
                if let Value::Object(fields) = value {
                    for child in fields.values_mut() {
                        visit_kind(child, kind, change);
                    }
                }
            }
            Value::Array(values) => {
                for child in values {
                    visit_kind(child, kind, change);
                }
            }
            _ => (),
        }
    }

    #[test]
    fn original_start_is_admitted() {
        assert!(admitted(&source(), "start"));
        let mut renamed = source();
        for (from, to) in [
            ("start", "renamed_start"),
            ("receiveShielded", "renamed_receive"),
            ("mergeCoinImmediate", "renamed_immediate_merge"),
            ("mergeCoin", "renamed_merge"),
        ] {
            fn rename(value: &mut Value, from: &str, to: &str) {
                match value {
                    Value::Object(fields) => {
                        if fields.get("name").and_then(Value::as_str) == Some(from) {
                            fields.insert("name".into(), json!(to));
                        }
                        for child in fields.values_mut() {
                            rename(child, from, to);
                        }
                    }
                    Value::Array(values) => {
                        for child in values {
                            rename(child, from, to);
                        }
                    }
                    _ => (),
                }
            }
            rename(&mut renamed, from, to);
        }
        assert!(admitted(&renamed, "renamed_start"));
    }

    #[test]
    fn hidden_queries_and_extra_intents_are_rejected() {
        for hidden in [
            json!({"name":"unused", "ty":{"kind":"enum","name":"State","variants":["no_game","red_started","blue_started","red_turn","blue_turn","red_wins","blue_wins"]}, "value":{"kind":"cell_read","field":"state","index":5}}),
            json!({"name":"unused", "ty":{"kind":"boolean"}, "value":{"kind":"set_member","field":"hidden","index":0,"value":{"kind":"bytes_literal","bytes":vec![0u8;32]}}}),
        ] {
            let mut value = source();
            value["stateful_circuits"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|c| c["name"] == "start")
                .unwrap()["return_value"]["body"]["result"]["bindings"]
                .as_array_mut()
                .unwrap()
                .push(hidden);
            assert!(!admitted(&value, "start"));
        }
        let mut extra = source();
        let prefix = &mut extra["stateful_circuits"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|c| c["name"] == "start")
            .unwrap()["return_value"]["body"]["result"]["result"]["actions"];
        let second_receive = prefix[2].clone();
        prefix.as_array_mut().unwrap().push(second_receive);
        assert!(!admitted(&extra, "start"));
    }

    #[test]
    fn funded_coin_provenance_is_per_path_and_shadow_safe() {
        let mut changed_forward = source();
        let receive = changed_forward["stateful_circuits"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|c| c["name"] == "receiveShielded")
            .unwrap();
        let mut changed = false;
        visit_kind(
            &mut receive["actions"],
            "create_zswap_output",
            &mut |node| {
                node["coin"] =
                    json!({"kind":"default", "ty":crate::stateful::shielded_coin_type()});
                changed = true;
            },
        );
        assert!(changed && !admitted(&changed_forward, "start"));

        let mut same_receive = source();
        let start = same_receive["stateful_circuits"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|c| c["name"] == "start")
            .unwrap();
        let calls = &mut start["return_value"]["body"]["result"]["result"]["actions"];
        calls[2]["arguments"][0]["value"]["name"] = json!("wager");
        assert!(!admitted(&same_receive, "start"));

        let mut shadow = source();
        let start = shadow["stateful_circuits"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|c| c["name"] == "start")
            .unwrap();
        start["return_value"]["body"]["result"]["bindings"]
            .as_array_mut()
            .unwrap()
            .push(
                json!({"name":"wager", "ty":crate::stateful::shielded_coin_type(),
                "value":{"kind":"parameter","name":"deposit"}}),
            );
        assert!(!admitted(&shadow, "start"));

        let mut wrong_red_store = source();
        let start = wrong_red_store["stateful_circuits"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|c| c["name"] == "start")
            .unwrap();
        let mut changed = false;
        visit_kind(&mut start["return_value"], "cell_write_coin", &mut |node| {
            if !changed {
                node["coin"]["name"] = json!("deposit");
                changed = true;
            }
        });
        assert!(changed && !admitted(&wrong_red_store, "start"));

        let mut wrong_merge_right = source();
        let start = wrong_merge_right["stateful_circuits"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|c| c["name"] == "start")
            .unwrap();
        let mut changed = false;
        visit_kind(&mut start["return_value"], "call", &mut |node| {
            if node["name"] == "mergeCoinImmediate" {
                node["arguments"][1]["value"]["name"] = json!("deposit");
                changed = true;
            }
        });
        assert!(changed && !admitted(&wrong_merge_right, "start"));

        let mut wrong_historical = source();
        let start = wrong_historical["stateful_circuits"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|c| c["name"] == "start")
            .unwrap();
        let mut changed = false;
        visit_kind(&mut start["return_value"], "call", &mut |node| {
            if node["name"] == "mergeCoinImmediate" {
                node["arguments"][0]["value"]["field"] = json!("red_deposit");
                node["arguments"][0]["value"]["index"] = json!(7);
                changed = true;
            }
        });
        assert!(changed && !admitted(&wrong_historical, "start"));

        let mut relocated_receive = source();
        let start = relocated_receive["stateful_circuits"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|c| c["name"] == "start")
            .unwrap();
        let sequence = &mut start["return_value"]["body"]["result"]["result"];
        let moved = sequence["actions"].as_array_mut().unwrap().remove(2);
        sequence["result"]["result"]["result"]["result"]["then"]["actions"]
            .as_array_mut()
            .unwrap()
            .insert(0, moved);
        assert!(!admitted(&relocated_receive, "start"));

        let mut premature_store = source();
        let start = premature_store["stateful_circuits"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|c| c["name"] == "start")
            .unwrap();
        let sequence = &mut start["return_value"]["body"]["result"]["result"];
        let moved = sequence["result"]["result"]["result"]["result"]["then"]["actions"]
            .as_array_mut()
            .unwrap()
            .remove(4);
        sequence["actions"].as_array_mut().unwrap().insert(0, moved);
        assert!(!admitted(&premature_store, "start"));

        // Same merge expression and static node count, but evaluated before
        // either wallet-funded receive instead of on the selected blue path.
        let mut premature_merge = source();
        let start = premature_merge["stateful_circuits"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|c| c["name"] == "start")
            .unwrap();
        let outer = &mut start["return_value"]["body"]["result"];
        let merged = outer["result"]["result"]["result"]["result"]["result"]
            ["otherwise"]["actions"][0]["bindings"]
            .as_array_mut().unwrap().remove(0);
        outer["bindings"].as_array_mut().unwrap().push(merged);
        assert!(!admitted(&premature_merge, "start"));
    }

    #[test]
    fn malformed_types_helpers_and_branch_results_are_rejected() {
        let mut wrong_slot = source();
        let start = wrong_slot["stateful_circuits"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|c| c["name"] == "start")
            .unwrap();
        let mut changed = false;
        visit_kind(&mut start["return_value"], "cell_write_coin", &mut |node| {
            if !changed {
                node["field"] = json!("state");
                node["index"] = json!(5);
                changed = true;
            }
        });
        assert!(changed && !admitted(&wrong_slot, "start"));
        let mut wrong_witness = source();
        wrong_witness["witnesses"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|w| w["name"] == "fresh_nonce")
            .unwrap()["result"] = json!({"kind":"boolean"});
        assert!(!admitted(&wrong_witness, "start"));
        let mut recursive = source();
        let helper = recursive["circuits"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|c| c["name"] == "coinCommitment")
            .unwrap();
        helper["body"] = json!({"kind":"call","name":"coinCommitment","arguments":[]});
        assert!(!admitted(&recursive, "start"));
        let mut hidden_helper_read = source();
        let helper = hidden_helper_read["stateful_circuits"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|c| c["name"] == "mergeCoin")
            .unwrap();
        let original = helper["return_value"]["value"].take();
        helper["return_value"]["value"] = json!({"kind":"let","bindings":[
            {"name":"unused","ty":{"kind":"enum","name":"State","variants":["no_game","red_started","blue_started","red_turn","blue_turn","red_wins","blue_wins"]},
             "value":{"kind":"cell_read","field":"state","index":5}}],"body":original});
        assert!(!admitted(&hidden_helper_read, "start"));
        let mut mismatched = source();
        let start = mismatched["stateful_circuits"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|c| c["name"] == "start")
            .unwrap();
        let mut changed = false;
        visit_kind(&mut start["return_value"], "conditional", &mut |node| {
            if !changed {
                node["otherwise"] =
                    json!({"kind":"value","value":{"kind":"field_literal","value":"1"}});
                changed = true;
            }
        });
        assert!(changed && !admitted(&mismatched, "start"));
    }
}
