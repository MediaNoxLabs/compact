// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;
use crate::ir::{
    Contract, CounterAmount, Expr, LedgerFieldKind, Parameter, SCHEMA_VERSION, StateAction,
    WitnessDeclaration,
};
use crate::recorded::{LegacyActionPrecision, RecordingGapCode, admission_probe::trace};

fn subject() -> Contract {
    let helper = StatefulCircuit {
        source: None,
        internal: true,
        name: "helper".into(),
        parameters: vec![],
        result: Type::Unit,
        return_value: StateReturn::Unit,
        actions: vec![StateAction::Expression {
            value: Expr::Boolean { value: true },
        }],
    };
    let root = StatefulCircuit {
        source: None,
        internal: false,
        name: "root".into(),
        parameters: vec![
            Parameter {
                name: "tag".into(),
                ty: Type::OpaqueString,
            },
            Parameter {
                name: "number".into(),
                ty: Type::Field,
            },
        ],
        result: Type::Unit,
        return_value: StateReturn::Unit,
        actions: vec![
            StateAction::CircuitCall {
                name: "helper".into(),
                arguments: vec![],
            },
            public_action(),
        ],
    };
    Contract {
        schema_version: SCHEMA_VERSION,
        type_aliases: vec![],
        constructor: None,
        witnesses: vec![],
        circuits: vec![],
        ledger_fields: vec![LedgerField {
            source: None,
            id: "count".into(),
            index: 0,
            path: vec![],
            declaration: LedgerFieldKind::Counter,
        }],
        stateful_circuits: vec![root, helper],
    }
}
fn public_action() -> StateAction {
    StateAction::CounterIncrement {
        field: "count".into(),
        index: 0,
        amount: CounterAmount::Literal { value: 1 },
    }
}
fn attempt_contract(
    c: &Contract,
) -> Result<ProfileAttempt<TypedPlan, effect_plan::LoweringRejection>, RenderError> {
    attempt(
        &c.stateful_circuits[0],
        &c.ledger_fields.iter().map(|v| (v.id.as_str(), v)).collect(),
        &c.witnesses.iter().map(|v| (v.name.as_str(), v)).collect(),
        &c.circuits.iter().map(|v| (v.name.as_str(), v)).collect(),
        &c.stateful_circuits
            .iter()
            .map(|v| (v.name.as_str(), v))
            .collect(),
    )
}
fn assert_supported(c: &Contract) {
    assert!(matches!(
        attempt_contract(c).unwrap(),
        ProfileAttempt::Admitted(_)
    ));
}
fn rejected(c: &Contract) -> effect_plan::LoweringRejection {
    match attempt_contract(c).unwrap() {
        ProfileAttempt::Rejected(r) => r,
        _ => panic!("expected typed refusal"),
    }
}

fn checked(c: &Contract) -> effect_plan::CheckedPlan {
    effect_plan::check_with_calls_diagnostic(
        &c.stateful_circuits[0],
        &c.ledger_fields.iter().map(|v| (v.id.as_str(), v)).collect(),
        &c.witnesses.iter().map(|v| (v.name.as_str(), v)).collect(),
        &c.circuits.iter().map(|v| (v.name.as_str(), v)).collect(),
        &c.stateful_circuits
            .iter()
            .map(|v| (v.name.as_str(), v))
            .collect(),
    )
    .unwrap()
}

#[test]
fn complete_old_unit_root_moves_once_at_the_same_priority() {
    let c = subject();
    let old: ProfileAttempt<TypedPlan, typed_plan::CompositionRejection> =
        typed_plan::lower_unit_composition_checked(
            &c.stateful_circuits[0],
            &c.ledger_fields.iter().map(|v| (v.id.as_str(), v)).collect(),
            &HashMap::new(),
            &HashMap::new(),
            &c.stateful_circuits
                .iter()
                .map(|v| (v.name.as_str(), v))
                .collect(),
        );
    assert!(
        matches!(old, ProfileAttempt::Admitted(_)),
        "whole old premise first"
    );
    let (result, rows) = trace::capture_json(|| crate::render_with_capabilities(&c));
    let rendered = result.unwrap();
    assert!(
        rendered.capabilities.circuits[0].recorded
            && rendered.capabilities.circuits[0].observed_call
    );
    let public: Vec<_> = rows
        .as_array()
        .unwrap()
        .iter()
        .filter(|v| {
            v["circuit"] == "root" && v["phase"] == "public_emission" && v["helper"] == false
        })
        .collect();
    assert_eq!(public.len(), 1);
    assert_eq!(public[0]["body_owner"], "algebra.unit-composition");
    let routes: Vec<_> = public[0]["attempts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["route"].as_str().unwrap())
        .collect();
    assert_eq!(
        routes
            .iter()
            .filter(|&&r| r == "algebra.unit-composition")
            .count(),
        1
    );
    assert!(!routes.contains(&"selector.unit-composition"));
    assert!(!routes.contains(&"algebra.zswap"));
    assert!(
        routes
            .iter()
            .position(|&r| r == "algebra.unit-actions")
            .unwrap()
            < routes
                .iter()
                .position(|&r| r == "algebra.unit-composition")
                .unwrap()
    );
}

#[test]
fn legacy_signature_is_a_subset_of_the_checked_public_interface() {
    for (ty, accepted) in [
        (Type::OpaqueString, true),
        (Type::OpaqueBytes, false),
        (Type::Unit, false),
        (
            Type::Unsigned {
                max: u64::MAX.to_string(),
            },
            true,
        ),
        (
            Type::Unsigned {
                max: (u128::from(u64::MAX) + 1).to_string(),
            },
            false,
        ),
        (
            Type::Tuple {
                elements: vec![Type::Field; 8],
            },
            true,
        ),
        (
            Type::Tuple {
                elements: vec![Type::Field; 9],
            },
            false,
        ),
        (Type::Tuple { elements: vec![] }, false),
        (
            Type::Vector {
                element: Box::new(Type::Field),
                length: 0,
            },
            true,
        ),
        (
            Type::Vector {
                element: Box::new(Type::OpaqueBytes),
                length: 0,
            },
            false,
        ),
    ] {
        // Preserve the immutable old rule as historical evidence, while the
        // production adapter delegates eligibility to checked declarations.
        assert_eq!(typed_plan::unit_composition_parameter_type(&ty), accepted);
        let mut c = subject();
        c.stateful_circuits[0].parameters[0].ty = ty;
        let result = attempt_contract(&c).unwrap();
        assert!(matches!(result, ProfileAttempt::Admitted(_)));
    }
    let mut c = subject();
    c.stateful_circuits[0].return_value = StateReturn::Expression { value: Expr::Unit };
    assert!(matches!(
        attempt_contract(&c).unwrap(),
        ProfileAttempt::NotApplicable
    ));
    c.stateful_circuits[0].result = Type::Boolean;
    assert!(matches!(
        attempt_contract(&c).unwrap(),
        ProfileAttempt::NotApplicable
    ));
}

#[test]
fn public_evidence_composes_calls_and_lazy_branches_without_helper_shape_rules() {
    let mut c = subject();
    c.stateful_circuits[0].actions = vec![public_action()];
    assert_supported(&c); // No helper count gate.
    c.stateful_circuits[0].actions = vec![StateAction::CircuitCall {
        name: "helper".into(),
        arguments: vec![],
    }];
    c.stateful_circuits[1].actions = vec![StateAction::If {
        condition: Expr::Boolean { value: false },
        then: Box::new(public_action()),
        otherwise: Box::new(StateAction::Sequence { actions: vec![] }),
    }];
    assert_supported(&c); // Static evidence does not require every runtime arm.
    c.stateful_circuits[1].actions = vec![StateAction::Expression {
        value: Expr::CounterRead {
            field: "count".into(),
            index: 0,
        },
    }];
    assert_supported(&c); // A read supplies evidence without a mutation.
}

#[test]
fn pure_witness_key_and_private_intent_effects_do_not_supply_public_evidence() {
    let mut c = subject();
    c.circuits.push(PureCircuit {
        source: None,
        internal: true,
        name: "pure".into(),
        parameters: vec![],
        result: Type::Unit,
        body: Expr::Unit,
    });
    c.witnesses.push(WitnessDeclaration {
        source: None,
        name: "private".into(),
        parameters: vec![],
        result: Type::Unit,
    });
    let private = vec![
        StateAction::PureCall {
            name: "pure".into(),
            arguments: vec![],
        },
        StateAction::Expression {
            value: Expr::Default { ty: Type::Boolean },
        },
        StateAction::Assert {
            condition: Expr::Boolean { value: true },
            message: "pure".into(),
        },
        StateAction::Expression {
            value: Expr::WitnessCall {
                name: "private".into(),
                arguments: vec![],
            },
        },
        StateAction::Expression {
            value: Expr::NativeWitnessCall {
                builtin: crate::ir::NativeWitnessBuiltin::OwnPublicKey,
            },
        },
    ];
    for action in private {
        c.stateful_circuits[0].actions = vec![action];
        assert!(
            !effect_plan::has_public_program(&checked(&c)),
            "valid private plan has no VM operations"
        );
        assert_supported(&c);
        let ProfileAttempt::Admitted(plan) = attempt_contract(&c).unwrap() else {
            unreachable!("qualified execution remains recordable")
        };
        let steps = &plan.steps;
        let text = quote::quote!(#(#steps)*).to_string();
        let required = match &c.stateful_circuits[0].actions[0] {
            StateAction::PureCall { .. } => "pure_circuits",
            StateAction::Assert { .. } => "AssertionFailed",
            StateAction::Expression {
                value: Expr::Default { .. },
            } => "Default",
            StateAction::Expression {
                value: Expr::WitnessCall { .. },
            } => "try_witness_metered",
            StateAction::Expression {
                value: Expr::NativeWitnessCall { .. },
            } => "own_coin_public_key",
            _ => unreachable!(),
        };
        assert!(text.contains(required), "recording preserves {required}");
    }
    c.stateful_circuits[0].actions = vec![StateAction::Expression {
        value: Expr::CreateZswapInput {
            coin: Box::new(Expr::Default {
                ty: crate::coin_shapes::qualified_coin_type(),
            }),
        },
    }];
    assert!(!effect_plan::has_public_program(&checked(&c)));
    let ProfileAttempt::Admitted(plan) = attempt_contract(&c).unwrap() else {
        panic!("checked private intent remains recordable")
    };
    let steps = &plan.steps;
    let text = quote::quote!(#(#steps)*).to_string();
    assert!(text.contains("create_zswap_input"));
    c.stateful_circuits[0].actions = vec![StateAction::Expression {
        value: Expr::CreateZswapOutput {
            coin: Box::new(Expr::Default {
                ty: crate::coin_shapes::shielded_coin_type(),
            }),
            recipient: Box::new(Expr::Default {
                ty: crate::coin_shapes::shielded_recipient_type(),
            }),
        },
    }];
    assert!(!effect_plan::has_public_program(&checked(&c)));
    let ProfileAttempt::Admitted(plan) = attempt_contract(&c).unwrap() else {
        panic!("checked private intent remains recordable")
    };
    let steps = &plan.steps;
    let text = quote::quote!(#(#steps)*).to_string();
    assert!(text.contains("create_zswap_output"));
}

#[test]
fn unsupported_refines_same_action_and_invalid_remains_fatal() {
    let mut c = subject();
    c.stateful_circuits[0].actions[1] = StateAction::Expression {
        value: Expr::PersistentCommit {
            value: Box::new(Expr::FieldLiteral { value: "1".into() }),
            opening: Box::new(Expr::BytesLiteral { bytes: vec![0; 32] }),
        },
    };
    let refusal = rejected(&c);
    let outer = RecordingGap {
        resource: None,
        code: RecordingGapCode::UnsupportedAction,
        ir_node: "StateAction::Expression".into(),
        path: "unrelated text".into(),
        detail: "outer".into(),
    };
    let refined = refusal.refine_action(1, LegacyActionPrecision::Coarse, outer.clone());
    assert_eq!(refined.path, "actions[1].value");
    assert!(refined.detail.contains("persistent commitment"));
    assert_eq!(
        refusal.refine_action(0, LegacyActionPrecision::Coarse, outer.clone()),
        outer
    );
    assert_eq!(
        refusal.refine_action(1, LegacyActionPrecision::Concrete, outer.clone()),
        outer
    );
    assert!(matches!(
        failure(effect_plan::DiagnosticPlanError::Invalid(refined)),
        Err(RenderError::RecordingPlanInvariant { .. })
    ));
}

#[test]
fn argument_observations_and_earlier_owner_priority_are_preserved() {
    let mut c = subject();
    c.stateful_circuits[1].parameters = vec![Parameter {
        name: "n".into(),
        ty: Type::Unsigned {
            max: u64::MAX.to_string(),
        },
    }];
    c.stateful_circuits[0].actions = vec![StateAction::CircuitCall {
        name: "helper".into(),
        arguments: vec![Expr::CounterRead {
            field: "count".into(),
            index: 0,
        }],
    }];
    assert!(effect_plan::has_public_program(&checked(&c)));
    assert_supported(&c);
    c.stateful_circuits[0].parameters.clear();
    let (rendered, rows) = trace::capture_json(|| crate::render_with_capabilities(&c));
    assert!(rendered.unwrap().capabilities.circuits[0].recorded);
    let root = rows
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["circuit"] == "root" && r["phase"] == "public_emission" && r["helper"] == false)
        .unwrap();
    assert_eq!(root["body_owner"], "algebra.membership");
    assert!(
        !root["attempts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|a| a["route"] == "algebra.unit-composition")
    );
}

#[path = "tests/interfaces.rs"]
mod interfaces;

#[path = "tests/empty_execution.rs"]
mod empty_execution;
