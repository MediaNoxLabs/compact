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
//! Audited no-argument phase advancement with a literal-false reset helper.
//! The shared Plan owns typed values, lexical scopes and ordered execution.
use super::*;
pub(super) const WIDENED: &str = "18446744073709551616";
pub(super) const SUM: &str = "36893488147419103231";
const U64: &str = "18446744073709551615";
pub(super) fn helper_signature(c: &StatefulCircuit) -> bool {
    c.internal
        && c.result == Type::Unit
        && c.return_value == StateReturn::Unit
        && matches!(c.parameters.as_slice(), [p] if p.ty == Type::Boolean)
        && !c.actions.is_empty()
}
pub(super) fn false_arguments(args: &[Expr]) -> bool {
    fn is_false(v: &Expr) -> bool {
        match v {
            Expr::Boolean { value: false } => true,
            Expr::Coerce {
                value,
                ty: Type::Boolean,
            } => is_false(value),
            _ => false,
        }
    }
    matches!(args, [v] if is_false(v))
}
pub(super) fn cell_type(ty: &Type) -> bool {
    super::cell_type(ty)
        || *ty == Type::Boolean
        || *ty == crate::stateful::qualified_coin_type()
        || matches!(ty, Type::Struct { fields, .. } if matches!(fields.as_slice(), [tag, value] if tag.name == "is_some" && tag.ty == Type::Boolean && value.name == "value" && matches!(&value.ty, Type::Struct { name, fields } if name == "ZswapCoinPublicKey" && matches!(fields.as_slice(), [bytes] if bytes.name == "bytes" && bytes.ty == (Type::Bytes { length: 32 })))))
}
struct Audit<'a> {
    ledger: &'a HashMap<&'a str, &'a LedgerField>,
    witnesses: &'a HashMap<&'a str, &'a WitnessDeclaration>,
    pure: &'a HashMap<&'a str, &'a PureCircuit>,
    circuits: &'a HashMap<&'a str, &'a StatefulCircuit>,
    helpers: usize,
    counters: usize,
    trees: usize,
    sets: usize,
    increments: usize,
}
impl Audit<'_> {
    fn field(&self, field: &str, index: u8) -> Option<&LedgerFieldKind> {
        let f = self.ledger.get(field)?;
        (f.index == index && f.physical_path() == [index]).then_some(&f.declaration)
    }
    fn value(&self, v: &Expr) -> bool {
        match v {
            Expr::Parameter { .. }
            | Expr::Boolean { .. }
            | Expr::BytesLiteral { .. }
            | Expr::EnumVariant { .. } => true,
            Expr::UnsignedLiteral { max, .. } => matches!(max.as_str(), "65535" | U64 | WIDENED),
            Expr::Default { ty } => *ty == crate::stateful::qualified_coin_type(),
            Expr::Coerce { value, .. } => self.value(value),
            Expr::Equal { left, right } => self.value(left) && self.value(right),
            Expr::If {
                condition,
                then,
                otherwise,
            } => [condition, then, otherwise].iter().all(|v| self.value(v)),
            Expr::Let { bindings, body } => {
                bindings.iter().all(|b| self.value(&b.value)) && self.value(body)
            }
            Expr::Call { name, arguments } => {
                self.pure.contains_key(name.as_str()) && arguments.iter().all(|v| self.value(v))
            }
            Expr::WitnessCall { name, arguments } => {
                arguments.is_empty()
                    && self.witnesses.get(name.as_str()).is_some_and(|w| {
                        w.parameters.is_empty() && w.result == (Type::Bytes { length: 32 })
                    })
            }
            Expr::CellRead { field, index } => matches!(
                self.field(field, *index),
                Some(LedgerFieldKind::Cell {
                    ty: Type::Enum { .. } | Type::Bytes { length: 32 }
                })
            ),
            Expr::CounterRead { field, index } => {
                self.field(field, *index) == Some(&LedgerFieldKind::Counter)
            }
            Expr::CounterLessThan {
                field,
                index,
                threshold,
            } => {
                self.field(field, *index) == Some(&LedgerFieldKind::Counter)
                    && self.value(threshold)
            }
            Expr::UnsignedAdd { max, left, right } => {
                max == SUM && self.value(left) && self.value(right)
            }
            Expr::UnsignedCast { max, value } => {
                matches!(max.as_str(), U64 | WIDENED) && self.value(value)
            }
            _ => false,
        }
    }
    fn action(&mut self, a: &StateAction, helper: bool) -> bool {
        match a {
            StateAction::Sequence { actions } => actions.iter().all(|a| self.action(a, helper)),
            StateAction::Let { bindings, action } => {
                bindings.iter().all(|b| self.value(&b.value)) && self.action(action, helper)
            }
            StateAction::If {
                condition,
                then,
                otherwise,
            } => {
                self.value(condition) && self.action(then, helper) && self.action(otherwise, helper)
            }
            StateAction::Assert { condition, .. } => !helper && self.value(condition),
            StateAction::CellWrite {
                field,
                index,
                value,
            } => {
                matches!(self.field(field,*index),Some(LedgerFieldKind::Cell {ty}) if (helper && cell_type(ty)) || matches!(ty,Type::Enum {..}))
                    && self.value(value)
            }
            StateAction::CounterReset { field, index } if helper => {
                self.counters += 1;
                self.field(field, *index) == Some(&LedgerFieldKind::Counter)
            }
            StateAction::MerkleResetToDefault { field, index } if helper => {
                self.trees += 1;
                matches!(
                    self.field(field, *index),
                    Some(LedgerFieldKind::MerkleTree {
                        depth: 10,
                        ty: Type::Bytes { length: 32 }
                    })
                )
            }
            StateAction::SetReset { field, index } if helper => {
                self.sets += 1;
                matches!(
                    self.field(field, *index),
                    Some(LedgerFieldKind::Set {
                        ty: Type::Bytes { length: 32 }
                    })
                )
            }
            StateAction::CounterIncrement { field, index, .. } if helper => {
                self.increments += 1;
                self.field(field, *index) == Some(&LedgerFieldKind::Counter)
            }
            StateAction::CircuitCall { name, arguments }
                if !helper && false_arguments(arguments) =>
            {
                let Some(c) = self.circuits.get(name.as_str()).copied() else {
                    return false;
                };
                if self.pure.contains_key(name.as_str()) || !helper_signature(c) {
                    return false;
                }
                self.helpers += 1;
                c.actions.iter().all(|a| self.action(a, true))
            }
            _ => false,
        }
    }
}
pub(super) fn lower<'a>(
    circuit: &StatefulCircuit,
    ledger: &'a HashMap<&'a str, &'a LedgerField>,
    witnesses: &'a HashMap<&'a str, &'a WitnessDeclaration>,
    pure: &'a HashMap<&'a str, &'a PureCircuit>,
    circuits: &'a HashMap<&'a str, &'a StatefulCircuit>,
) -> Option<TypedPlan> {
    if circuit.internal
        || !circuit.parameters.is_empty()
        || circuit.result != Type::Unit
        || circuit.return_value != StateReturn::Unit
        || !matches!(circuit.actions.as_slice(), [StateAction::Let { .. }])
    {
        return None;
    }
    let mut audit = Audit {
        ledger,
        witnesses,
        pure,
        circuits,
        helpers: 0,
        counters: 0,
        trees: 0,
        sets: 0,
        increments: 0,
    };
    if !circuit.actions.iter().all(|a| audit.action(a, false))
        || (
            audit.helpers,
            audit.counters,
            audit.trees,
            audit.sets,
            audit.increments,
        ) != (1, 2, 1, 2, 1)
    {
        return None;
    }
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
        effectful_field_cells: false,
        read_only_assertions: false,
        unit_actions: false,
        phase_reset: true,
        composite_domain: CompositeDomain::None,
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

    let mut steps = Vec::new();
    for action in &circuit.actions {
        plan.action(action, &Scope::new(), &mut steps)?;
    }
    (plan.witness_calls == 1
        && plan.counter_reads == 1
        && plan.counter_comparisons == 1
        && plan.counter_writes == 3
        && plan.tree_writes == 1
        && plan.set_writes == 2)
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
            "../../../tests/micro-dao-advance-schema20-ir.json"
        ))
        .unwrap()
    }
    fn admitted(v: &Value) -> Option<TypedPlan> {
        let c: crate::ir::Contract = serde_json::from_value(v.clone()).unwrap();
        lower(
            &c.stateful_circuits[0],
            &c.ledger_fields.iter().map(|f| (f.id.as_str(), f)).collect(),
            &c.witnesses.iter().map(|w| (w.name.as_str(), w)).collect(),
            &c.circuits.iter().map(|p| (p.name.as_str(), p)).collect(),
            &c.stateful_circuits
                .iter()
                .map(|s| (s.name.as_str(), s))
                .collect(),
        )
    }
    fn first(v: &mut Value, kind: &str, change: &impl Fn(&mut Value)) -> bool {
        if v["kind"] == kind {
            change(v);
            return true;
        }
        match v {
            Value::Object(m) => m.values_mut().any(|v| first(v, kind, change)),
            Value::Array(a) => a.iter_mut().any(|v| first(v, kind, change)),
            _ => false,
        }
    }
    #[test]
    fn phase_reset_preserves_whole_helper_scope_types_and_false_call_boundary() {
        let source = source();
        let plan = admitted(&source).expect("original advance admitted");
        let steps = plan.steps;
        let text = quote::quote!(#(#steps)*).to_string();
        assert!(text.contains("add_unsigned"));
        assert!(text.contains("record_reset_to_default"));
        assert!(text.contains("ledger_slots :: pot . record_write"));
        let reject = |change: &dyn Fn(&mut Value)| {
            let mut v = source.clone();
            change(&mut v);
            assert!(admitted(&v).is_none());
        };
        reject(&|v| {
            assert!(first(
                &mut v["stateful_circuits"][0],
                "circuit_call",
                &|x| x["arguments"] = json!([{"kind":"boolean","value":true}])
            ));
        });
        reject(&|v| {
            assert!(first(
                &mut v["stateful_circuits"][0],
                "circuit_call",
                &|x| x["arguments"] = json!([{"kind":"parameter","name":"dynamic"}])
            ));
        });
        reject(&|v| {
            assert!(first(
                &mut v["stateful_circuits"][0],
                "unsigned_add",
                &|x| x["max"] = json!("18446744073709551615")
            ));
        });
        reject(&|v| {
            assert!(first(
                &mut v["stateful_circuits"][0],
                "unsigned_add",
                &|x| x["right"] = json!({"kind":"boolean","value":true})
            ));
        });
        reject(&|v| {
            assert!(first(
                &mut v["stateful_circuits"][1],
                "counter_reset",
                &|x| x["index"] = json!(0)
            ));
        });
        reject(&|v| {
            v["ledger_fields"][7]["declaration"]["depth"] = json!(9);
        });
        reject(&|v| {
            v["ledger_fields"][8]["declaration"]["ty"] = json!({"kind":"field"});
        });
        reject(&|v| {
            v["ledger_fields"][3]["declaration"]["ty"] = json!({"kind":"boolean"});
        });
        reject(&|v| {
            v["stateful_circuits"][1]["parameters"][0]["ty"] = json!({"kind":"field"});
        });
        reject(&|v| {
            v["stateful_circuits"][1]["result"] = json!({"kind":"boolean"});
        });
        reject(&|v| {
            v["stateful_circuits"][1]["internal"] = json!(false);
        });
        reject(&|v| {
            v["stateful_circuits"][1]["actions"].as_array_mut().unwrap().push(json!({"kind":"circuit_call","name":"reset_state","arguments":[{"kind":"boolean","value":false}]}));
        });
        reject(&|v| {
            v["stateful_circuits"][1]["actions"].as_array_mut().unwrap().last_mut().unwrap()["then"]["actions"].as_array_mut().unwrap().push(json!({"kind":"expression","value":{"kind":"kernel_self","ty":{"kind":"struct","name":"ContractAddress","fields":[{"name":"bytes","ty":{"kind":"bytes","length":32}}]}}}));
        });
        reject(&|v| {
            v["stateful_circuits"][1]["actions"][0]["value"] =
                json!({"kind":"parameter","name":"sk"});
        });
        reject(&|v| {
            let p = v["circuits"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|p| p["name"] == "successor")
                .unwrap();
            p["body"] = json!({"kind":"call","name":"successor","arguments":[{"kind":"parameter","name":"state"}]});
        });
        reject(&|v| {
            let p = v["circuits"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|p| p["name"] == "public_key")
                .unwrap();
            p["result"] = json!({"kind":"boolean"});
        });
    }
}
