// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Finite whole-family old-first controls, separate from compositional expansions.
use super::*;
use crate::ir::{
    Contract, Expr, LocalBinding, Parameter, StateAction, StateReturn, StructField, Type,
};
use crate::recorded::admission_probe::trace;
use quote::quote;
#[path = "receive_migration_tests/actions.rs"]
mod actions;
#[path = "receive_migration_tests/negative.rs"]
mod negative;
#[path = "receive_migration_tests/routing.rs"]
mod routing;
#[path = "receive_migration_tests/values.rs"]
mod values;

fn source() -> Contract {
    serde_json::from_str(include_str!(
        "../../../tests/shielded-receive-schema20-ir.json"
    ))
    .unwrap()
}
fn root(c: &Contract) -> &StatefulCircuit {
    c.stateful_circuits
        .iter()
        .find(|c| c.name == "accept")
        .unwrap()
}
fn root_mut(c: &mut Contract) -> &mut StatefulCircuit {
    c.stateful_circuits
        .iter_mut()
        .find(|c| c.name == "accept")
        .unwrap()
}
fn helper_mut(c: &mut Contract) -> &mut StatefulCircuit {
    c.stateful_circuits
        .iter_mut()
        .find(|c| c.name == "receiveShielded")
        .unwrap()
}
fn parameter(name: &str) -> Expr {
    Expr::Parameter { name: name.into() }
}
fn field(value: u64) -> Expr {
    Expr::FieldLiteral {
        value: value.to_string(),
    }
}
fn bytes() -> Expr {
    Expr::BytesLiteral { bytes: vec![7; 32] }
}
fn number(value: u64, max: &str) -> Expr {
    Expr::UnsignedLiteral {
        value: value.to_string(),
        max: max.into(),
    }
}
fn structure(name: &str, ty: Type) -> Type {
    Type::Struct {
        name: name.into(),
        fields: vec![StructField {
            name: "value".into(),
            ty,
        }],
    }
}
fn local(name: &str, ty: Type, value: Expr, body: Expr) -> Expr {
    Expr::Let {
        bindings: vec![LocalBinding {
            name: name.into(),
            ty,
            value,
        }],
        body: Box::new(body),
    }
}
fn prepend(c: &mut Contract, ty: Type, value: Expr) {
    let StateAction::Let { bindings, .. } = &mut helper_mut(c).actions[0] else {
        panic!("retained helper changed")
    };
    bindings.insert(
        0,
        LocalBinding {
            name: "probe".into(),
            ty,
            value,
        },
    );
}
struct Context<'a> {
    ledger: HashMap<&'a str, &'a LedgerField>,
    witnesses: HashMap<&'a str, &'a WitnessDeclaration>,
    pure: HashMap<&'a str, &'a PureCircuit>,
    circuits: HashMap<&'a str, &'a StatefulCircuit>,
}
impl<'a> Context<'a> {
    fn new(c: &'a Contract) -> Self {
        Self {
            ledger: c.ledger_fields.iter().map(|v| (v.id.as_str(), v)).collect(),
            witnesses: c.witnesses.iter().map(|v| (v.name.as_str(), v)).collect(),
            pure: c.circuits.iter().map(|v| (v.name.as_str(), v)).collect(),
            circuits: c
                .stateful_circuits
                .iter()
                .map(|v| (v.name.as_str(), v))
                .collect(),
        }
    }
    fn old(&self, c: &StatefulCircuit) -> Option<TypedPlan> {
        typed_plan::lower_shielded_receive(
            c,
            &self.ledger,
            &self.witnesses,
            &self.pure,
            &self.circuits,
        )
    }
    fn algebra(&self, c: &StatefulCircuit) -> Result<TypedPlan, effect_plan::PlanError> {
        effect_plan::lower_with_calls(c, &self.ledger, &self.witnesses, &self.pure, &self.circuits)
    }
    fn domain(&self, c: &StatefulCircuit) -> Result<Option<TypedPlan>, RenderError> {
        checked_receive_domain(c, &self.ledger, &self.witnesses, &self.pure, &self.circuits)
    }
    fn selected(&self, c: &StatefulCircuit) -> Result<ProfileSelection, RenderError> {
        select(
            c,
            &self.ledger,
            &self.witnesses,
            &self.pure,
            &self.circuits,
            None,
        )
    }
}
fn tokens(plan: &TypedPlan) -> String {
    let (steps, result) = (&plan.steps, &plan.result);
    quote!(#(#steps)* #result).to_string()
}
fn public_owner(c: &Contract, name: &str) {
    let (output, rows) = trace::capture_json(|| crate::render_with_capabilities(c));
    let output = output.unwrap();
    let exports = output
        .capabilities
        .circuits
        .iter()
        .filter(|e| e.name == name)
        .collect::<Vec<_>>();
    assert_eq!(exports.len(), 1, "export {name}");
    assert!(
        exports[0].recorded && exports[0].observed_call,
        "export {name}"
    );
    let rows = rows
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["circuit"] == name && r["phase"] == "public_emission" && r["helper"] == false)
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), 1, "actual invocation {name}");
    assert_eq!(rows[0]["body_owner"], "algebra.shielded-receive");
    assert_eq!(rows[0]["outcome"]["status"], "supported");
    assert_eq!(
        rows[0]["attempts"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|a| a["route"] == "algebra.shielded-receive")
            .count(),
        1
    );
}
fn included(c: &Contract, name: &str, label: &str) {
    let root = c.stateful_circuits.iter().find(|r| r.name == name).unwrap();
    let maps = Context::new(c);
    let old = maps
        .old(root)
        .unwrap_or_else(|| panic!("old-FIRST {label}"));
    let old = tokens(&old);
    for operation in ["kernel_self", "create_zswap_output", "CoinReceive"] {
        assert_eq!(
            old.matches(operation).count(),
            1,
            "{label}: old evidence {operation}"
        );
    }
    let checked = maps
        .algebra(root)
        .unwrap_or_else(|e| panic!("checked {label}: {e:?}"));
    assert_eq!(
        tokens(&maps.domain(root).unwrap().unwrap()),
        tokens(&checked),
        "{label}"
    );
    if root.internal {
        let (item, rows) = trace::capture_json(|| {
            let _phase = trace::HelperPlanning::begin();
            crate::recorded::render_recorded_helper(
                root,
                &maps.ledger,
                &maps.witnesses,
                &maps.pure,
                &maps.circuits,
                &std::collections::HashSet::new(),
            )
        });
        assert!(item.unwrap().is_supported(), "internal {label}");
        assert_eq!(rows.as_array().unwrap().len(), 1);
        assert_eq!(rows[0]["phase"], "helper_planning");
        assert_eq!(rows[0]["helper"], true);
        assert_eq!(rows[0]["body_owner"], "algebra.shielded-receive");
    } else {
        public_owner(c, name);
    }
}
#[test]
fn original_aliases_and_internal_wrapper_have_actual_algebra_owners() {
    let c = source();
    let exports = c
        .stateful_circuits
        .iter()
        .filter(|r| !r.internal)
        .map(|r| r.name.as_str())
        .collect::<Vec<_>>();
    assert_eq!(exports, ["accept", "accept_renamed"]);
    for name in exports {
        included(&c, name, name);
    }
    let mut c = source();
    root_mut(&mut c).internal = true;
    included(&c, "accept", "internal old wrapper");
}
