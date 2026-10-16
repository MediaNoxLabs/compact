// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Statement calls reuse the checked call boundary even when results are discarded.
use super::*;
fn parameter(name: &str) -> Expr {
    Expr::Parameter { name: name.into() }
}
fn field(n: u64) -> Expr {
    Expr::FieldLiteral {
        value: n.to_string(),
    }
}
fn statement(arguments: Vec<Expr>) -> StateAction {
    StateAction::CircuitCall {
        name: "helper".into(),
        arguments,
    }
}
fn sources() -> Vec<StatefulCircuit> {
    vec![
        StatefulCircuit {
            source: None,
            name: "root".into(),
            internal: false,
            parameters: vec![],
            actions: vec![statement(vec![field(1), field(2)])],
            result: Type::Field,
            return_value: StateReturn::Expression { value: field(3) },
        },
        StatefulCircuit {
            source: None,
            name: "helper".into(),
            internal: true,
            parameters: vec![
                Parameter {
                    name: "a".into(),
                    ty: Type::Field,
                },
                Parameter {
                    name: "b".into(),
                    ty: Type::Field,
                },
            ],
            actions: vec![StateAction::CellWrite {
                field: "stored".into(),
                index: 0,
                value: parameter("a"),
            }],
            result: Type::Field,
            return_value: StateReturn::Expression {
                value: parameter("b"),
            },
        },
    ]
}
fn checked(circuits: &[StatefulCircuit], pure: &[PureCircuit]) -> Result<CheckedPlan, PlanError> {
    let field = LedgerField {
        source: None,
        id: "stored".into(),
        index: 0,
        path: vec![0],
        declaration: LedgerFieldKind::Cell { ty: Type::Field },
    };
    let witness = WitnessDeclaration {
        source: None,
        name: "next".into(),
        parameters: vec![],
        result: Type::Field,
    };
    check_with_calls(
        &circuits[0],
        &HashMap::from([("stored", &field)]),
        &HashMap::from([("next", &witness)]),
        &pure.iter().map(|c| (c.name.as_str(), c)).collect(),
        &circuits.iter().map(|c| (c.name.as_str(), c)).collect(),
    )
}
#[test]
fn discarded_nonunit_call_preserves_ltr_arguments_fresh_scope_and_body_effects() {
    let mut circuits = sources();
    let witness = || Expr::WitnessCall {
        name: "next".into(),
        arguments: vec![],
    };
    circuits[0].actions = vec![statement(vec![witness(), witness()])];
    let checked = checked(&circuits, &[]).unwrap();
    let bindings = &checked.plan().body.bindings;
    assert_eq!(bindings.len(), 4);
    assert!(matches!(bindings[0].operation, Operation::Witness { .. }));
    assert!(matches!(bindings[1].operation, Operation::Witness { .. }));
    assert_eq!(bindings[0].path, "actions[0].arguments[0]");
    assert_eq!(bindings[1].path, "actions[0].arguments[1]");
    let Operation::Call {
        arguments,
        parameters,
        body,
        ..
    } = &bindings[2].operation
    else {
        panic!("statement retains checked call")
    };
    assert_eq!(
        arguments,
        &vec![bindings[0].value.clone(), bindings[1].value.clone()]
    );
    assert!(
        parameters
            .iter()
            .all(|p| arguments.iter().all(|a| a.id != p.id))
    );
    assert!(
        body.bindings
            .iter()
            .any(|b| matches!(b.operation, Operation::CellWrite { .. }))
    );
    assert_eq!(body.result, parameters[1]);
    assert_eq!(bindings[2].value.ty, Type::Field);
    assert_ne!(checked.plan().body.result.id, bindings[2].value.id);
    let rendered = render(&checked).unwrap();
    let steps = rendered.steps;
    let source = quote::quote!(#(#steps)*).to_string();
    assert_eq!(source.matches("try_witness_metered").count(), 2);
    assert_eq!(source.matches("record_write").count(), 1);
}
#[test]
fn statement_calls_reject_unknown_pure_arity_type_recursion_and_scope_capture() {
    for mode in 0..7 {
        let mut circuits = sources();
        let mut pure = vec![];
        match mode {
            0 => {
                circuits[0].actions = vec![StateAction::CircuitCall {
                    name: "unknown".into(),
                    arguments: vec![],
                }]
            }
            1 => {
                let helper = circuits.pop().unwrap();
                pure.push(PureCircuit {
                    source: None,
                    name: helper.name,
                    internal: true,
                    parameters: helper.parameters,
                    result: Type::Field,
                    body: field(0),
                });
            }
            2 => circuits[0].actions = vec![statement(vec![field(1)])],
            3 => {
                circuits[0].actions = vec![statement(vec![
                    Expr::BytesLiteral { bytes: vec![0; 32] },
                    field(2),
                ])]
            }
            4 => circuits[1]
                .actions
                .push(statement(vec![parameter("a"), parameter("b")])),
            5 => {
                circuits[0].parameters.push(Parameter {
                    name: "caller_only".into(),
                    ty: Type::Field,
                });
                circuits[1].return_value = StateReturn::Expression {
                    value: parameter("caller_only"),
                };
            }
            6 => {
                circuits[0].return_value = StateReturn::Expression {
                    value: parameter("a"),
                }
            }
            _ => unreachable!(),
        }
        assert!(
            matches!(checked(&circuits, &pure), Err(PlanError::Unsupported(_))),
            "statement negative {mode}"
        );
    }
}
#[test]
fn statement_arguments_keep_first_error_before_later_argument_or_callee_effect() {
    let mut circuits = sources();
    circuits[0].actions = vec![statement(vec![
        parameter("first_missing"),
        parameter("second_missing"),
    ])];
    let PlanError::Unsupported(gap) = checked(&circuits, &[]).unwrap_err() else {
        panic!("source error")
    };
    assert_eq!(gap.path, "actions[0].arguments[0]");
}
#[test]
fn statement_arguments_match_native_exact_types_and_accept_explicit_coercion() {
    let narrow = Type::Unsigned { max: "255".into() };
    let wide = Type::Unsigned {
        max: "65535".into(),
    };
    let literal = Expr::UnsignedLiteral {
        value: "7".into(),
        max: "255".into(),
    };
    let mut circuits = sources();
    circuits[1].parameters = vec![Parameter {
        name: "a".into(),
        ty: wide.clone(),
    }];
    circuits[1].actions.clear();
    circuits[1].return_value = StateReturn::Expression { value: field(0) };
    let contract = |circuits: Vec<StatefulCircuit>| Contract {
        schema_version: crate::ir::SCHEMA_VERSION,
        type_aliases: vec![],
        ledger_fields: vec![],
        constructor: None,
        witnesses: vec![],
        circuits: vec![],
        stateful_circuits: circuits,
    };
    circuits[0].actions = vec![statement(vec![literal.clone()])];
    let PlanError::Unsupported(gap) = checked(&circuits, &[]).unwrap_err() else {
        panic!("source type refusal")
    };
    assert_eq!(gap.path, "actions[0].arguments[0]");
    assert!(gap.detail.contains("statement call argument type"));
    assert!(
        matches!(crate::render_with_capabilities(&contract(circuits.clone())),Err(crate::RenderError::TypeMismatch{expected,actual}) if expected==wide && actual==narrow),
        "native statement must not implicitly widen"
    );
    circuits[0].actions = vec![statement(vec![Expr::Coerce {
        value: Box::new(literal),
        ty: wide,
    }])];
    let checked = checked(&circuits, &[]).unwrap();
    render(&checked).unwrap();
    crate::render_with_capabilities(&contract(circuits)).unwrap();
}
