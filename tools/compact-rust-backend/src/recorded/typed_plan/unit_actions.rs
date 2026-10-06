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
//! Bounded Field→Unit Cell helper admission; execution is owned by the shared Plan.
use super::*;

pub(super) fn value_type(ty: &Type) -> bool {
    match ty {
        Type::Field | Type::Boolean | Type::Bytes { length: 32 } | Type::Enum { .. } => true,
        Type::Struct { fields, .. } => {
            !fields.is_empty() && fields.iter().all(|f| value_type(&f.ty))
        }
        _ => false,
    }
}
pub(super) fn field_structure(ty: &Type) -> bool {
    match ty {
        Type::Field => true,
        Type::Struct { fields, .. } => {
            !fields.is_empty() && fields.iter().all(|f| field_structure(&f.ty))
        }
        _ => false,
    }
}
pub(super) fn helper_signature(c: &StatefulCircuit) -> bool {
    c.result == Type::Unit
        && matches!(c.return_value, StateReturn::Unit)
        && c.parameters.len() == 1
        && c.parameters[0].ty == Type::Field
        && !c.actions.is_empty()
}
struct Audit<'a> {
    ledger: &'a HashMap<&'a str, &'a LedgerField>,
    witnesses: &'a HashMap<&'a str, &'a WitnessDeclaration>,
    pure: &'a HashMap<&'a str, &'a PureCircuit>,
    circuits: &'a HashMap<&'a str, &'a StatefulCircuit>,
    active: HashSet<String>,
    helpers: usize,
}
impl Audit<'_> {
    fn slot(&self, field: &str, index: u8) -> bool {
        self.ledger.get(field).is_some_and(|f| {
            f.index == index
                && f.physical_path() == [index]
                && matches!(&f.declaration, LedgerFieldKind::Cell {ty} if value_type(ty))
        })
    }
    fn value(&mut self, v: &Expr, pure: bool) -> bool {
        match v {
            Expr::Parameter { .. }
            | Expr::Boolean { .. }
            | Expr::FieldLiteral { .. }
            | Expr::BytesLiteral { .. }
            | Expr::EnumVariant { .. } => true,
            Expr::Unit => !pure,
            Expr::Default { ty } => value_type(ty),
            Expr::StructLiteral { ty, fields } => {
                value_type(ty) && fields.iter().all(|v| self.value(v, pure))
            }
            Expr::StructField { value, .. } => self.value(value, pure),
            Expr::PersistentHash { value } => pure && self.value(value, true),
            Expr::TransientCommit { value, opening } => {
                pure && self.value(value, true) && self.value(opening, true)
            }
            Expr::Coerce { value, ty } => value_type(ty) && self.value(value, pure),
            Expr::Equal { left, right } => self.value(left, pure) && self.value(right, pure),
            Expr::Tuple { elements } => elements.iter().all(|v| self.value(v, pure)),
            Expr::If {
                condition,
                then,
                otherwise,
            } => [condition, then, otherwise]
                .iter()
                .all(|v| self.value(v, pure)),
            Expr::Let { bindings, body } => {
                bindings
                    .iter()
                    .all(|b| value_type(&b.ty) && self.value(&b.value, pure))
                    && self.value(body, pure)
            }
            Expr::CellRead { field, index } => !pure && self.slot(field, *index),
            Expr::WitnessCall { name, arguments } => {
                !pure
                    && self.witnesses.get(name.as_str()).is_some_and(|w| {
                        arguments.is_empty() && w.parameters.is_empty() && value_type(&w.result)
                    })
            }
            Expr::Sequence { steps, value } => {
                !pure && steps.iter().all(|v| self.value(v, false)) && self.value(value, false)
            }
            Expr::Assert { condition, .. } => !pure && self.value(condition, false),
            Expr::Call { name, arguments } => {
                if !arguments.iter().all(|v| self.value(v, pure))
                    || !self.active.insert(name.clone())
                {
                    return false;
                }
                let valid = match (
                    self.pure.get(name.as_str()).copied(),
                    self.circuits.get(name.as_str()).copied(),
                ) {
                    (Some(c), None) => {
                        value_type(&c.result)
                            && c.parameters.iter().all(|p| value_type(&p.ty))
                            && self.value(&c.body, true)
                    }
                    (None, Some(c)) if !pure && helper_signature(c) => {
                        self.helpers += 1;
                        c.actions.iter().all(|a| self.action(a))
                    }
                    _ => false,
                };
                self.active.remove(name);
                valid
            }
            _ => false,
        }
    }
    fn action(&mut self, a: &StateAction) -> bool {
        match a {
            StateAction::Sequence { actions } => actions.iter().all(|a| self.action(a)),
            StateAction::Let { bindings, action } => {
                bindings
                    .iter()
                    .all(|b| value_type(&b.ty) && self.value(&b.value, false))
                    && self.action(action)
            }
            StateAction::If {
                condition,
                then,
                otherwise,
            } => self.value(condition, false) && self.action(then) && self.action(otherwise),
            StateAction::Assert { condition, .. } => self.value(condition, false),
            StateAction::CellWrite {
                field,
                index,
                value,
            } => self.slot(field, *index) && self.value(value, false),
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
    if circuit.result != Type::Unit
        || circuit.parameters.len() != 1
        || circuit.parameters[0].ty != Type::Field
        || !circuit.actions.is_empty()
    {
        return None;
    }
    let StateReturn::Expression { value } = &circuit.return_value else {
        return None;
    };
    let mut audit = Audit {
        ledger,
        witnesses,
        pure,
        circuits,
        active: HashSet::from([circuit.name.clone()]),
        helpers: 0,
    };
    if !audit.value(value, false) || audit.helpers == 0 {
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
        unit_actions: true,
        phase_reset: false,
        composite_domain: CompositeDomain::None,
        composition_calls: None,
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
    let p = &circuit.parameters[0];
    let name = ident("__compact_param_0").ok()?;
    let scope = Scope::from([(
        p.name.clone(),
        TypedValue {
            ty: p.ty.clone(),
            value: syn::parse_quote!(#name),
        },
    )]);
    let result = plan.expression(value, &scope, &mut steps)?;
    (result.ty == Type::Unit && plan.cell_writes > 0).then_some(TypedPlan {
        steps,
        result: result.value,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};
    fn planned(value: &Value) -> Option<TypedPlan> {
        let c: crate::ir::Contract = serde_json::from_value(value.clone()).unwrap();
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
    fn first_kind(value: &mut Value, kind: &str, change: &impl Fn(&mut Value)) -> bool {
        if value["kind"] == kind {
            change(value);
            return true;
        }
        match value {
            Value::Object(map) => map.values_mut().any(|v| first_kind(v, kind, change)),
            Value::Array(values) => values.iter_mut().any(|v| first_kind(v, kind, change)),
            _ => false,
        }
    }
    #[test]
    fn unit_action_helpers_check_scope_types_and_both_commitment_operands() {
        let source: Value = serde_json::from_str(include_str!(
            "../../../tests/coracle-guess-schema20-ir.json"
        ))
        .unwrap();
        let plan = planned(&source).unwrap();
        let steps = plan.steps;
        let tokens = quote::quote!(#(#steps)*).to_string();
        assert_eq!(tokens.matches("runtime :: transient_commit").count(), 2);
        assert_eq!(tokens.matches("last_guess . record_write").count(), 2);
        assert_eq!(tokens.matches("state . record_write").count(), 2);
        assert_eq!(tokens.matches("witnesses . local_board").count(), 2);
        let renamed = source
            .to_string()
            .replace("red_guess", "move_a")
            .replace("blue_guess", "move_b")
            .replace("is_player_honest", "valid_opening");
        assert!(planned(&serde_json::from_str(&renamed).unwrap()).is_some());
        let reject = |mutate: &dyn Fn(&mut Value)| {
            let mut changed = source.clone();
            mutate(&mut changed);
            assert!(planned(&changed).is_none());
        };
        reject(&|v| v["stateful_circuits"][0]["parameters"][0]["ty"] = json!({"kind":"boolean"}));
        reject(&|v| v["stateful_circuits"][1]["result"] = json!({"kind":"field"}));
        reject(&|v| v["stateful_circuits"][1]["parameters"] = json!([]));
        reject(&|v| {
            v["stateful_circuits"][1]["parameters"][0]["ty"] = json!({"kind":"bytes","length":32})
        });
        reject(&|v| {
            v["stateful_circuits"][1]["return_value"] =
                json!({"kind":"expression","value":{"kind":"unit"}})
        });
        reject(&|v| {
            assert!(first_kind(
                &mut v["stateful_circuits"][0],
                "cell_read",
                &|v| v["index"] = json!(8)
            ));
        });
        reject(&|v| {
            let slot = v["ledger_fields"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|f| f["id"] == "last_guess")
                .unwrap();
            slot["declaration"] = json!({"kind":"counter"});
        });
        reject(&|v| {
            assert!(first_kind(
                &mut v["stateful_circuits"][1],
                "parameter",
                &|v| v["name"] = json!("position")
            ));
        });
        reject(&|v| {
            assert!(first_kind(
                &mut v["stateful_circuits"][1],
                "struct_field",
                &|v| v["index"] = json!(99)
            ));
        });
        reject(&|v| {
            assert!(first_kind(
                &mut v["stateful_circuits"][0],
                "witness_call",
                &|v| v["arguments"] = json!([{"kind":"field_literal","value":"1"}])
            ));
        });
        reject(&|v| {
            let c = v["circuits"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|c| c["name"] == "commit")
                .unwrap();
            c["body"] = json!({"kind":"call","name":"commit","arguments":[{"kind":"parameter","name":"c"}]});
        });
        reject(&|v| {
            v["circuits"].as_array_mut().unwrap().push(json!({"name":"red_guess","parameters":[{"name":"x","ty":{"kind":"field"}}],"result":{"kind":"field"},"body":{"kind":"parameter","name":"x"}}));
        });
        reject(&|v| {
            *v.pointer_mut(
                "/stateful_circuits/0/return_value/value/body/body/body/value/otherwise",
            )
            .unwrap() = json!({"kind":"boolean","value":true});
        });
        reject(&|v| {
            let c = v["circuits"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|c| c["name"] == "commit")
                .unwrap();
            c["result"] = json!({"kind":"boolean"});
        });
        // Each operand is independently audited: both hidden effects have valid
        // result types and would otherwise add queries/private outputs.
        reject(&|v| {
            assert!(first_kind(&mut v["circuits"], "transient_commit", &|v| v
                ["opening"] = json!({"kind":"struct_field","field":"value","index":0,"value":{"kind":"cell_read","field":"red_board","index":2}})));
        });
        reject(&|v| {
            assert!(first_kind(&mut v["circuits"], "transient_commit", &|v| v
                ["value"] = json!({"kind":"struct_field","field":"contents","index":1,"value":{"kind":"witness_call","name":"local_board","arguments":[]}})));
        });
        reject(&|v| {
            let c = v["circuits"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|c| c["name"] == "is_red")
                .unwrap();
            let old = c["body"].clone();
            c["body"] = json!({"kind":"if","condition":{"kind":"boolean","value":false},"then":{"kind":"sequence","steps":[{"kind":"call","name":"red_guess","arguments":[{"kind":"field_literal","value":"1"}]}],"value":{"kind":"boolean","value":true}},"otherwise":old});
        });
        reject(&|v| {
            let coin = v["ledger_fields"]
                .as_array()
                .unwrap()
                .iter()
                .find(|f| f["id"] == "pot")
                .unwrap()["declaration"]["ty"]
                .clone();
            v["stateful_circuits"][1]["actions"].as_array_mut().unwrap().push(json!({"kind":"cell_write","field":"pot","index":6,"value":{"kind":"default","ty":coin}}));
        });
    }
}
