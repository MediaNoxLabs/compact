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
//! Checked-price funding followed by a shielded mint. Admission composes the
//! received-coin provenance audit; evaluation and scopes remain in shared Plan.
use super::*;
use crate::coin_shapes::shielded_coin_type;

pub(super) const PRODUCT: &str = "340282366920938463426481119284349108225";

pub(super) fn lower<'a>(
    circuit: &StatefulCircuit,
    ledger: &'a HashMap<&'a str, &'a LedgerField>,
    witnesses: &'a HashMap<&'a str, &'a WitnessDeclaration>,
    pure: &'a HashMap<&'a str, &'a PureCircuit>,
    circuits: &'a HashMap<&'a str, &'a StatefulCircuit>,
) -> Option<TypedPlan> {
    let [coin, amount] = circuit.parameters.as_slice() else {
        return None;
    };
    let StateReturn::Expression { value } = &circuit.return_value else {
        return None;
    };
    if coin.name == amount.name
        || coin.ty != shielded_coin_type()
        || amount.ty
            != (Type::Unsigned {
                max: u64::MAX.to_string(),
            })
        || circuit.result != coin.ty
        || !guarded_deposit::prefix(circuit, &coin.name, Some(&amount.name), pure, circuits)
        || !shielded_value(
            value,
            pure,
            circuits,
            &mut HashSet::from([circuit.name.clone()]),
            CompositeDomain::FundedShieldedMint,
        )
    {
        return None;
    }

    let mut plan = shielded_plan(
        ledger,
        witnesses,
        pure,
        circuits,
        CompositeDomain::FundedShieldedMint,
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
    let result = plan.expression(value, &scope, &mut steps)?;
    // Structural totals include the unselected conditional self-receive claim
    // in the mint helper, while runtime evaluation preserves lazy branching.
    (result.ty == circuit.result
        && plan.witness_calls == 0
        && plan.cell_reads == 3
        && plan.cell_writes == 1
        && plan.qualified_cell_writes == 2
        && plan.kernel_self_reads == 6
        && plan.zswap_inputs == 2
        && plan.zswap_outputs == 3
        && plan.intent_queries == 8
        && plan.counter_reads == 0
        && plan.counter_writes == 0
        && plan.counter_comparisons == 0
        && plan.tree_writes == 0
        && plan.set_writes == 0)
        .then_some(TypedPlan {
            steps,
            result: result.value,
        })
}

impl Plan<'_> {
    pub(super) fn funded_mint(
        &mut self,
        expression: &Expr,
        domain: &Expr,
        amount: &Expr,
        scope: &Scope,
        steps: &mut Vec<syn::Stmt>,
    ) -> Option<TypedValue> {
        let domain = self.expression(domain, scope, steps)?;
        let amount = self.expression(amount, scope, steps)?;
        let effect = intent_effect::emit(
            expression,
            &[(domain.ty, domain.value), (amount.ty, amount.value)],
        )?;
        self.intent_queries += usize::from(effect.public_query);
        steps.push(effect.statement);
        self.bind(syn::parse_quote!(()), Type::Unit, steps)
    }
    pub(super) fn funded_multiply(
        &mut self,
        expression: &Expr,
        left: &Expr,
        right: &Expr,
        max: &str,
        scope: &Scope,
        steps: &mut Vec<syn::Stmt>,
    ) -> Option<TypedValue> {
        let left = self.expression(left, scope, steps)?;
        let right = self.expression(right, scope, steps)?;
        let (Type::Unsigned { max: a }, Type::Unsigned { max: b }) = (&left.ty, &right.ty) else {
            return None;
        };
        if a != funded_mint::PRODUCT || b != funded_mint::PRODUCT || max != shielded_merge::INPUT {
            return None;
        }
        let value =
            crate::unsigned_arithmetic_syntax(expression, left.value, right.value, a, b, max)
                .ok()?;
        self.bind(
            value,
            Type::Unsigned {
                max: max.to_owned(),
            },
            steps,
        )
    }
    pub(super) fn funded_not_equal(
        &mut self,
        left: &Expr,
        right: &Expr,
        scope: &Scope,
        steps: &mut Vec<syn::Stmt>,
    ) -> Option<TypedValue> {
        let left = self.expression(left, scope, steps)?;
        let right = self.expression(right, scope, steps)?;
        if left.ty
            != (Type::Unsigned {
                max: u64::MAX.to_string(),
            })
            || left.ty != right.ty
        {
            return None;
        }
        let (left, right) = (left.value, right.value);
        self.bind(syn::parse_quote!(#left != #right), Type::Boolean, steps)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    fn source() -> Value {
        serde_json::from_str(include_str!(
            "../../../tests/micro-dao-buy-in-schema20-ir.json"
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
                .find(|c| c.name == "buy_in")
                .unwrap(),
            &ledger,
            &witnesses,
            &pure,
            &circuits,
        )
        .is_some()
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
    #[test]
    fn original_funded_mint_is_structural_and_typed() {
        assert!(admitted(&source()));
    }

    #[test]
    fn price_bounds_and_pure_nonce_effects_fail_closed() {
        let mut value = source();
        assert!(edit(&mut value, &mut |v| {
            if v["kind"] == "unsigned_multiply" {
                v["max"] = json!(u64::MAX.to_string());
                true
            } else {
                false
            }
        }));
        assert!(!admitted(&value));
        let mut value = source();
        value["circuits"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|c| c["name"] == "evolveNonce")
            .unwrap()["body"] = json!({"kind":"kernel_mint_shielded", "domain":{"kind":"parameter","name":"nonce"}, "amount":{"kind":"unsigned_literal","value":"1","max":u64::MAX.to_string()}});
        assert!(!admitted(&value));
    }

    #[test]
    fn funding_scope_stores_and_unused_effects_fail_closed() {
        for case in 0..7 {
            let mut value = source();
            let root = value["stateful_circuits"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|c| c["name"] == "buy_in")
                .unwrap();
            match case {
                0 => {
                    assert!(edit(root, &mut |v| {
                        if v["kind"] == "unsigned_cast" && v["max"] == PRODUCT {
                            v["max"] = json!(u64::MAX.to_string());
                            true
                        } else {
                            false
                        }
                    }));
                }
                1 => {
                    assert!(edit(root, &mut |v| {
                        if v["kind"] == "cell_read" && v["field"] == "costs" {
                            v["index"] = json!(11);
                            true
                        } else {
                            false
                        }
                    }));
                }
                2 => {
                    assert!(edit(root, &mut |v| {
                        if v["kind"] == "cell_write_coin" {
                            *v = json!({"kind":"sequence","actions":[]});
                            true
                        } else {
                            false
                        }
                    }));
                }
                3 | 4 => {
                    let actions = root["actions"].take();
                    let name = if case == 3 { "coin" } else { "amount" };
                    let parameter = root["parameters"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .find(|p| p["name"] == name)
                        .unwrap();
                    root["actions"] = json!([{"kind":"let","bindings":[{"name":name,"ty":parameter["ty"],"value":{"kind":"parameter","name":name}}],"action":{"kind":"sequence","actions":actions}}]);
                }
                5 => {
                    let body = root["return_value"]["value"].take();
                    root["return_value"]["value"] = json!({"kind":"let","bindings":[{"name":"hidden","ty":{"kind":"bytes","length":32},"value":{"kind":"witness_call","name":"local_secret_key","arguments":[]}}],"body":body});
                }
                6 => {
                    assert!(edit(root, &mut |v| {
                        if v["kind"] == "unsigned_multiply" {
                            v["right"] = json!({"kind":"parameter","name":"unknown"});
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
    fn mint_helper_hidden_effects_and_malformed_amount_are_refused() {
        for case in 0..3 {
            let mut value = source();
            let helper = value["stateful_circuits"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|c| c["name"] == "mintShieldedToken")
                .unwrap();
            match case {
                0 => helper["parameters"][1]["ty"] = json!({"kind":"field"}),
                1 => {
                    assert!(edit(helper, &mut |v| {
                        if v["kind"] == "kernel_mint_shielded" {
                            v["amount"] = json!({"kind":"parameter","name":"coin"});
                            true
                        } else {
                            false
                        }
                    }));
                }
                2 => {
                    assert!(edit(helper, &mut |v| {
                        if v["kind"] == "unit" {
                            *v = json!({"kind":"witness_call","name":"next_state","arguments":[]});
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
}
