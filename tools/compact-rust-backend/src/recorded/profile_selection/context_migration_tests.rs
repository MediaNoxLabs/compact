// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Old-first context-query obligations, separate from rollout expansion.
use super::*;
use crate::ir::{Contract, Expr, LocalBinding, Parameter, StateReturn, StructField, Type};
use quote::quote;
#[path = "context_migration_tests/routing.rs"]
mod routing;

fn bytes_type() -> Type {
    Type::Bytes { length: 32 }
}
fn address_type() -> Type {
    Type::Struct {
        name: "ContractAddress".into(),
        fields: vec![StructField {
            name: "bytes".into(),
            ty: bytes_type(),
        }],
    }
}
fn bytes() -> Expr {
    Expr::BytesLiteral { bytes: vec![7; 32] }
}
fn parameter(name: &str) -> Expr {
    Expr::Parameter { name: name.into() }
}
fn address() -> Expr {
    Expr::KernelSelf { ty: address_type() }
}
fn project(value: Expr) -> Expr {
    Expr::StructField {
        value: Box::new(value),
        field: "bytes".into(),
        index: 0,
    }
}
fn commit(left: Expr, right: Expr, opening: Expr) -> Expr {
    Expr::PersistentCommit {
        value: Box::new(Expr::Tuple {
            elements: vec![left, right],
        }),
        opening: Box::new(opening),
    }
}
fn call(name: &str, arguments: Vec<Expr>) -> Expr {
    Expr::Call {
        name: name.into(),
        arguments,
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
fn subject(value: Expr) -> Contract {
    Contract {
        schema_version: crate::ir::SCHEMA_VERSION,
        type_aliases: vec![],
        ledger_fields: vec![],
        constructor: None,
        witnesses: vec![],
        circuits: vec![],
        stateful_circuits: vec![StatefulCircuit {
            source: None,
            name: "root".into(),
            internal: false,
            parameters: vec![],
            actions: vec![],
            result: bytes_type(),
            return_value: StateReturn::Expression { value },
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
        typed_plan::lower_context_query(c, &self.ledger, &self.witnesses, &self.pure)
    }
    fn algebra(&self, c: &StatefulCircuit) -> Result<TypedPlan, effect_plan::PlanError> {
        effect_plan::lower_with_calls(c, &self.ledger, &self.witnesses, &self.pure, &self.circuits)
    }
    fn domain(&self, c: &StatefulCircuit) -> Result<Option<TypedPlan>, RenderError> {
        checked_context_domain(c, &self.ledger, &self.witnesses, &self.pure, &self.circuits)
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

#[test]
fn minimal_projection_registers_the_type_from_kernel_annotation_alone() {
    let contract = subject(project(address()));
    assert!(contract.type_aliases.is_empty() && contract.circuits.is_empty());
    included(&contract, "root", "sole KernelSelf type annotation");
}

#[test]
fn direct_old_value_rules_compose_with_each_observation_operand() {
    let cases = vec![
        ("literal", bytes()),
        ("parameter", project(parameter("observed"))),
        ("project", project(address())),
        (
            "bytes-coerce",
            Expr::Coerce {
                value: Box::new(project(address())),
                ty: bytes_type(),
            },
        ),
        (
            "address-coerce",
            project(Expr::Coerce {
                value: Box::new(address()),
                ty: address_type(),
            }),
        ),
        (
            "shadow",
            local(
                "observed",
                address_type(),
                address(),
                project(parameter("observed")),
            ),
        ),
        (
            "left-observation",
            commit(project(address()), bytes(), bytes()),
        ),
        (
            "right-observation",
            commit(bytes(), project(address()), bytes()),
        ),
        (
            "opening-observation",
            commit(bytes(), bytes(), project(address())),
        ),
        (
            "all-observations",
            commit(project(address()), project(address()), project(address())),
        ),
        (
            "nested-commit",
            commit(
                commit(bytes(), project(address()), bytes()),
                bytes(),
                project(address()),
            ),
        ),
    ];
    assert_eq!(cases.len(), 11);
    for (label, value) in cases {
        let contract = subject(local("observed", address_type(), address(), value));
        included(&contract, "root", label);
    }
}

#[test]
fn old_pure_calls_keep_exact_interfaces_transitive_scope_and_caller_order() {
    let mut contract = subject(call("derive", vec![project(address()), address()]));
    contract.circuits = vec![
        PureCircuit {
            source: None,
            name: "bytes_identity".into(),
            internal: true,
            parameters: vec![Parameter {
                name: "value".into(),
                ty: bytes_type(),
            }],
            result: bytes_type(),
            body: parameter("value"),
        },
        PureCircuit {
            source: None,
            name: "derive".into(),
            internal: true,
            parameters: vec![
                Parameter {
                    name: "seed".into(),
                    ty: bytes_type(),
                },
                Parameter {
                    name: "location".into(),
                    ty: address_type(),
                },
            ],
            result: bytes_type(),
            body: local(
                "seed",
                bytes_type(),
                call("bytes_identity", vec![parameter("seed")]),
                commit(parameter("seed"), project(parameter("location")), bytes()),
            ),
        },
    ];
    included(&contract, "root", "nested pure calls");
    contract.circuits[1].body = bytes(); // Discard both already-evaluated actuals.
    included(&contract, "root", "discarded observed actuals");
    let plan = Context::new(&contract)
        .algebra(&contract.stateful_circuits[0])
        .unwrap();
    assert_eq!(tokens(&plan).matches("kernel_self").count(), 2);
}

#[test]
fn retained_token_export_and_renamed_symbols_are_explicit_old_controls() {
    let contract: Contract =
        serde_json::from_str(include_str!("../../../tests/token-query-schema20-ir.json")).unwrap();
    let exports: Vec<_> = contract
        .stateful_circuits
        .iter()
        .filter(|c| !c.internal)
        .map(|c| c.name.as_str())
        .collect();
    assert_eq!(exports, ["dao_voting_token"]);
    included(&contract, "dao_voting_token", "retained token query");
    let renamed = serde_json::to_string(&contract)
        .unwrap()
        .replace("tokenType", "derive_resource")
        .replace("dao_voting_token", "resource_id");
    included(
        &serde_json::from_str(&renamed).unwrap(),
        "resource_id",
        "renamed token query",
    );
}

#[test]
fn malformed_direct_query_commit_scope_and_projection_are_refused() {
    let mut wrong_address = address_type();
    let Type::Struct { fields, .. } = &mut wrong_address else {
        unreachable!()
    };
    fields[0].ty = Type::Bytes { length: 31 };
    let cases = vec![
        project(Expr::KernelSelf { ty: wrong_address }),
        Expr::StructField {
            value: Box::new(address()),
            field: "wrong".into(),
            index: 0,
        },
        Expr::StructField {
            value: Box::new(address()),
            field: "bytes".into(),
            index: 1,
        },
        commit(bytes(), bytes(), Expr::BytesLiteral { bytes: vec![0; 31] }),
        Expr::PersistentCommit {
            value: Box::new(Expr::Tuple {
                elements: vec![project(address())],
            }),
            opening: Box::new(bytes()),
        },
        commit(parameter("unknown"), bytes(), project(address())),
    ];
    for (i, value) in cases.into_iter().enumerate() {
        let contract = subject(value);
        let context = Context::new(&contract);
        let root = &contract.stateful_circuits[0];
        assert!(context.old(root).is_none(), "old negative {i}");
        assert!(
            matches!(
                context.algebra(root),
                Err(effect_plan::PlanError::Unsupported(_))
            ),
            "new negative {i}"
        );
    }
}

#[test]
fn pure_helper_cannot_observe_kernel_capture_caller_or_recurse() {
    for mode in 0..3 {
        let mut contract = subject(local(
            "caller",
            address_type(),
            address(),
            call("helper", vec![]),
        ));
        let body = match mode {
            0 => project(address()),
            1 => project(parameter("caller")),
            2 => call("helper", vec![]),
            _ => unreachable!(),
        };
        contract.circuits.push(PureCircuit {
            source: None,
            name: "helper".into(),
            internal: true,
            parameters: vec![],
            result: bytes_type(),
            body,
        });
        let context = Context::new(&contract);
        let root = &contract.stateful_circuits[0];
        assert!(context.old(root).is_none());
        assert!(matches!(
            context.algebra(root),
            Err(effect_plan::PlanError::Unsupported(_))
        ));
    }
}
