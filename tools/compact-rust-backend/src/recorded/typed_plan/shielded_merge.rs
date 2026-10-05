// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Bounded two-input shielded merge policy. Evaluation and helper scope belong
//! to the shared Plan; this module distinguishes historical and received input
//! provenance and audits the complete reachable expression domain.
use super::immediate_send::{parameter, singleton_bridge, uncoerced};
use super::*;
use crate::ir::Parameter;

pub(super) const INPUT: &str = "340282366920938463463374607431768211455";
pub(super) const WIDENED: &str = "680564733841876926926749214863536422910";
pub(super) const SUM: &str = "680564733841876926926749214863536422911";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Inputs {
    HistoricalPair,
    ReceivedRight,
}

// An immediate wrapper must forward its historical left input unchanged and
// qualify exactly the received right coin using the audited singleton bridge.
fn received_bridge(
    value: &Expr,
    left: &Parameter,
    right: &Parameter,
    pure: &HashMap<&str, &PureCircuit>,
    circuits: &HashMap<&str, &StatefulCircuit>,
) -> bool {
    let Expr::Call { name, arguments } = value else {
        return false;
    };
    if !matches!(arguments.as_slice(), [a,b] if parameter(a,&left.name) && parameter(b,&right.name))
        || pure.contains_key(name.as_str())
    {
        return false;
    }
    let Some(bridge) = circuits.get(name.as_str()) else {
        return false;
    };
    let [a, b] = bridge.parameters.as_slice() else {
        return false;
    };
    if !bridge.actions.is_empty()
        || a.ty != left.ty
        || b.ty != right.ty
        || bridge.result != crate::stateful::shielded_coin_type()
    {
        return false;
    }
    let StateReturn::Expression {
        value: Expr::Call { name, arguments },
    } = &bridge.return_value
    else {
        return false;
    };
    let [first, second] = arguments.as_slice() else {
        return false;
    };
    let Some(merge) = circuits.get(name.as_str()) else {
        return false;
    };
    if pure.contains_key(name.as_str())
        || !parameter(first, &a.name)
        || merge.parameters.len() != 2
        || merge
            .parameters
            .iter()
            .any(|p| p.ty != crate::stateful::qualified_coin_type())
        || merge.result != bridge.result
    {
        return false;
    }
    let Expr::Call { name, arguments } = uncoerced(second) else {
        return false;
    };
    matches!(arguments.as_slice(), [value] if parameter(value,&b.name))
        && !circuits.contains_key(name.as_str())
        && pure
            .get(name.as_str())
            .is_some_and(|callee| singleton_bridge(callee))
}

// Reuse the same exact declaration audit when a caller supplies a Cell read
// as its historical operand. Actual caller provenance is checked by its domain.
pub(super) fn received_helper(
    name: &str,
    pure: &HashMap<&str, &PureCircuit>,
    circuits: &HashMap<&str, &StatefulCircuit>,
) -> bool {
    let Some(callee) = circuits.get(name) else {
        return false;
    };
    let [left, right] = callee.parameters.as_slice() else {
        return false;
    };
    left.ty == crate::stateful::qualified_coin_type()
        && right.ty == crate::stateful::shielded_coin_type()
        && received_bridge(
            &Expr::Call {
                name: name.to_owned(),
                arguments: vec![
                    Expr::Parameter {
                        name: left.name.clone(),
                    },
                    Expr::Parameter {
                        name: right.name.clone(),
                    },
                ],
            },
            left,
            right,
            pure,
            circuits,
        )
}

pub(super) fn lower<'a>(
    circuit: &StatefulCircuit,
    ledger: &'a HashMap<&'a str, &'a LedgerField>,
    witnesses: &'a HashMap<&'a str, &'a WitnessDeclaration>,
    pure: &'a HashMap<&'a str, &'a PureCircuit>,
    circuits: &'a HashMap<&'a str, &'a StatefulCircuit>,
) -> Option<TypedPlan> {
    let [left, right] = circuit.parameters.as_slice() else {
        return None;
    };
    let StateReturn::Expression { value } = &circuit.return_value else {
        return None;
    };
    if left.name == right.name
        || left.ty != crate::stateful::qualified_coin_type()
        || circuit.result != crate::stateful::shielded_coin_type()
    {
        return None;
    }
    let inputs = if right.ty == left.ty && circuit.actions.is_empty() {
        Inputs::HistoricalPair
    } else if right.ty == crate::stateful::shielded_coin_type() {
        let [prefix @ StateAction::CircuitCall { arguments, .. }] = circuit.actions.as_slice()
        else {
            return None;
        };
        if !matches!(arguments.as_slice(), [value] if parameter(value,&right.name))
            || !shielded_receive_action(
                prefix,
                pure,
                circuits,
                &mut HashSet::from([circuit.name.clone()]),
            )
            || !received_bridge(value, left, right, pure, circuits)
        {
            return None;
        }
        Inputs::ReceivedRight
    } else {
        return None;
    };
    let domain = CompositeDomain::ShieldedMerge(inputs);
    if !shielded_value(
        value,
        pure,
        circuits,
        &mut HashSet::from([circuit.name.clone()]),
        domain,
    ) {
        return None;
    }
    let mut plan = shielded_plan(ledger, witnesses, pure, circuits, domain);
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
    let received = usize::from(inputs == Inputs::ReceivedRight);
    if plan.kernel_self_reads != received
        || plan.zswap_outputs != received
        || plan.zswap_inputs != 0
        || plan.intent_queries != received
    {
        return None;
    }
    let result = plan.expression(value, &scope, &mut steps)?;
    (result.ty == circuit.result
        && plan.kernel_self_reads == 1 + received
        && plan.zswap_inputs == 2
        && plan.zswap_outputs == 1 + received
        && plan.intent_queries == 4 + received
        && plan.witness_calls == 0
        && plan.cell_reads == 0
        && plan.cell_writes == 0
        && plan.counter_reads == 0
        && plan.counter_comparisons == 0
        && plan.counter_writes == 0
        && plan.tree_writes == 0
        && plan.set_writes == 0)
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
            "../../../tests/shielded-merge-schema20-ir.json"
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
        let circuits = source
            .stateful_circuits
            .iter()
            .map(|c| (c.name.as_str(), c))
            .collect();
        lower(
            source
                .stateful_circuits
                .iter()
                .find(|c| c.name == name)
                .unwrap(),
            &ledger,
            &witnesses,
            &pure,
            &circuits,
        )
        .is_some()
    }
    fn stateful<'a>(value: &'a mut Value, name: &str) -> &'a mut Value {
        value["stateful_circuits"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|c| c["name"] == name)
            .unwrap()
    }
    fn pure<'a>(value: &'a mut Value, name: &str) -> &'a mut Value {
        value["circuits"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|c| c["name"] == name)
            .unwrap()
    }
    fn change_kind(value: &mut Value, kind: &str, f: &mut impl FnMut(&mut Value)) {
        match value {
            Value::Object(_) => {
                if value["kind"] == kind {
                    f(value);
                }
                for child in value.as_object_mut().unwrap().values_mut() {
                    change_kind(child, kind, f);
                }
            }
            Value::Array(values) => {
                for value in values {
                    change_kind(value, kind, f);
                }
            }
            _ => (),
        }
    }
    #[test]
    fn qualified_and_received_domains_are_structural_and_distinct() {
        let mut value = source();
        for name in ["merge_qualified", "receive_then_merge"] {
            assert!(admitted(&value, name));
        }
        fn rename(value: &mut Value) {
            match value {
                Value::Object(fields) => {
                    if let Some(Value::String(name)) = fields.get_mut("name")
                        && [
                            "merge_qualified",
                            "receive_then_merge",
                            "mergeCoin",
                            "mergeCoinImmediate",
                            "upcastQualifiedCoin",
                            "receiveShielded",
                        ]
                        .contains(&name.as_str())
                    {
                        *name = format!("renamed_{name}");
                    }
                    for value in fields.values_mut() {
                        rename(value);
                    }
                }
                Value::Array(values) => {
                    for value in values {
                        rename(value);
                    }
                }
                _ => (),
            }
        }
        rename(&mut value);
        assert!(admitted(&value, "renamed_merge_qualified"));
        assert!(admitted(&value, "renamed_receive_then_merge"));
        let mut no_receive = source();
        stateful(&mut no_receive, "receive_then_merge")["actions"] = json!([]);
        assert!(!admitted(&no_receive, "receive_then_merge"));
    }
    #[test]
    fn exact_wide_bounds_and_unsigned_operations_are_closed() {
        for kind in ["unsigned_add", "unsigned_cast"] {
            let mut value = source();
            change_kind(stateful(&mut value, "mergeCoin"), kind, &mut |part| {
                part["max"] = json!("10");
            });
            assert!(!admitted(&value, "merge_qualified"));
            assert!(!admitted(&value, "receive_then_merge"));
        }
        for kind in ["unsigned_subtract", "unsigned_multiply"] {
            let mut value = source();
            change_kind(
                stateful(&mut value, "mergeCoin"),
                "unsigned_add",
                &mut |part| {
                    part["kind"] = json!(kind);
                },
            );
            assert!(!admitted(&value, "merge_qualified"));
        }
    }
    #[test]
    fn received_identity_singleton_and_prefix_are_checked() {
        for field in 0..4 {
            let mut value = source();
            pure(&mut value, "upcastQualifiedCoin")["body"]["fields"][field] = if field == 3 {
                json!({"kind":"unsigned_literal","value":"1","max":u64::MAX.to_string()})
            } else {
                json!({"kind":"parameter","name":"missing"})
            };
            assert!(!admitted(&value, "receive_then_merge"));
        }
        for arg in 0..2 {
            let mut value = source();
            stateful(&mut value, "receive_then_merge")["return_value"]["value"]["arguments"][arg] =
                json!({"kind":"parameter","name":"different"});
            assert!(!admitted(&value, "receive_then_merge"));
        }
        let mut value = source();
        stateful(&mut value, "receive_then_merge")["actions"][0]["arguments"][0] =
            json!({"kind":"parameter","name":"a"});
        assert!(!admitted(&value, "receive_then_merge"));
        let mut value = source();
        let prefix = stateful(&mut value, "receive_then_merge")["actions"][0].clone();
        stateful(&mut value, "receive_then_merge")["actions"]
            .as_array_mut()
            .unwrap()
            .push(prefix);
        assert!(!admitted(&value, "receive_then_merge"));
    }
    #[test]
    fn whole_call_graph_hidden_effects_and_scope_are_audited() {
        for hidden in [
            json!({"kind":"cell_read","field":"hidden","index":0}),
            json!({"kind":"parameter","name":"leaked"}),
        ] {
            let mut value = source();
            let helper = pure(&mut value, "coinNullifier");
            let old = helper["body"].take();
            helper["body"] = json!({"kind":"if","condition":{"kind":"boolean","value":true},"then":old,"otherwise":hidden});
            assert!(!admitted(&value, "merge_qualified"));
            assert!(!admitted(&value, "receive_then_merge"));
        }
        let mut cycle = source();
        stateful(&mut cycle, "mergeCoin")["return_value"]["value"] = json!({"kind":"call","name":"mergeCoin","arguments":[{"kind":"parameter","name":"a"},{"kind":"parameter","name":"b"}]});
        assert!(!admitted(&cycle, "merge_qualified"));
        let mut extra = source();
        let original = stateful(&mut extra, "mergeCoin")["return_value"]["value"].take();
        stateful(&mut extra, "mergeCoin")["return_value"]["value"] = json!({"kind":"let","bindings":[{"name":"unused","ty":{"kind":"field"},"value":{"kind":"cell_read","field":"hidden","index":0}}],"body":original});
        assert!(!admitted(&extra, "merge_qualified"));
    }
}
