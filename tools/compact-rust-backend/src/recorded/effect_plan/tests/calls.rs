// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Bounded stateful call composition; production selector retirement is separate.
use super::*;

fn circuit(name: &str, parameters: &[&str], value: Expr, result: Type) -> StatefulCircuit {
    StatefulCircuit {
        source: None,
        name: name.into(),
        internal: name != "root",
        parameters: parameters
            .iter()
            .map(|name| Parameter {
                name: (*name).into(),
                ty: Type::Field,
            })
            .collect(),
        actions: vec![],
        result,
        return_value: StateReturn::Expression { value },
    }
}
fn parameter(name: &str) -> Expr {
    Expr::Parameter { name: name.into() }
}
fn field(value: u64) -> Expr {
    Expr::FieldLiteral {
        value: value.to_string(),
    }
}
fn call(name: &str, arguments: Vec<Expr>) -> Expr {
    Expr::Call {
        name: name.into(),
        arguments,
    }
}
fn checked(circuits: &[StatefulCircuit]) -> Result<CheckedPlan, PlanError> {
    let ledger = LedgerField {
        source: None,
        id: "stored".into(),
        index: 0,
        path: vec![0],
        declaration: LedgerFieldKind::Cell { ty: Type::Field },
    };
    let fields = HashMap::from([("stored", &ledger)]);
    let calls = circuits.iter().map(|c| (c.name.as_str(), c)).collect();
    let declarations =
        declarations::Declarations::with_calls(&fields, &HashMap::new(), &HashMap::new(), &calls)
            .with_root(&circuits[0]);
    let plan = lower::lower_with_calls(&circuits[0], &declarations, &calls)
        .map_err(PlanError::Unsupported)?;
    validate::check(plan, declarations).map_err(PlanError::Invalid)
}
fn refused(circuits: &[StatefulCircuit]) -> RecordingGap {
    match checked(circuits).unwrap_err() {
        PlanError::Unsupported(gap) => gap,
        e => panic!("unexpected invariant failure: {e:?}"),
    }
}

#[test]
fn arguments_are_left_to_right_once_and_formals_are_fresh() {
    let read = || Expr::CellRead {
        field: "stored".into(),
        index: 0,
    };
    let root = circuit(
        "root",
        &[],
        call("helper", vec![read(), read()]),
        Type::Field,
    );
    let helper = circuit(
        "helper",
        &["a", "unused"],
        Expr::Add {
            left: Box::new(parameter("a")),
            right: Box::new(parameter("a")),
        },
        Type::Field,
    );
    let checked = checked(&[root, helper]).unwrap();
    let bindings = &checked.plan().body.bindings;
    assert_eq!(bindings.len(), 3);
    assert!(matches!(bindings[0].operation, Operation::CellRead(_)));
    assert!(matches!(bindings[1].operation, Operation::CellRead(_)));
    assert!(bindings[0].path.ends_with("arguments[0]"));
    assert!(bindings[1].path.ends_with("arguments[1]"));
    let Operation::Call {
        arguments,
        parameters,
        body,
        ..
    } = &bindings[2].operation
    else {
        panic!("expected explicit call")
    };
    assert_eq!(
        arguments,
        &vec![bindings[0].value.clone(), bindings[1].value.clone()]
    );
    assert_ne!(parameters[0].id, arguments[0].id);
    assert_ne!(parameters[1].id, arguments[1].id);
    let Operation::Add(a, b) = &body.bindings[0].operation else {
        panic!("expected helper add")
    };
    assert_eq!((a, b), (&parameters[0], &parameters[0]));
    render(&checked).unwrap();
}

#[test]
fn nested_argument_call_is_sequential_not_recursive() {
    let root = circuit(
        "root",
        &[],
        call("helper", vec![call("helper", vec![field(9)])]),
        Type::Field,
    );
    let helper = circuit("helper", &["x"], parameter("x"), Type::Field);
    let checked = checked(&[root, helper]).unwrap();
    let bindings = &checked.plan().body.bindings;
    assert_eq!(
        bindings
            .iter()
            .filter(|b| matches!(b.operation, Operation::Call { .. }))
            .count(),
        2
    );
    let Operation::Call { arguments, .. } = &bindings[2].operation else {
        panic!()
    };
    assert_eq!(arguments, &vec![bindings[1].value.clone()]);
}

#[test]
fn helper_terminal_scope_preserves_return_without_shadowing_caller() {
    let root = circuit(
        "root",
        &["x"],
        Expr::Add {
            left: Box::new(call("helper", vec![parameter("x")])),
            right: Box::new(parameter("x")),
        },
        Type::Field,
    );
    let mut helper = circuit("helper", &["x"], parameter("x"), Type::Field);
    helper.actions = vec![StateAction::Let {
        bindings: vec![LocalBinding {
            name: "x".into(),
            ty: Type::Field,
            value: field(17),
        }],
        action: Box::new(StateAction::Sequence { actions: vec![] }),
    }];
    let checked = checked(&[root, helper]).unwrap();
    let plan = checked.plan();
    let Operation::Call { body, .. } = &plan.body.bindings[0].operation else {
        panic!()
    };
    assert_eq!(body.result, body.bindings[0].value);
    assert!(
        body.bindings[0]
            .path
            .contains(".callee[helper].actions[0].bindings[0].value")
    );
    let Operation::Add(_, right) = &plan.body.bindings[1].operation else {
        panic!()
    };
    assert_eq!(right, &plan.parameters[0]);
}

#[test]
fn callee_cannot_capture_caller_names() {
    let root = circuit(
        "root",
        &["caller_secret"],
        call("helper", vec![]),
        Type::Field,
    );
    let helper = circuit("helper", &[], parameter("caller_secret"), Type::Field);
    let gap = refused(&[root, helper]);
    assert!(gap.detail.contains("unbound value"));
    assert_eq!(
        gap.path,
        "return_value.value.callee[helper].return_value.value"
    );
}

#[test]
fn helper_effectful_body_owns_its_actions_and_result_coercion() {
    let root = circuit("root", &[], call("helper", vec![]), Type::Field);
    let mut helper = circuit("helper", &[], Expr::Unit, Type::Field);
    helper.return_value = StateReturn::Effectful {
        body: ReturnPlan::Value { value: field(3) },
    };
    checked(&[root.clone(), helper.clone()]).unwrap();
    helper
        .actions
        .push(StateAction::Expression { value: Expr::Unit });
    assert_eq!(
        refused(&[root, helper]).path,
        "return_value.value.callee[helper].actions"
    );
}

#[test]
fn wrong_arity_types_and_duplicate_parameters_refuse() {
    let helper = circuit("helper", &["x"], parameter("x"), Type::Field);
    let root = |args| circuit("root", &[], call("helper", args), Type::Field);
    assert!(
        refused(&[root(vec![]), helper.clone()])
            .detail
            .contains("argument count")
    );
    assert!(
        refused(&[root(vec![Expr::Boolean { value: true }]), helper])
            .path
            .ends_with("arguments[0]")
    );
    let helper = circuit("helper", &["x", "x"], parameter("x"), Type::Field);
    assert!(
        refused(&[root(vec![field(1), field(2)]), helper])
            .detail
            .contains("duplicate callee parameter")
    );
}

#[test]
fn recursive_calls_are_rejected_with_the_nested_call_path() {
    let root = circuit("root", &[], call("a", vec![]), Type::Unit);
    let a = circuit("a", &[], call("b", vec![]), Type::Unit);
    let b = circuit("b", &[], call("a", vec![]), Type::Unit);
    let gap = refused(&[root, a, b]);
    assert!(gap.detail.contains("recursive"));
    assert!(
        gap.path
            .contains(".callee[a].return_value.value.callee[b].return_value.value")
    );
}

fn chain(depth: usize) -> Vec<StatefulCircuit> {
    (0..depth)
        .map(|i| {
            circuit(
                &format!("c{i}"),
                &[],
                if i + 1 == depth {
                    Expr::Unit
                } else {
                    call(&format!("c{}", i + 1), vec![])
                },
                Type::Unit,
            )
        })
        .collect()
}
#[test]
fn call_depth_matches_the_existing_policy_boundary() {
    let (depth, _) = crate::resource_limits::recording_call_limits();
    checked(&chain(depth)).unwrap();
    assert!(refused(&chain(depth + 1)).detail.contains("call depth"));
}

#[test]
fn repeated_dag_expansion_spends_global_fuel_per_invocation() {
    let mut circuits = chain(11);
    for (i, circuit) in circuits.iter_mut().enumerate().take(10) {
        circuit.return_value = StateReturn::Expression {
            value: Expr::Sequence {
                steps: vec![
                    call(&format!("c{}", i + 1), vec![]),
                    call(&format!("c{}", i + 1), vec![]),
                ],
                value: Box::new(call(&format!("c{}", i + 1), vec![])),
            },
        };
    }
    let gap = refused(&circuits);
    assert!(gap.detail.contains("expansion work"), "{gap:?}");
}

#[test]
fn callee_local_names_do_not_escape_into_the_caller_continuation() {
    let root = circuit(
        "root",
        &[],
        Expr::Sequence {
            steps: vec![call("helper", vec![])],
            value: Box::new(parameter("hidden")),
        },
        Type::Field,
    );
    let mut helper = circuit("helper", &[], Expr::Unit, Type::Unit);
    helper.actions = vec![StateAction::Let {
        bindings: vec![LocalBinding {
            name: "hidden".into(),
            ty: Type::Field,
            value: field(17),
        }],
        action: Box::new(StateAction::Sequence { actions: vec![] }),
    }];
    let gap = refused(&[root, helper]);
    assert_eq!(gap.path, "return_value.value.value");
    assert!(gap.detail.contains("unbound value"));
}

#[test]
fn argument_and_return_coercions_use_declared_types() {
    let small = || Expr::UnsignedLiteral {
        value: "9".into(),
        max: "255".into(),
    };
    let root = circuit("root", &[], call("helper", vec![small()]), Type::Field);
    let helper = circuit("helper", &["x"], small(), Type::Field);
    let checked = checked(&[root, helper]).unwrap();
    let bindings = &checked.plan().body.bindings;
    assert!(matches!(
        bindings[1].operation,
        Operation::Convert {
            kind: Conversion::Coerce,
            ..
        }
    ));
    let Operation::Call {
        arguments,
        parameters,
        body,
        ..
    } = &bindings[2].operation
    else {
        panic!()
    };
    assert_eq!(arguments[0].ty, Type::Field);
    assert_eq!(parameters[0].ty, Type::Field);
    assert_eq!(body.result.ty, Type::Field);
    assert!(matches!(
        body.bindings.last().unwrap().operation,
        Operation::Convert {
            kind: Conversion::Coerce,
            ..
        }
    ));
}
