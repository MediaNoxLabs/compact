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
//! Attach extracted returns to their terminal lexical continuation. This is a
//! structural adapter and domain audit; the shared Plan owns all evaluation.
use super::*;

/// Only the final Sequence/Let suffix encloses an extracted return. Earlier
/// actions and If branches remain ordinary action scopes. Already explicit
/// Effectful ReturnPlans never pass through this adapter.
pub(super) fn adapt(actions: &[StateAction], value: &Expr) -> ReturnPlan {
    let Some((last, prefix)) = actions.split_last() else {
        return ReturnPlan::Value {
            value: value.clone(),
        };
    };
    let continuation = attach(last, value);
    if prefix.is_empty() {
        continuation
    } else {
        ReturnPlan::Sequence {
            actions: prefix.to_vec(),
            result: Box::new(continuation),
        }
    }
}
fn attach(action: &StateAction, value: &Expr) -> ReturnPlan {
    match action {
        StateAction::Let { bindings, action } => ReturnPlan::Let {
            bindings: bindings.clone(),
            result: Box::new(attach(action, value)),
        },
        StateAction::Sequence { actions } => adapt(actions, value),
        _ => ReturnPlan::Sequence {
            actions: vec![action.clone()],
            result: Box::new(ReturnPlan::Value {
                value: value.clone(),
            }),
        },
    }
}
fn scalar(ty: &Type) -> bool {
    matches!(ty, Type::Field | Type::Boolean)
}
struct Audit<'a> {
    ledger: &'a HashMap<&'a str, &'a LedgerField>,
    witnesses: &'a HashMap<&'a str, &'a WitnessDeclaration>,
    pure: &'a HashMap<&'a str, &'a PureCircuit>,
    circuits: &'a HashMap<&'a str, &'a StatefulCircuit>,
    active: HashSet<String>,
    slot: Option<(String, u8)>,
}
impl Audit<'_> {
    fn cell(&mut self, field: &str, index: u8) -> bool {
        if !self.ledger.get(field).is_some_and(|f| {
            f.index == index
                && f.physical_path() == [index]
                && f.declaration == (LedgerFieldKind::Cell { ty: Type::Field })
        }) {
            return false;
        }
        let slot = (field.to_owned(), index);
        match &self.slot {
            Some(old) => old == &slot,
            None => {
                self.slot = Some(slot);
                true
            }
        }
    }
    fn value(&mut self, v: &Expr) -> bool {
        match v {
            Expr::Parameter { .. } | Expr::FieldLiteral { .. } | Expr::Boolean { .. } => true,
            Expr::CellRead { field, index } => self.cell(field, *index),
            Expr::Add { left, right } | Expr::Equal { left, right } => {
                self.value(left) && self.value(right)
            }
            Expr::Coerce { value, ty } => scalar(ty) && self.value(value),
            Expr::If {
                condition,
                then,
                otherwise,
            } => self.value(condition) && self.value(then) && self.value(otherwise),
            Expr::Let { bindings, body } => self.bindings(bindings) && self.value(body),
            Expr::WitnessCall { name, arguments } => {
                self.witnesses.get(name.as_str()).is_some_and(|w| {
                    w.result == Type::Field && w.parameters.iter().all(|p| p.ty == Type::Field)
                }) && arguments.iter().all(|v| self.value(v))
            }
            Expr::Call { name, arguments } => {
                if !arguments.iter().all(|v| self.value(v)) {
                    return false;
                }
                match (
                    self.pure.get(name.as_str()).copied(),
                    self.circuits.get(name.as_str()).copied(),
                ) {
                    // Shared Plan::pure_call audits the full pure body, types and cycles.
                    (Some(c), None) => {
                        scalar(&c.result) && c.parameters.iter().all(|p| scalar(&p.ty))
                    }
                    (None, Some(c)) => {
                        if !self.active.insert(name.clone()) {
                            return false;
                        }
                        let ok = self.circuit(c);
                        self.active.remove(name);
                        ok
                    }
                    _ => false,
                }
            }
            _ => false,
        }
    }
    fn bindings(&mut self, b: &[LocalBinding]) -> bool {
        b.iter().all(|b| scalar(&b.ty) && self.value(&b.value))
    }
    fn action(&mut self, a: &StateAction) -> bool {
        match a {
            StateAction::Sequence { actions } => actions.iter().all(|a| self.action(a)),
            StateAction::Let { bindings, action } => self.bindings(bindings) && self.action(action),
            StateAction::If {
                condition,
                then,
                otherwise,
            } => self.value(condition) && self.action(then) && self.action(otherwise),
            StateAction::Assert { condition, .. } => self.value(condition),
            StateAction::CellWrite {
                field,
                index,
                value,
            } => self.cell(field, *index) && self.value(value),
            _ => false,
        }
    }
    fn circuit(&mut self, c: &StatefulCircuit) -> bool {
        c.result == Type::Field
            && c.parameters.iter().all(|p| scalar(&p.ty))
            && c.actions.iter().all(|a| self.action(a))
            && matches!(&c.return_value,StateReturn::Expression {value} if self.value(value))
    }
}
pub(super) fn lower<'a>(
    circuit: &StatefulCircuit,
    ledger: &'a HashMap<&'a str, &'a LedgerField>,
    witnesses: &'a HashMap<&'a str, &'a WitnessDeclaration>,
    pure: &'a HashMap<&'a str, &'a PureCircuit>,
    circuits: &'a HashMap<&'a str, &'a StatefulCircuit>,
) -> Option<TypedPlan> {
    let mut audit = Audit {
        ledger,
        witnesses,
        pure,
        circuits,
        active: HashSet::from([circuit.name.clone()]),
        slot: None,
    };
    if !audit.circuit(circuit) {
        return None;
    }
    let StateReturn::Expression { value } = &circuit.return_value else {
        return None;
    };
    let mut plan = Plan {
        ledger,
        witnesses,
        pure,
        next: 0,
        witness_calls: 0,
        kernel_self_reads: 0,
        context_query: false,
        root_observations: 0,
        tree_writes: 0,
        set_writes: 0,
        counter_writes: 0,
        counter_reads: 0,
        counter_comparisons: 0,
        cell_reads: 0,
        cell_writes: 0,
        field_cell_writes: 0,
        field_cell_slot: None,
        effectful_field_cells: true,
        read_only_assertions: false,
        unit_actions: false,
        phase_reset: false,
        composite_domain: CompositeDomain::TerminalReturns,
        composition_calls: None,
        composition_lookup_sites: Default::default(),
        intent_effects: 0,
        intent_queries: 0,
        zswap_inputs: 0,
        zswap_outputs: 0,
        counter_hash_helpers: false,
        scalar_arguments: false,
        scalar_body_depth: 0,
        scalar_helper_calls: 0,
        scalar_counter_reads: 0,
        active_calls: HashSet::new(),
        stateful_circuits: Some(circuits),
        optional_cells: 0,
        opaque_cells: 0,
        historic_roots: 0,
        historic_writes: 0,
        qualified_set_reads: 0,
        qualified_set_writes: 0,
        qualified_cell_writes: 0,
    };

    let scope: Scope = circuit
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
    if scope.len() != circuit.parameters.len() {
        return None;
    }
    let mut steps = Vec::new();
    let result = plan.return_plan(&adapt(&circuit.actions, value), &scope, &mut steps)?;
    (result.ty == Type::Field && plan.cell_reads > 0 && plan.field_cell_writes > 0).then_some(
        TypedPlan {
            steps,
            result: result.value,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};
    fn source() -> Value {
        serde_json::from_str(include_str!(
            "../../../tests/terminal-lexical-return-schema20-ir.json"
        ))
        .unwrap()
    }
    fn admitted(v: &Value, name: &str) -> Option<TypedPlan> {
        let c: crate::ir::Contract = serde_json::from_value(v.clone()).unwrap();
        lower(
            c.stateful_circuits.iter().find(|c| c.name == name)?,
            &c.ledger_fields.iter().map(|f| (f.id.as_str(), f)).collect(),
            &c.witnesses.iter().map(|w| (w.name.as_str(), w)).collect(),
            &c.circuits.iter().map(|p| (p.name.as_str(), p)).collect(),
            &c.stateful_circuits
                .iter()
                .map(|c| (c.name.as_str(), c))
                .collect(),
        )
    }
    fn first(v: &mut Value, kind: &str, change: &impl Fn(&mut Value)) -> bool {
        if v["kind"] == kind {
            change(v);
            return true;
        }
        match v {
            Value::Array(a) => a.iter_mut().any(|v| first(v, kind, change)),
            Value::Object(m) => m.values_mut().any(|v| first(v, kind, change)),
            _ => false,
        }
    }
    #[test]
    fn borrowed_facade_qualifies_normalized_and_raw_identifier_collisions() {
        for (name, parameter) in [("echo", "echo"), ("a$b", "a_b"), ("type", "type")] {
            let mut contract: crate::ir::Contract = serde_json::from_value(source()).unwrap();
            let circuit = &mut contract.stateful_circuits[2];
            circuit.name = name.into();
            circuit.parameters[0].name = parameter.into();
            for uses_witness in [false, true] {
                let method = crate::recorded::render_borrowed_recorded_contract_method(
                    circuit,
                    uses_witness,
                )
                .unwrap();
                let generated = quote::quote!(#method).to_string();
                assert!(
                    generated.contains("crate :: ledger_contract :: recorded ::"),
                    "{generated}"
                );
            }
        }
    }
    #[test]
    fn borrowed_facade_qualifies_fixed_context_without_source_parameters() {
        let mut contract: crate::ir::Contract = serde_json::from_value(source()).unwrap();
        let circuit = &mut contract.stateful_circuits[0];
        circuit.name = "context".into();
        assert!(circuit.parameters.is_empty());
        for uses_witness in [false, true] {
            let method =
                crate::recorded::render_borrowed_recorded_contract_method(circuit, uses_witness)
                    .unwrap();
            let generated = quote::quote!(#method).to_string();
            assert!(generated.contains("crate :: ledger_contract :: recorded :: context"));
        }
    }
    #[test]
    fn extracted_returns_share_the_evaluator_and_preserve_terminal_shadowing() {
        let source = source();
        for name in ["two", "three", "echo", "observed", "nested"] {
            assert!(admitted(&source, name).is_some(), "{name}");
        }
        let mut shadow = source.clone();
        shadow["stateful_circuits"][2]["parameters"][0]["name"] = json!("after");
        shadow["stateful_circuits"][2]["return_value"]["value"]["name"] = json!("after");
        let plan = admitted(&shadow, "echo").unwrap();
        let result = plan.result;
        assert!(
            !quote::quote!(#result)
                .to_string()
                .contains("__compact_param_0")
        );
        let mut repeated = source.clone();
        let call = repeated["stateful_circuits"][5]["return_value"]["value"].clone();
        repeated["stateful_circuits"][5]["return_value"]["value"] =
            json!({"kind":"add","left":call,"right":call});
        let steps = admitted(&repeated, "nested").unwrap().steps;
        let text = quote::quote!(#(#steps)*).to_string();
        assert_eq!(text.matches("record_read").count(), 2);
        assert_eq!(text.matches("record_write").count(), 2);
    }
    #[test]
    fn earlier_sibling_branch_and_helper_scopes_never_enclose_an_extracted_return() {
        let source = source();
        let reject = |name: &str, change: &dyn Fn(&mut Value)| {
            let mut v = source.clone();
            change(&mut v);
            assert!(admitted(&v, name).is_none());
        };
        reject("two", &|v| {
            v["stateful_circuits"][0]["actions"].as_array_mut().unwrap().push(json!({"kind":"cell_write","field":"stored","index":0,"value":{"kind":"field_literal","value":"0"}}));
        });
        reject("two", &|v| {
            v["stateful_circuits"][0]["actions"][0]["action"]["actions"].as_array_mut().unwrap().push(json!({"kind":"assert","condition":{"kind":"boolean","value":true},"message":"sibling"}));
        });
        reject("two", &|v| {
            let body = v["stateful_circuits"][0]["actions"][0].clone();
            v["stateful_circuits"][0]["actions"] = json!([{"kind":"if","condition":{"kind":"boolean","value":true},"then":body,"otherwise":body}]);
        });
        reject("nested", &|v| {
            v["stateful_circuits"][5]["actions"] = json!([{"kind":"let","bindings":[{"name":"caller_only","ty":{"kind":"field"},"value":{"kind":"field_literal","value":"99"}}],"action":{"kind":"sequence","actions":[]}}]);
            v["stateful_circuits"][4]["return_value"]["value"] =
                json!({"kind":"parameter","name":"caller_only"});
        });
        reject("nested", &|v| {
            let call = v["stateful_circuits"][5]["return_value"]["value"].clone();
            v["stateful_circuits"][5]["return_value"]["value"] = json!({"kind":"let","bindings":[{"name":"result","ty":{"kind":"field"},"value":call}],"body":{"kind":"parameter","name":"after"}});
        });
        reject("nested", &|v| {
            v["stateful_circuits"][4]["result"] = json!({"kind":"boolean"});
        });
        reject("nested", &|v| {
            v["stateful_circuits"][4]["return_value"]["value"] =
                json!({"kind":"call","name":"nested","arguments":[]});
        });
        reject("nested", &|v| {
            v["stateful_circuits"][5]["return_value"]["value"]["name"] = json!("missing");
        });
        reject("nested", &|v| {
            v["circuits"].as_array_mut().unwrap().push(json!({"name":"inner","parameters":[],"result":{"kind":"field"},"body":{"kind":"field_literal","value":"0"}}));
        });
        reject("two", &|v| {
            v["ledger_fields"][0]["declaration"]["ty"] = json!({"kind":"boolean"});
        });
        reject("two", &|v| {
            assert!(first(&mut v["stateful_circuits"][0], "cell_read", &|r| r
                ["index"] =
                json!(1)));
        });
        reject("observed", &|v| {
            v["witnesses"][0]["parameters"][0]["ty"] = json!({"kind":"boolean"});
        });
        reject("two", &|v| {
            v["stateful_circuits"][0]["actions"][0]["bindings"].as_array_mut().unwrap().push(json!({"name":"hidden","ty":{"kind":"field"},"value":{"kind":"counter_read","field":"unknown","index":0}}));
        });
        reject("two", &|v| {
            v["stateful_circuits"][0]["actions"][0]["bindings"].as_array_mut().unwrap().push(json!({"name":"hidden","ty":{"kind":"field"},"value":{"kind":"if","condition":{"kind":"boolean","value":false},"then":{"kind":"counter_read","field":"unknown","index":0},"otherwise":{"kind":"field_literal","value":"0"}}}));
        });
    }
}
