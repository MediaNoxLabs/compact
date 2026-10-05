// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Structural policy for read-only Field observations in composite results.
//! This audits every branch and binding; the shared Plan owns evaluation,
//! concrete types, lexical scope, argument ordering and same-frame calls.
use super::*;

fn value_type(ty: &Type) -> bool {
    match ty {
        Type::Field | Type::Boolean => true,
        Type::Struct { fields, .. } => {
            !fields.is_empty() && fields.iter().all(|f| value_type(&f.ty))
        }
        _ => false,
    }
}
struct Audit<'a> {
    ledger: &'a HashMap<&'a str, &'a LedgerField>,
    pure: &'a HashMap<&'a str, &'a PureCircuit>,
    circuits: &'a HashMap<&'a str, &'a StatefulCircuit>,
    visiting: HashSet<String>,
}
impl Audit<'_> {
    fn read(&self, field: &str, index: u8) -> bool {
        self.ledger.get(field).is_some_and(|f| {
            f.index == index
                && f.physical_path() == [index]
                && f.declaration == (LedgerFieldKind::Cell { ty: Type::Field })
        })
    }
    fn value(&mut self, value: &Expr) -> bool {
        match value {
            Expr::Parameter { .. } | Expr::Boolean { .. } | Expr::FieldLiteral { .. } => true,
            Expr::CellRead { field, index } => self.read(field, *index),
            Expr::StructLiteral { ty, fields } => {
                matches!(ty, Type::Struct { .. })
                    && value_type(ty)
                    && fields.iter().all(|v| self.value(v))
            }
            Expr::Coerce { value, ty } => value_type(ty) && self.value(value),
            Expr::If {
                condition,
                then,
                otherwise,
            } => self.value(condition) && self.value(then) && self.value(otherwise),
            Expr::Let { bindings, body } => {
                bindings
                    .iter()
                    .all(|b| value_type(&b.ty) && self.value(&b.value))
                    && self.value(body)
            }
            Expr::Call { name, arguments } => {
                if self.pure.contains_key(name.as_str())
                    || !arguments.iter().all(|v| self.value(v))
                    || !self.visiting.insert(name.clone())
                {
                    return false;
                }
                let valid = self
                    .circuits
                    .get(name.as_str())
                    .copied()
                    .is_some_and(|callee| self.circuit(callee));
                self.visiting.remove(name);
                valid
            }
            _ => false,
        }
    }
    fn circuit(&mut self, circuit: &StatefulCircuit) -> bool {
        circuit.actions.is_empty()
            && value_type(&circuit.result)
            && circuit.parameters.iter().all(|p| value_type(&p.ty))
            && match &circuit.return_value {
                StateReturn::Expression { value } => self.value(value),
                StateReturn::CellRead { field, index } => {
                    circuit.result == Type::Field && self.read(field, *index)
                }
                _ => false,
            }
    }
}
pub(super) fn audit<'a>(
    circuit: &StatefulCircuit,
    ledger: &'a HashMap<&'a str, &'a LedgerField>,
    pure: &'a HashMap<&'a str, &'a PureCircuit>,
    circuits: &'a HashMap<&'a str, &'a StatefulCircuit>,
) -> bool {
    matches!(circuit.result, Type::Struct { .. })
        && Audit {
            ledger,
            pure,
            circuits,
            visiting: HashSet::from([circuit.name.clone()]),
        }
        .circuit(circuit)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};
    fn planned(source: &Value, index: usize) -> Option<TypedPlan> {
        let c: crate::ir::Contract = serde_json::from_value(source.clone()).unwrap();
        let ledger = c.ledger_fields.iter().map(|f| (f.id.as_str(), f)).collect();
        let witnesses = c.witnesses.iter().map(|w| (w.name.as_str(), w)).collect();
        let pure = c.circuits.iter().map(|p| (p.name.as_str(), p)).collect();
        let circuits = c
            .stateful_circuits
            .iter()
            .map(|s| (s.name.as_str(), s))
            .collect();
        lower_composite(
            &c.stateful_circuits[index],
            &ledger,
            &witnesses,
            &pure,
            &circuits,
        )
    }
    #[test]
    fn observations_are_typed_read_only_scoped_and_acyclic() {
        let source: Value = serde_json::from_str(include_str!(
            "../../../tests/field-observation-schema20-ir.json"
        ))
        .unwrap();
        for index in [1, 4, 5, 6, 7, 8] {
            assert!(planned(&source, index).is_some(), "{index}");
        }
        let reject = |index, pointer: &str, value: Value| {
            let mut altered = source.clone();
            *altered.pointer_mut(pointer).unwrap() = value;
            assert!(planned(&altered, index).is_none(), "{pointer}");
        };
        reject(1, "/ledger_fields", json!([]));
        reject(1, "/ledger_fields/0/index", json!(1));
        reject(1, "/ledger_fields/0/path", json!([1]));
        reject(1, "/ledger_fields/0/path", json!([0, 1]));
        reject(
            1,
            "/ledger_fields/0/declaration/ty",
            json!({"kind":"boolean"}),
        );
        reject(
            1,
            "/stateful_circuits/1/return_value/value/fields",
            json!([]),
        );
        reject(
            1,
            "/stateful_circuits/1/return_value/value/fields/0",
            json!({"kind":"boolean","value":true}),
        );
        reject(
            1,
            "/stateful_circuits/1/return_value/value/fields/0",
            json!({"kind":"parameter","name":"leaked"}),
        );
        reject(4, "/stateful_circuits/2/result", json!({"kind":"boolean"}));
        reject(
            4,
            "/stateful_circuits/2/parameters",
            json!([{"name":"missing","ty":{"kind":"field"}}]),
        );
        reject(
            5,
            "/stateful_circuits/3/return_value/value",
            json!({"kind":"call","name":"via_snapshot_helper","arguments":[]}),
        );
        reject(
            7,
            "/stateful_circuits/7/return_value/value/condition",
            json!({"kind":"field_literal","value":"1"}),
        );
        reject(
            7,
            "/stateful_circuits/7/return_value/value/otherwise",
            json!({"kind":"field_literal","value":"1"}),
        );
        let write = json!({"kind":"cell_write","field":"first_cell","index":0,"value":{"kind":"field_literal","value":"1"}});
        reject(1, "/stateful_circuits/1/actions", json!([write.clone()]));
        reject(7, "/stateful_circuits/3/actions", json!([write]));
        for hidden in [
            json!({"kind":"witness_call","name":"hidden","arguments":[]}),
            json!({"kind":"counter_read","field":"first_cell","index":0}),
            json!({"kind":"kernel_self","ty":{"kind":"field"}}),
        ] {
            // Even an unused binding in a statically unselected branch is audited.
            let original =
                source["stateful_circuits"][8]["return_value"]["value"]["otherwise"].clone();
            reject(
                8,
                "/stateful_circuits/8/return_value/value/otherwise",
                json!({"kind":"let","bindings":[{"name":"unused","ty":{"kind":"field"},"value":hidden}],"body":original}),
            );
        }
        // Caller-only bindings do not enter the helper's isolated scope.
        reject(
            5,
            "/stateful_circuits/3/return_value/value/fields/0",
            json!({"kind":"parameter","name":"caller_only"}),
        );
        // Read arguments must be materialized once, in caller order, even when unused.
        let mut args = source.clone();
        args["stateful_circuits"][2]["parameters"] =
            json!([{"name":"first","ty":{"kind":"field"}},{"name":"second","ty":{"kind":"field"}}]);
        args["stateful_circuits"][4]["return_value"]["value"]["fields"][0]["arguments"] = json!([
            {"kind":"cell_read","field":"second_cell","index":1},
            {"kind":"cell_read","field":"first_cell","index":0}
        ]);
        let p = planned(&args, 4).unwrap();
        let steps = p.steps;
        let rendered = quote::quote!(#(#steps)*).to_string();
        assert_eq!(rendered.matches("record_read").count(), 3);
        assert!(rendered.find("second_cell").unwrap() < rendered.find("first_cell").unwrap());
    }
    #[test]
    fn admission_domains_preserve_receive_and_phase_reset_routing() {
        assert!(!CompositeDomain::None.values());
        assert!(!CompositeDomain::None.intents());
        assert!(CompositeDomain::Values.values());
        assert!(!CompositeDomain::Values.intents());
        assert!(CompositeDomain::Intents.values());
        assert!(CompositeDomain::Intents.intents());
        assert!(!CompositeDomain::ShieldedReceive.values());
        assert!(CompositeDomain::ShieldedReceive.intents());
        assert!(CompositeDomain::FieldObservations.values());
        assert!(!CompositeDomain::FieldObservations.intents());
    }
}
