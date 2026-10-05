// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Read-only qualified-coin payout composition. The send walker audits every
//! branch and binding; the shared Plan owns typed values, calls and scope.
use super::*;

pub(super) fn cell_type(ty: &Type) -> bool {
    matches!(ty, Type::Bytes { length: 32 } | Type::Enum { .. })
        || *ty == crate::stateful::qualified_coin_type()
}
fn coin_result(ty: &Type) -> bool {
    if *ty == crate::stateful::shielded_coin_type() {
        return true;
    }
    matches!(ty,Type::Struct { fields, .. } if !fields.is_empty() && fields.iter().all(|field|coin_result(&field.ty)))
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
            CompositeDomain::ShieldedPayout,
        )
    {
        return None;
    }
    let mut plan = shielded_plan(
        ledger,
        witnesses,
        pure,
        circuits,
        CompositeDomain::ShieldedPayout,
    );
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
