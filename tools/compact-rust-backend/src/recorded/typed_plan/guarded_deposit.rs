// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Guarded receive followed by an optional historical merge and typed Cell
//! updates. This module audits admission and provenance; Plan owns evaluation,
//! lexical scope, exact types, query order and branch-local frame ownership.
use super::immediate_send::{parameter, uncoerced};
use super::*;

pub(super) fn read_type(ty: &Type) -> bool {
    matches!(
        ty,
        Type::Boolean | Type::Bytes { length: 32 } | Type::Enum { .. }
    ) || *ty == crate::stateful::qualified_coin_type()
        || matches!(ty, Type::Struct { fields, .. }
            if fields.len() == 2 && fields.iter().all(|field|
                field.ty == Type::Unsigned { max: u64::MAX.to_string() }))
}
pub(super) fn write_type(ty: &Type) -> bool {
    matches!(ty, Type::Boolean | Type::Enum { .. })
        || optional_string(ty)
        || matches!(ty, Type::Struct { fields, .. }
            if matches!(fields.as_slice(), [present, value]
                if present.name == "is_some" && present.ty == Type::Boolean
                    && value.name == "value" && wrapped_bytes32(&value.ty)))
}

// The application supplies a received coin, not merely a same-spelled helper
// argument. Require its direct output forwarding without formal shadowing.
// Owner/commitment/claim identity is still checked by exact offer reconciliation
// and the ledger; this predicate does not attempt to prove crypto equations.
fn forwards_received_coin(action: &StateAction, formal: &str) -> bool {
    match action {
        StateAction::Sequence { actions } => {
            actions.iter().all(|a| forwards_received_coin(a, formal))
        }
        StateAction::Let { bindings, action } => {
            bindings.iter().all(|b| b.name != formal) && forwards_received_coin(action, formal)
        }
        StateAction::Expression {
            value: Expr::CreateZswapOutput { coin, .. },
        } => parameter(coin, formal),
        StateAction::Expression {
            value:
                Expr::KernelClaim {
                    claim: KernelClaimKind::CoinReceive,
                    ..
                },
        } => true,
        _ => false,
    }
}

// Flow summarizes each executable path, independently of static node totals.
#[derive(Clone, Copy, Default)]
struct Flow {
    received: bool,
    merges: usize,
    stores: usize,
}

struct Audit<'a> {
    seed: &'a str,
    price_amount: Option<&'a str>,
    pure: &'a HashMap<&'a str, &'a PureCircuit>,
    circuits: &'a HashMap<&'a str, &'a StatefulCircuit>,
    active: HashSet<String>,
    receives: usize,
    merges: usize,
    writes: usize,
    coin_slot: Option<(String, u8)>,
}
impl Audit<'_> {
    fn slot(&mut self, field: &str, index: u8) -> bool {
        let slot = (field.to_owned(), index);
        match &self.coin_slot {
            Some(existing) => existing == &slot,
            None => {
                self.coin_slot = Some(slot);
                true
            }
        }
    }
    // Root observations and guards may not hide any mutation or intent. Only a
    // binding explicitly permits the audited receive-then-merge helper below.
    fn value(&mut self, value: &Expr, received: bool, merge: bool) -> bool {
        match value {
            Expr::Parameter { .. }
            | Expr::Boolean { .. }
            | Expr::BytesLiteral { .. }
            | Expr::EnumVariant { .. }
            | Expr::CellRead { .. } => true,
            Expr::KernelSelf { ty } => *ty == contract_address_type(),
            Expr::StructField { value, .. } | Expr::Coerce { value, .. } => {
                self.value(value, received, merge)
            }
            Expr::UnsignedCast { value, max } if max == shielded_merge::INPUT => {
                self.value(value, received, false)
            }
            Expr::UnsignedLiteral { .. } if self.price_amount.is_some() => true,
            Expr::UnsignedCast { value, max }
                if self.price_amount.is_some() && max == funded_mint::PRODUCT =>
            {
                self.value(value, received, false)
            }
            Expr::UnsignedMultiply { left, right, max }
                if self.price_amount.is_some() && max == shielded_merge::INPUT =>
            {
                self.value(left, received, false) && self.value(right, received, false)
            }
            Expr::NotEqual { left, right } if self.price_amount.is_some() => {
                self.value(left, received, false) && self.value(right, received, false)
            }
            Expr::Equal { left, right } => {
                self.value(left, received, false) && self.value(right, received, false)
            }
            Expr::If {
                condition,
                then,
                otherwise,
            } => {
                self.value(condition, received, false)
                    && self.value(then, received, false)
                    && self.value(otherwise, received, false)
            }
            Expr::WitnessCall { arguments, .. } => arguments.is_empty(),
            Expr::Call { name, arguments } => {
                if self.pure.contains_key(name.as_str()) {
                    return !self.circuits.contains_key(name.as_str())
                        && arguments
                            .iter()
                            .all(|value| self.value(value, received, false));
                }
                let [left, right] = arguments.as_slice() else {
                    return false;
                };
                let Expr::CellRead { field, index } = uncoerced(left) else {
                    return false;
                };
                if !merge
                    || !received
                    || !parameter(right, self.seed)
                    || !self.slot(field, *index)
                    || !shielded_merge::received_helper(name, self.pure, self.circuits)
                {
                    return false;
                }
                let callee = self.circuits[name.as_str()];
                let StateReturn::Expression { value } = &callee.return_value else {
                    return false;
                };
                if !self.active.insert(name.clone()) {
                    return false;
                }
                let valid = shielded_value(
                    value,
                    self.pure,
                    self.circuits,
                    &mut self.active,
                    CompositeDomain::GuardedShieldedDeposit,
                );
                self.active.remove(name);
                if valid {
                    self.merges += 1;
                }
                valid
            }
            _ => false,
        }
    }
    fn action(&mut self, action: &StateAction, flow: &mut Flow) -> bool {
        match action {
            StateAction::Sequence { actions } => {
                actions.iter().all(|action| self.action(action, flow))
            }
            StateAction::Let { bindings, action } => {
                for binding in bindings {
                    // Never confuse a shadowing local with the earlier intent.
                    let merges = self.merges;
                    if binding.name == self.seed
                        || self.price_amount == Some(binding.name.as_str())
                        || !self.value(&binding.value, flow.received, true)
                    {
                        return false;
                    }
                    flow.merges += self.merges - merges;
                    if flow.merges > 1 {
                        return false;
                    }
                }
                self.action(action, flow)
            }
            StateAction::Assert { condition, .. } => self.value(condition, flow.received, false),
            StateAction::If {
                condition,
                then,
                otherwise,
            } => {
                if !self.value(condition, flow.received, false) {
                    return false;
                }
                let (mut left, mut right) = (*flow, *flow);
                let valid = self.action(then, &mut left)
                    && self.action(otherwise, &mut right)
                    && left.received == right.received
                    && left.stores == right.stores;
                *flow = Flow {
                    received: left.received,
                    stores: left.stores,
                    merges: left.merges.max(right.merges),
                };
                valid
            }
            StateAction::CircuitCall { name, arguments } => {
                let Some(callee) = self.circuits.get(name.as_str()) else {
                    return false;
                };
                if !shielded_unit_signature(callee)
                    || !callee
                        .actions
                        .iter()
                        .all(|a| forwards_received_coin(a, &callee.parameters[0].name))
                {
                    return false;
                }
                if flow.received
                    || !matches!(arguments.as_slice(), [value] if parameter(value,self.seed))
                    || !shielded_receive_action(action, self.pure, self.circuits, &mut self.active)
                {
                    return false;
                }
                self.receives += 1;
                flow.received = true;
                true
            }
            StateAction::CellWriteCoin {
                field,
                index,
                coin,
                recipient,
            } => {
                if !flow.received
                    || !self.slot(field, *index)
                    || !self.value(coin, flow.received, false)
                    || !self.value(recipient, flow.received, false)
                {
                    return false;
                }
                self.writes += 1;
                flow.stores += 1;
                flow.stores <= 1
            }
            StateAction::CellWrite { value, .. } => self.value(value, flow.received, false),
            _ => false,
        }
    }
}

// Shared admission of a received coin followed by exactly one qualified store
// per execution path. Price arithmetic is opted into by the funded-mint profile;
// the original guarded deposit retains its previous expression domain.
pub(super) fn prefix(
    circuit: &StatefulCircuit,
    seed: &str,
    price_amount: Option<&str>,
    pure: &HashMap<&str, &PureCircuit>,
    circuits: &HashMap<&str, &StatefulCircuit>,
) -> bool {
    let mut audit = Audit {
        seed,
        price_amount,
        pure,
        circuits,
        active: HashSet::from([circuit.name.clone()]),
        receives: 0,
        merges: 0,
        writes: 0,
        coin_slot: None,
    };
    let mut flow = Flow::default();
    circuit
        .actions
        .iter()
        .all(|action| audit.action(action, &mut flow))
        && flow.received
        && flow.stores == 1
        && flow.merges <= 1
        && (audit.receives, audit.merges, audit.writes) == (1, 1, 2)
}

pub(super) fn lower<'a>(
    circuit: &StatefulCircuit,
    ledger: &'a HashMap<&'a str, &'a LedgerField>,
    witnesses: &'a HashMap<&'a str, &'a WitnessDeclaration>,
    pure: &'a HashMap<&'a str, &'a PureCircuit>,
    circuits: &'a HashMap<&'a str, &'a StatefulCircuit>,
) -> Option<TypedPlan> {
    let [topic, beneficiary, seed] = circuit.parameters.as_slice() else {
        return None;
    };
    if circuit.result != Type::Unit
        || !matches!(circuit.return_value, StateReturn::Unit)
        || topic.ty != Type::OpaqueString
        || !wrapped_bytes32(&beneficiary.ty)
        || seed.ty != crate::stateful::shielded_coin_type()
        || circuit
            .parameters
            .iter()
            .map(|p| &p.name)
            .collect::<HashSet<_>>()
            .len()
            != 3
    {
        return None;
    }
    if !prefix(circuit, &seed.name, None, pure, circuits) {
        return None;
    }
    let mut plan = shielded_plan(
        ledger,
        witnesses,
        pure,
        circuits,
        CompositeDomain::GuardedShieldedDeposit,
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
    for action in &circuit.actions {
        plan.action(action, &scope, &mut steps)?;
    }
    // Counts are structural (both branches), while generated execution retains
    // exactly the selected branch. No counters, collections or hidden effects.
    (plan.witness_calls == 1
        && plan.cell_reads == 5
        && plan.cell_writes == 4
        && plan.qualified_cell_writes == 2
        && plan.kernel_self_reads == 4
        && plan.zswap_inputs == 2
        && plan.zswap_outputs == 2
        && plan.intent_queries == 5
        && plan.counter_reads == 0
        && plan.counter_writes == 0
        && plan.counter_comparisons == 0
        && plan.tree_writes == 0
        && plan.set_writes == 0)
        .then_some(TypedPlan {
            steps,
            result: syn::parse_quote!(()),
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};
    fn source() -> Value {
        serde_json::from_str(include_str!(
            "../../../tests/micro-dao-set-topic-schema20-ir.json"
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
                .find(|c| c.name.ends_with("set_topic"))
                .unwrap(),
            &ledger,
            &witnesses,
            &pure,
            &circuits,
        )
        .is_some()
    }
    #[test]
    fn original_guarded_deposit_is_structural_and_typed() {
        assert!(admitted(&source()));
    }
    fn edit(value: &mut Value, change: &mut impl FnMut(&mut Value) -> bool) -> bool {
        if change(value) {
            return true;
        }
        match value {
            Value::Object(map) => map.values_mut().any(|v| edit(v, change)),
            Value::Array(values) => values.iter_mut().any(|v| edit(v, change)),
            _ => false,
        }
    }
    fn root(value: &mut Value) -> &mut Value {
        value["stateful_circuits"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|c| c["name"] == "set_topic")
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
    #[test]
    fn malformed_deposit_types_slots_and_lexical_inputs_fail_closed() {
        for case in 0..9 {
            let mut value = source();
            match case {
                0 => {
                    value["ledger_fields"]
                        .as_array_mut()
                        .unwrap()
                        .retain(|f| f["id"] != "costs");
                }
                1 => {
                    value["ledger_fields"][12]["declaration"]["ty"]["fields"][0]["ty"] =
                        json!({"kind":"field"});
                }
                2 => {
                    root(&mut value)["parameters"][0]["ty"] = json!({"kind":"field"});
                }
                3 => {
                    root(&mut value)["result"] = json!({"kind":"field"});
                }
                4 => {
                    value["witnesses"][0]["result"] = json!({"kind":"field"});
                }
                5 => {
                    value["witnesses"][0]["parameters"] =
                        json!([{"name":"unexpected","ty":{"kind":"boolean"}}]);
                }
                6 => {
                    assert!(edit(root(&mut value), &mut |v| {
                        if v["kind"] == "cell_read" && v["field"] == "pot" {
                            v["index"] = json!(9);
                            true
                        } else {
                            false
                        }
                    }));
                }
                7 => {
                    assert!(edit(root(&mut value), &mut |v| {
                        if v["kind"] == "cell_write_coin" {
                            v["coin"] = json!({"kind":"parameter","name":"branch_local_escape"});
                            true
                        } else {
                            false
                        }
                    }));
                }
                8 => {
                    assert!(edit(root(&mut value), &mut |v| {
                        if v["kind"] == "cell_write_coin" {
                            v["recipient"] = json!({"kind":"boolean","value":true});
                            true
                        } else {
                            false
                        }
                    }));
                }
                _ => unreachable!(),
            }
            assert!(!admitted(&value), "case {case}");
        }
    }
    #[test]
    fn receive_merge_provenance_and_helper_audit_fail_closed() {
        for case in 0..8 {
            let mut value = source();
            match case {
                0 => {
                    assert!(edit(root(&mut value), &mut |v| {
                        if v["kind"] == "circuit_call" {
                            v["arguments"][0] =
                                json!({"kind":"parameter","name":"beneficiary_param"});
                            true
                        } else {
                            false
                        }
                    }));
                }
                1 => {
                    assert!(edit(root(&mut value), &mut |v| {
                        if v["kind"] == "call" && v["name"] == "mergeCoinImmediate" {
                            v["arguments"][1] = v["arguments"][0].clone();
                            true
                        } else {
                            false
                        }
                    }));
                }
                2 => {
                    assert!(edit(
                        &mut pure(&mut value, "upcastQualifiedCoin")["body"],
                        &mut |v| {
                            if v["kind"] == "unsigned_literal" {
                                v["value"] = json!("1");
                                true
                            } else {
                                false
                            }
                        }
                    ));
                }
                3 => {
                    let p = pure(&mut value, "public_key");
                    p["body"] = json!({"kind":"call","name":"public_key","arguments":[{"kind":"parameter","name":p["parameters"][0]["name"]}]});
                }
                4 => {
                    let p = pure(&mut value, "public_key");
                    let old = p["body"].clone();
                    p["body"] = json!({"kind":"let","bindings":[{"name":"unused","ty":{"kind":"boolean"},"value":{"kind":"cell_read","field":"pot_has_coin","index":11}}],"body":old});
                }
                5 => {
                    let p = pure(&mut value, "public_key");
                    let old = p["body"].clone();
                    p["body"] = json!({"kind":"if","condition":{"kind":"boolean","value":true},"then":old,"otherwise":{"kind":"witness_call","name":"local_secret_key","arguments":[]}});
                }
                6 => {
                    let actions = root(&mut value)["actions"].as_array_mut().unwrap();
                    actions.push(json!({"kind":"counter_reset","field":"round","index":2}));
                }
                7 => {
                    assert!(edit(root(&mut value), &mut |v| {
                        if v["kind"] == "sequence"
                            && let Some(actions) = v["actions"].as_array_mut()
                            && let Some(i) =
                                actions.iter().position(|a| a["kind"] == "circuit_call")
                        {
                            actions.swap(i, i + 1);
                            return true;
                        }
                        false
                    }));
                }
                _ => unreachable!(),
            }
            assert!(!admitted(&value), "case {case}");
        }
    }
    #[test]
    fn each_selected_path_stores_once_instead_of_only_matching_static_totals() {
        let mut value = source();
        assert!(edit(root(&mut value), &mut |v| {
            if v["kind"] == "if"
                && v["then"]["kind"] == "sequence"
                && v["then"]["actions"][0]["kind"] == "let"
                && v["then"]["actions"][0]["action"]["kind"] == "cell_write_coin"
            {
                let moved = v["then"]["actions"].as_array_mut().unwrap().remove(0);
                let old = v["otherwise"].take();
                v["otherwise"] = json!({"kind":"sequence","actions":[moved,old]});
                return true;
            }
            false
        }));
        assert!(!admitted(&value));
    }
    #[test]
    fn receive_declaration_must_forward_the_protected_coin() {
        for shadow in [false, true] {
            let mut value = source();
            let callee = value["stateful_circuits"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|c| c["name"] == "receiveShielded")
                .unwrap();
            let formal = callee["parameters"][0].clone();
            if shadow {
                let old = callee["actions"].take();
                callee["actions"] = json!([{"kind":"let","bindings":[{"name":formal["name"],"ty":formal["ty"],"value":{"kind":"default","ty":formal["ty"]}}],"action":{"kind":"sequence","actions":old}}]);
            } else {
                assert!(edit(callee, &mut |v| {
                    if v["kind"] == "create_zswap_output" {
                        v["coin"] = json!({"kind":"default","ty":formal["ty"]});
                        return true;
                    }
                    false
                }));
            }
            assert!(!admitted(&value));
        }
    }
    #[test]
    fn root_seed_shadowing_cannot_change_received_provenance() {
        let mut value = source();
        let source_root = root(&mut value);
        let seed = source_root["parameters"][2].clone();
        let old = source_root["actions"].clone();
        source_root["actions"] = json!([{
            "kind":"let",
            "bindings":[{"name":seed["name"],"ty":seed["ty"],
                "value":{"kind":"parameter","name":seed["name"]}}],
            "action":{"kind":"sequence","actions":old}
        }]);
        assert!(!admitted(&value));
    }
    #[test]
    fn declaration_names_are_data_not_admission_gates() {
        fn rename(v: &mut Value) {
            match v {
                Value::Object(map) => {
                    if let Some(Value::String(name)) = map.get_mut("name")
                        && [
                            "set_topic",
                            "receiveShielded",
                            "mergeCoinImmediate",
                            "upcastQualifiedCoin",
                        ]
                        .contains(&name.as_str())
                    {
                        name.insert_str(0, "renamed_");
                    }
                    for v in map.values_mut() {
                        rename(v);
                    }
                }
                Value::Array(values) => {
                    for v in values {
                        rename(v);
                    }
                }
                _ => {}
            }
        }
        let mut value = source();
        rename(&mut value);
        assert!(admitted(&value));
    }
}
