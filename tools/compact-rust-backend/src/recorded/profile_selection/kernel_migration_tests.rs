// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Old-first kernel grammar controls, separate from new compositions.
use super::*;
use crate::ir::{
    Contract, Expr, KernelClaimKind, LocalBinding, Parameter, StateAction, StateReturn, Type,
};
use quote::quote;
#[path = "kernel_migration_tests/inclusion.rs"]
mod inclusion;
#[path = "kernel_migration_tests/routing.rs"]
mod routing;

fn bytes_type() -> Type {
    Type::Bytes { length: 32 }
}
fn amount_type() -> Type {
    Type::Unsigned {
        max: u64::MAX.to_string(),
    }
}
fn parameter(name: &str) -> Expr {
    Expr::Parameter { name: name.into() }
}
fn witness(name: &str) -> Expr {
    Expr::WitnessCall {
        name: name.into(),
        arguments: vec![],
    }
}
fn claim(kind: KernelClaimKind, value: Expr) -> Expr {
    Expr::KernelClaim {
        claim: kind,
        value: Box::new(value),
    }
}
fn mint(domain: Expr, amount: Expr) -> Expr {
    Expr::KernelMintShielded {
        domain: Box::new(domain),
        amount: Box::new(amount),
    }
}
fn action(value: Expr) -> StateAction {
    StateAction::Expression { value }
}
fn empty() -> StateAction {
    StateAction::Sequence { actions: vec![] }
}
fn subject() -> Contract {
    let types = [
        ("selected", Type::Boolean),
        ("domain", bytes_type()),
        ("amount", amount_type()),
    ];
    Contract {
        schema_version: crate::ir::SCHEMA_VERSION,
        type_aliases: vec![],
        ledger_fields: vec![],
        constructor: None,
        circuits: vec![],
        witnesses: types
            .iter()
            .map(|(name, ty)| WitnessDeclaration {
                source: None,
                name: format!("get_{name}"),
                parameters: vec![],
                result: ty.clone(),
            })
            .collect(),
        stateful_circuits: vec![StatefulCircuit {
            source: None,
            name: "root".into(),
            internal: false,
            parameters: types
                .into_iter()
                .map(|(name, ty)| Parameter {
                    name: name.into(),
                    ty,
                })
                .collect(),
            actions: vec![],
            result: Type::Unit,
            return_value: StateReturn::Unit,
        }],
    }
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
        crate::recorded::kernel_plan::lower(c, &self.witnesses)
    }
    fn algebra(&self, c: &StatefulCircuit) -> Result<TypedPlan, effect_plan::PlanError> {
        effect_plan::lower_with_calls(c, &self.ledger, &self.witnesses, &self.pure, &self.circuits)
    }
    fn domain(&self, c: &StatefulCircuit) -> Result<Option<TypedPlan>, RenderError> {
        checked_kernel_domain(c, &self.ledger, &self.witnesses, &self.pure, &self.circuits)
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
fn included(contract: &Contract, name: &str, label: &str) {
    let root = contract
        .stateful_circuits
        .iter()
        .find(|c| c.name == name)
        .unwrap();
    let context = Context::new(contract);
    assert!(context.old(root).is_some(), "old-FIRST {label}");
    let candidate = context
        .algebra(root)
        .unwrap_or_else(|e| panic!("algebra {label}: {e:?}"));
    assert_eq!(
        tokens(&context.domain(root).unwrap().unwrap()),
        tokens(&candidate),
        "{label}"
    );
    let rendered =
        crate::render_with_capabilities(contract).unwrap_or_else(|e| panic!("render {label}: {e}"));
    let exports: Vec<_> = rendered
        .capabilities
        .circuits
        .iter()
        .filter(|c| c.name == name)
        .collect();
    assert_eq!(exports.len(), 1, "{label}");
    assert!(exports[0].recorded && exports[0].observed_call, "{label}");
}
