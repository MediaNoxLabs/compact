// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Qualified-coin payout composition. Distinct read-only and actionful domains
//! audit every branch and binding; the shared Plan owns values, calls and scope.
use super::*;

pub(super) fn cell_type(ty: &Type) -> bool {
    matches!(ty, Type::Bytes { length: 32 } | Type::Enum { .. })
        || *ty == crate::stateful::qualified_coin_type()
}
pub(super) fn actionful_cell_type(ty: &Type) -> bool {
    cell_type(ty)
        || matches!(ty, Type::Struct { fields, .. }
            if matches!(fields.as_slice(), [value]
                if value.name == "value" && value.ty == Type::Field)
            || matches!(fields.as_slice(), [present, value]
                if present.name == "is_some" && present.ty == Type::Boolean
                    && value.name == "value" && value.ty == Type::Field))
}
fn coin_result(ty: &Type) -> bool {
    if *ty == crate::stateful::shielded_coin_type() {
        return true;
    }
    matches!(ty,Type::Struct { fields, .. } if !fields.is_empty() && fields.iter().all(|field|coin_result(&field.ty)))
}

// The private board opening is a typed pair of a nonce and a one-field board.
// The byte witness remains available to the existing read-only payout path.
pub(super) fn witness_type(ty: &Type) -> bool {
    if *ty == (Type::Bytes { length: 32 }) {
        return true;
    }
    matches!(ty, Type::Struct { fields, .. }
        if matches!(fields.as_slice(), [nonce, contents]
            if nonce.name == "nonce" && nonce.ty == Type::Field
                && contents.name == "contents"
                && matches!(&contents.ty, Type::Struct { fields, .. }
                    if matches!(fields.as_slice(), [position]
                        if position.name == "position" && position.ty == Type::Field))))
}

// Action predicates and bindings may observe ledger state and read witnesses,
// but their values cannot hide a second mutation or Zswap intent. The Plan
// subsequently validates each complete declaration, type and lexical scope.
fn action_value(value: &Expr, pure: &HashMap<&str, &PureCircuit>) -> bool {
    match value {
        Expr::Parameter { .. }
        | Expr::Boolean { .. }
        | Expr::BytesLiteral { .. }
        | Expr::FieldLiteral { .. }
        | Expr::EnumVariant { .. }
        | Expr::Unit
        | Expr::CellRead { .. } => true,
        Expr::StructField { value, .. } | Expr::Coerce { value, .. } => action_value(value, pure),
        Expr::Equal { left, right } => action_value(left, pure) && action_value(right, pure),
        Expr::If {
            condition,
            then,
            otherwise,
        } => {
            action_value(condition, pure)
                && action_value(then, pure)
                && action_value(otherwise, pure)
        }
        Expr::Let { bindings, body } => {
            bindings
                .iter()
                .all(|binding| action_value(&binding.value, pure))
                && action_value(body, pure)
        }
        Expr::Call { name, arguments } => {
            pure.contains_key(name.as_str())
                && arguments
                    .iter()
                    .all(|argument| action_value(argument, pure))
        }
        Expr::WitnessCall { arguments, .. } => arguments
            .iter()
            .all(|argument| action_value(argument, pure)),
        _ => false,
    }
}

fn action_writes(action: &StateAction, pure: &HashMap<&str, &PureCircuit>) -> Option<usize> {
    match action {
        StateAction::Sequence { actions } => actions.iter().try_fold(0usize, |count, action| {
            count.checked_add(action_writes(action, pure)?)
        }),
        StateAction::Let { bindings, action }
            if bindings
                .iter()
                .all(|binding| action_value(&binding.value, pure)) =>
        {
            action_writes(action, pure)
        }
        StateAction::Assert { condition, .. } if action_value(condition, pure) => Some(0),
        StateAction::CellWrite {
            value: Expr::EnumVariant { .. },
            ..
        } => Some(1),
        _ => None,
    }
}

// Every admitted actionful helper has one typed enum write before its return.
// An If action, hidden expression effect or additional write fails closed.
pub(super) fn action_shape(
    actions: &[StateAction],
    pure: &HashMap<&str, &PureCircuit>,
) -> Option<usize> {
    actions.iter().try_fold(0usize, |count, action| {
        count.checked_add(action_writes(action, pure)?)
    })
}

// A path contains the number of helpers that own an ordered enum write.
// Alternative branches contribute their minimum and maximum, whereas
// sequential operands and bindings add. This rejects a second helper on any
// executable path without confusing the two mutually exclusive player arms.
fn add_paths(left: (usize, usize), right: (usize, usize)) -> Option<(usize, usize)> {
    Some((left.0.checked_add(right.0)?, left.1.checked_add(right.1)?))
}

pub(super) fn actionful_call_paths(
    value: &Expr,
    pure: &HashMap<&str, &PureCircuit>,
    circuits: &HashMap<&str, &StatefulCircuit>,
    active: &mut HashSet<String>,
    allow_unit: bool,
) -> Option<(usize, usize)> {
    let visit = |part: &Expr, active: &mut HashSet<String>| {
        actionful_call_paths(part, pure, circuits, active, allow_unit)
    };
    let sum = |values: &[Expr], active: &mut HashSet<String>| {
        values
            .iter()
            .try_fold((0, 0), |count, part| add_paths(count, visit(part, active)?))
    };
    match value {
        Expr::Parameter { .. }
        | Expr::Boolean { .. }
        | Expr::BytesLiteral { .. }
        | Expr::FieldLiteral { .. }
        | Expr::UnsignedLiteral { .. }
        | Expr::EnumVariant { .. }
        | Expr::Unit
        | Expr::Default { .. }
        | Expr::CellRead { .. }
        | Expr::CounterRead { .. }
        | Expr::KernelSelf { .. }
        | Expr::NativeWitnessCall { .. } => Some((0, 0)),
        Expr::CounterLessThan {
            threshold: value, ..
        }
        | Expr::StructField { value, .. }
        | Expr::Coerce { value, .. }
        | Expr::UnsignedCast { value, .. }
        | Expr::DegradeToTransient { value }
        | Expr::TransientHash { value }
        | Expr::UpgradeFromTransient { value }
        | Expr::CreateZswapInput { coin: value }
        | Expr::KernelClaim { value, .. }
        | Expr::PersistentHash { value }
        | Expr::Assert {
            condition: value, ..
        } => visit(value, active),
        Expr::Equal { left, right }
        | Expr::UnsignedAdd { left, right, .. }
        | Expr::UnsignedSubtract { left, right, .. }
        | Expr::CreateZswapOutput {
            coin: left,
            recipient: right,
        }
        | Expr::TransientCommit {
            value: left,
            opening: right,
        } => add_paths(visit(left, active)?, visit(right, active)?),
        Expr::Tuple { elements }
        | Expr::StructLiteral {
            fields: elements, ..
        } => sum(elements, active),
        Expr::WitnessCall { arguments, .. } => sum(arguments, active),
        Expr::Let { bindings, body } => {
            let bindings = bindings.iter().try_fold((0, 0), |count, binding| {
                add_paths(count, visit(&binding.value, active)?)
            })?;
            add_paths(bindings, visit(body, active)?)
        }
        Expr::Sequence { steps, value } => add_paths(sum(steps, active)?, visit(value, active)?),
        Expr::If {
            condition,
            then,
            otherwise,
        } => {
            let condition = visit(condition, active)?;
            let then = visit(then, active)?;
            let otherwise = visit(otherwise, active)?;
            add_paths(
                condition,
                (then.0.min(otherwise.0), then.1.max(otherwise.1)),
            )
        }
        Expr::Call { name, arguments } => {
            let arguments = sum(arguments, active)?;
            match (pure.get(name.as_str()), circuits.get(name.as_str())) {
                (Some(_), None) => Some(arguments),
                (None, Some(callee)) if active.insert(name.clone()) => {
                    let body = match &callee.return_value {
                        StateReturn::Expression { value } => visit(value, active),
                        StateReturn::Unit if allow_unit => Some((0, 0)),
                        _ => None,
                    };
                    active.remove(name);
                    let body = body?;
                    let write = usize::from(!callee.actions.is_empty());
                    add_paths(arguments, add_paths((write, write), body)?)
                }
                _ => None,
            }
        }
        _ => None,
    }
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
    let actionful_paths = actionful_call_paths(
        value,
        pure,
        circuits,
        &mut HashSet::from([circuit.name.clone()]),
        false,
    )?;
    let domain = match actionful_paths {
        (0, 0) => CompositeDomain::ShieldedPayout,
        (1, 1) => CompositeDomain::ActionfulShieldedPayout,
        _ => return None,
    };
    if !circuit.actions.is_empty()
        || !coin_result(&circuit.result)
        || circuit
            .parameters
            .iter()
            .map(|p| &p.name)
            .collect::<HashSet<_>>()
            .len()
            != circuit.parameters.len()
        || !shielded_value(
            value,
            pure,
            circuits,
            &mut HashSet::from([circuit.name.clone()]),
            domain,
        )
    {
        return None;
    }
    let mut plan = shielded_plan(ledger, witnesses, pure, circuits, domain);
    let scope = circuit
        .parameters
        .iter()
        .enumerate()
        .map(|(index, p)| {
            let name = syn::Ident::new(&format!("__compact_param_{index}"), Span::call_site());
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
    let result = plan.expression(value, &scope, &mut steps)?;
    (result.ty == circuit.result
        && plan.cell_reads > 0
        && plan.zswap_inputs > 0
        && plan.zswap_outputs > 0
        && plan.intent_queries > 0
        && (domain == CompositeDomain::ActionfulShieldedPayout || plan.cell_writes == 0)
        && (domain == CompositeDomain::ShieldedPayout || plan.cell_writes > 0)
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
mod concede_tests {
    use super::*;
    use serde_json::{Value, json};

    fn source() -> Value {
        serde_json::from_str(include_str!(
            "../../../tests/coracle-concede-schema20-ir.json"
        ))
        .unwrap()
    }

    fn admitted(source: &Value) -> Option<String> {
        let source = source.clone();
        std::thread::Builder::new()
            .stack_size(8 * 1024 * 1024)
            .spawn(move || {
                let c: crate::ir::Contract = serde_json::from_value(source).unwrap();
                let ledger = c.ledger_fields.iter().map(|f| (f.id.as_str(), f)).collect();
                let witnesses = c.witnesses.iter().map(|w| (w.name.as_str(), w)).collect();
                let pure = c.circuits.iter().map(|c| (c.name.as_str(), c)).collect();
                let circuits = c
                    .stateful_circuits
                    .iter()
                    .map(|c| (c.name.as_str(), c))
                    .collect();
                let root = c.stateful_circuits.iter().find(|c| !c.internal)?;
                let plan = lower(root, &ledger, &witnesses, &pure, &circuits)?;
                let steps = plan.steps;
                Some(quote::quote!(#(#steps)*).to_string())
            })
            .unwrap()
            .join()
            .unwrap()
    }

    fn helper<'a>(source: &'a mut Value, name: &str) -> &'a mut Value {
        source["stateful_circuits"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|c| c["name"] == name)
            .unwrap()
    }

    #[test]
    fn original_concede_keeps_branch_writes_before_selected_send() {
        let mut source = source();
        let tokens = admitted(&source).expect("original concede must be admitted");
        assert!(tokens.contains("record_write"));
        assert!(tokens.contains("record_read"));
        assert!(tokens.contains("try_witness_metered"));
        assert!(tokens.contains("own_coin_public_key"));
        let write = tokens.find("record_write").unwrap();
        let input = tokens.find("create_zswap_input").unwrap();
        assert!(write < input, "winner write must precede selected send");
        helper(&mut source, "concede")["name"] = json!("renamed_concession");
        assert!(admitted(&source).is_some());
    }

    #[test]
    fn actionful_payout_rejects_hidden_effects_extra_writes_and_wrong_types() {
        for hidden in [
            json!({"kind":"counter_read","field":"hidden","index":9}),
            json!({"kind":"create_zswap_input","coin":{"kind":"parameter","name":"sk"}}),
            json!({"kind":"native_witness_call","builtin":"own_public_key"}),
        ] {
            let mut bad = source();
            let helper = helper(&mut bad, "red_concede");
            helper["actions"][0]["bindings"][0]["value"] = hidden;
            assert!(admitted(&bad).is_none());
        }
        let mut bad = source();
        let branch = helper(&mut bad, "red_concede");
        let action = branch["actions"][0]["action"]["actions"][0]["action"]["actions"]
            .as_array_mut()
            .unwrap();
        let duplicate = action.last().unwrap().clone();
        action.push(duplicate);
        assert!(admitted(&bad).is_none());

        let mut bad = source();
        helper(&mut bad, "red_concede")["actions"][0]["action"]["actions"][0]["action"]["actions"]
            [4]["value"] = json!({"kind":"boolean","value":true});
        assert!(admitted(&bad).is_none());

        let mut bad = source();
        bad["witnesses"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|w| w["name"] == "local_board")
            .unwrap()["result"] = json!({"kind":"field"});
        assert!(admitted(&bad).is_none());
    }

    #[test]
    fn actionful_payout_requires_exactly_one_writing_helper_per_path() {
        let mut twice = source();
        let root = helper(&mut twice, "concede");
        let original = root["return_value"]["value"].take();
        root["return_value"]["value"] = json!({
            "kind":"sequence",
            "steps":[{"kind":"call","name":"red_concede","arguments":[]}],
            "value":original
        });
        assert!(
            admitted(&twice).is_none(),
            "sequential second payout must refuse"
        );

        let mut missing = source();
        let root = helper(&mut missing, "concede");
        let result = root["result"].clone();
        let arm = &mut root["return_value"]["value"]["body"]["body"]["body"]["value"];
        assert_eq!(arm["kind"], "if");
        arm["otherwise"] = json!({"kind":"default","ty":result});
        assert!(
            admitted(&missing).is_none(),
            "a path without a payout must refuse"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};
    fn source() -> Value {
        serde_json::from_str(include_str!(
            "../../../tests/coracle-withdraw-schema20-ir.json"
        ))
        .unwrap()
    }
    fn admitted(source: &Value) -> Option<String> {
        // Original two-send closure exceeds the test harness's small worker stack.
        // Use an explicit 8 MiB stack for this full-source structural test.
        let source = source.clone();
        std::thread::Builder::new()
            .stack_size(8 * 1024 * 1024)
            .spawn(move || admitted_inner(&source))
            .unwrap()
            .join()
            .unwrap()
    }
    fn admitted_inner(source: &Value) -> Option<String> {
        let c: crate::ir::Contract = serde_json::from_value(source.clone()).unwrap();
        let ledger = c.ledger_fields.iter().map(|f| (f.id.as_str(), f)).collect();
        let witnesses = c.witnesses.iter().map(|w| (w.name.as_str(), w)).collect();
        let pure = c.circuits.iter().map(|c| (c.name.as_str(), c)).collect();
        let circuits = c
            .stateful_circuits
            .iter()
            .map(|c| (c.name.as_str(), c))
            .collect();
        let root = c.stateful_circuits.iter().find(|c| !c.internal).unwrap();
        assert!(lower_shielded_send(root, &ledger, &witnesses, &pure, &circuits).is_none());
        let plan = lower(root, &ledger, &witnesses, &pure, &circuits)?;
        let steps = plan.steps;
        Some(quote::quote!(#(#steps)*).to_string())
    }
    fn entry(source: &mut Value) -> &mut Value {
        source["stateful_circuits"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|c| c["name"] == "withdraw")
            .unwrap()
    }
    fn inject(source: &mut Value, expr: Value, ty: Value) {
        let root = entry(source);
        let value = root["return_value"]["value"].take();
        root["return_value"]["value"] =
            json!({"kind":"let","bindings":[{"name":"unused","ty":ty,"value":expr}],"body":value});
    }
    #[test]
    fn original_withdraw_uses_typed_native_key_and_shared_send_calls() {
        let source = source();
        let tokens = admitted(&source).expect("original withdraw must be admitted");
        assert!(tokens.contains("frame . own_coin_public_key () ?"));
        assert!(tokens.contains("witnesses . local_secret_key"));
        assert!(tokens.contains("crate :: ledger_slots :: pot . record_read"));
        assert!(!tokens.contains("pure_circuits :: ownPublicKey"));
        let mut renamed = source;
        entry(&mut renamed)["name"] = "renamed_payout".into();
        assert!(admitted(&renamed).is_some());
    }
    #[test]
    fn payout_audits_unused_effects_declarations_types_and_callee_scope() {
        let source = source();
        for (expr, ty) in [
            (
                json!({"kind":"counter_read","field":"hidden","index":9}),
                json!({"kind":"unsigned","max":"18446744073709551615"}),
            ),
            (
                json!({"kind":"set_member","field":"hidden","index":9,"value":{"kind":"bytes_literal","bytes":vec![0;32]}}),
                json!({"kind":"boolean"}),
            ),
            (
                json!({"kind":"cell_read","field":"pot","index":99}),
                serde_json::to_value(crate::stateful::qualified_coin_type()).unwrap(),
            ),
            (
                json!({"kind":"call","name":"missing","arguments":[]}),
                json!({"kind":"unit"}),
            ),
        ] {
            let mut bad = source.clone();
            inject(&mut bad, expr, ty);
            assert!(admitted(&bad).is_none());
        }
        let mut bad = source.clone();
        bad["witnesses"][0]["result"] = json!({"kind":"field"});
        assert!(admitted(&bad).is_none());
        let mut bad = source.clone();
        bad["ledger_fields"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|f| f["id"] == "pot")
            .unwrap()["declaration"]["ty"] = json!({"kind":"field"});
        assert!(admitted(&bad).is_none());
        let mut bad = source.clone();
        let helper = bad["stateful_circuits"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|c| c["name"] == "red_withdraw")
            .unwrap();
        helper["return_value"]["value"] = json!({"kind":"parameter","name":"sk"});
        assert!(admitted(&bad).is_none());
        let mut bad = source.clone();
        let helper = bad["circuits"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|c| c["name"] == "is_red")
            .unwrap();
        helper["body"] = json!({"kind":"cell_read","field":"red","index":0});
        assert!(admitted(&bad).is_none());
        let mut bad = source.clone();
        let helper = bad["stateful_circuits"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|c| c["name"] == "blue_withdraw")
            .unwrap();
        helper["return_value"]["value"] = json!({"kind":"call","name":"withdraw","arguments":[]});
        assert!(admitted(&bad).is_none());
    }
    #[test]
    fn payout_rejects_hidden_native_effects_cycles_and_lexical_escapes() {
        let source = source();
        let mut wrong_key_type = source.clone();
        inject(
            &mut wrong_key_type,
            json!({"kind":"native_witness_call","builtin":"own_public_key"}),
            json!({"kind":"boolean"}),
        );
        assert!(admitted(&wrong_key_type).is_none());
        for effect in [
            json!({"kind":"native_witness_call","builtin":"own_public_key"}),
            json!({"kind":"witness_call","name":"local_secret_key","arguments":[]}),
        ] {
            let mut bad = source.clone();
            let helper = bad["circuits"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|c| c["name"] == "is_red")
                .unwrap();
            let body = helper["body"].take();
            helper["body"] = json!({"kind":"let","bindings":[{"name":"unused","ty":{"kind":"bytes","length":32},"value":effect}],"body":body});
            assert!(admitted(&bad).is_none());
        }
        let mut bad = source.clone();
        let helper = bad["circuits"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|c| c["name"] == "is_red")
            .unwrap();
        let args: Vec<_> = helper["parameters"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| json!({"kind":"parameter","name":p["name"]}))
            .collect();
        helper["body"] = json!({"kind":"call","name":"is_red","arguments":args});
        assert!(admitted(&bad).is_none());
        let mut bad = source.clone();
        bad["witnesses"][0]["parameters"] =
            json!([{"name":"extra","ty":{"kind":"bytes","length":32}}]);
        assert!(admitted(&bad).is_none());
        let mut bad = source.clone();
        bad["ledger_fields"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|f| f["id"] == "pot")
            .unwrap()["path"] = json!([6, 0]);
        assert!(admitted(&bad).is_none());
        for branch in [false, true] {
            let mut bad = source.clone();
            let tail = entry(&mut bad)["return_value"]["value"].take();
            let binding = json!({"name":"escaped","ty":{"kind":"boolean"},"value":{"kind":"boolean","value":true}});
            let scoped = json!({"kind":"let","bindings":[binding],"body":{"kind":"unit"}});
            let scoped = if branch {
                json!({"kind":"if","condition":{"kind":"boolean","value":true},"then":scoped,"otherwise":{"kind":"unit"}})
            } else {
                scoped
            };
            entry(&mut bad)["return_value"]["value"] = json!({"kind":"sequence","steps":[scoped],"value":{"kind":"let","bindings":[{"name":"used","ty":{"kind":"boolean"},"value":{"kind":"parameter","name":"escaped"}}],"body":tail}});
            assert!(admitted(&bad).is_none());
        }
        let mut bad = source.clone();
        inject(
            &mut bad,
            json!({"kind":"if","condition":{"kind":"boolean","value":true},"then":{"kind":"boolean","value":true},"otherwise":{"kind":"counter_less_than","field":"hidden","index":9,"threshold":{"kind":"unsigned_literal","value":"1","max":"18446744073709551615"}}}),
            json!({"kind":"boolean"}),
        );
        assert!(admitted(&bad).is_none());
    }
}
