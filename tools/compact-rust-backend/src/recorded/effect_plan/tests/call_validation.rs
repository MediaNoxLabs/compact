// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Forged-plan checks independent of call lowering. Every mutation starts from
//! a structurally valid scoped call; these checks do not prove source equivalence.
use super::*;

fn parameter(name: &str) -> Parameter {
    Parameter {
        name: name.into(),
        ty: Type::Field,
    }
}
fn callee(name: &str) -> StatefulCircuit {
    StatefulCircuit {
        source: None,
        name: name.into(),
        internal: true,
        parameters: vec![parameter("left"), parameter("right")],
        result: Type::Field,
        actions: vec![],
        return_value: StateReturn::Expression {
            value: Expr::Add {
                left: Box::new(Expr::Parameter {
                    name: "left".into(),
                }),
                right: Box::new(Expr::Parameter {
                    name: "right".into(),
                }),
            },
        },
    }
}
fn environment(circuits: &[StatefulCircuit], pure: &[PureCircuit]) -> declarations::Declarations {
    declarations::Declarations::with_calls(
        &Default::default(),
        &Default::default(),
        &pure.iter().map(|c| (c.name.as_str(), c)).collect(),
        &circuits.iter().map(|c| (c.name.as_str(), c)).collect(),
    )
}
fn standard_environment() -> declarations::Declarations {
    environment(&[callee("leaf"), callee("wrapper")], &[])
}
fn call_binding(declaration: usize, argument_ids: [usize; 2], first_id: usize) -> Binding {
    let parameters = vec![field(first_id), field(first_id + 1)];
    let sum = field(first_id + 2);
    Binding {
        value: field(first_id + 3),
        operation: Operation::Call {
            declaration: CallId(declaration),
            arguments: argument_ids.into_iter().map(field).collect(),
            parameters: parameters.clone(),
            body: Block {
                bindings: vec![Binding {
                    value: sum.clone(),
                    operation: Operation::Add(parameters[0].clone(), parameters[1].clone()),
                    path: "callee.sum".into(),
                }],
                result: sum,
            },
        },
        path: "caller.call".into(),
    }
}
fn valid_plan() -> Plan {
    Plan {
        parameters: vec![field(0), field(1)],
        body: Block {
            bindings: vec![call_binding(0, [0, 1], 2)],
            result: field(5),
        },
        result_type: Type::Field,
    }
}
fn mutate_call(
    plan: &mut Plan,
    change: impl FnOnce(&mut CallId, &mut Vec<Value>, &mut Vec<Value>, &mut Block),
) {
    let Operation::Call {
        declaration,
        arguments,
        parameters,
        body,
    } = &mut plan.body.bindings[0].operation
    else {
        unreachable!("fixture must contain a call")
    };
    change(declaration, arguments, parameters, body);
}
fn rejects(change: impl FnOnce(&mut Plan)) {
    let mut plan = valid_plan();
    validate::check(plan.clone(), standard_environment()).expect("unmodified scoped call");
    change(&mut plan);
    assert!(validate::check(plan, standard_environment()).is_err());
}

#[test]
fn call_identity_must_resolve_to_one_stateful_declaration() {
    rejects(|p| mutate_call(p, |id, _, _, _| *id = CallId(99)));
    let stateful = callee("leaf");
    let pure = PureCircuit {
        source: None,
        name: "leaf".into(),
        internal: true,
        parameters: stateful.parameters.clone(),
        result: Type::Field,
        body: Expr::Parameter {
            name: "left".into(),
        },
    };
    validate::check(
        valid_plan(),
        environment(std::slice::from_ref(&stateful), &[]),
    )
    .unwrap();
    assert!(validate::check(valid_plan(), environment(&[], std::slice::from_ref(&pure))).is_err());
    let ambiguous = environment(&[stateful], &[pure]);
    assert!(ambiguous.resolve_call("leaf").is_none());
    assert!(validate::check(valid_plan(), ambiguous).is_err());
}

#[test]
fn call_signature_checks_actuals_formals_result_and_duplicate_names() {
    rejects(|p| {
        mutate_call(p, |_, args, _, _| {
            args.pop();
        })
    });
    rejects(|p| {
        mutate_call(p, |_, _, formals, _| {
            formals.pop();
        })
    });
    rejects(|p| {
        p.parameters[0].ty = Type::Boolean;
        mutate_call(p, |_, args, _, _| args[0].ty = Type::Boolean);
    });
    rejects(|p| mutate_call(p, |_, _, formals, _| formals[0].ty = Type::Boolean));
    rejects(|p| {
        p.body.bindings[0].value.ty = Type::Boolean;
        p.body.result.ty = Type::Boolean;
        p.result_type = Type::Boolean;
    });
    rejects(|p| {
        mutate_call(p, |_, _, _, body| {
            body.bindings = vec![Binding {
                value: Value {
                    id: 4,
                    ty: Type::Boolean,
                },
                operation: Operation::Literal(Literal::Boolean(true)),
                path: "callee.boolean".into(),
            }];
            body.result = Value {
                id: 4,
                ty: Type::Boolean,
            };
        })
    });
    let mut source = callee("leaf");
    source.parameters[1].name = source.parameters[0].name.clone();
    assert!(validate::check(valid_plan(), environment(&[source], &[])).is_err());
}

#[test]
fn isolated_call_scope_rejects_capture_leaks_and_id_reuse() {
    // Body values cannot capture the caller's argument IDs directly.
    rejects(|p| mutate_call(p, |_, _, _, body| body.result = field(0)));
    rejects(|p| {
        mutate_call(p, |_, _, _, body| {
            body.bindings[0].operation = Operation::Add(field(0), field(3));
        })
    });
    // Formals are fresh IDs, not aliases of caller or other formals.
    rejects(|p| mutate_call(p, |_, _, formals, _| formals[0] = field(0)));
    rejects(|p| mutate_call(p, |_, _, formals, _| formals[1] = formals[0].clone()));
    // The only callee value entering the caller is its call result binding.
    rejects(|p| p.body.result = field(4));
    rejects(|p| {
        p.body.bindings.push(call_binding(0, [2, 1], 6));
        p.body.result = field(9);
    });
    rejects(|p| {
        p.body.bindings.push(call_binding(0, [5, 1], 2));
        p.body.result = field(5);
    });
}

fn nested_plan() -> Plan {
    let mut outer = call_binding(1, [0, 1], 2);
    if let Operation::Call { body, .. } = &mut outer.operation {
        body.bindings = vec![call_binding(0, [2, 3], 4)];
        body.result = field(7);
    }
    outer.value = field(8);
    Plan {
        parameters: vec![field(0), field(1)],
        body: Block {
            bindings: vec![outer],
            result: field(8),
        },
        result_type: Type::Field,
    }
}

#[test]
fn repeated_siblings_and_acyclic_nested_calls_are_valid() {
    let mut repeated = valid_plan();
    repeated.body.bindings.push(call_binding(0, [5, 1], 6));
    repeated.body.result = field(9);
    validate::check(repeated, standard_environment()).unwrap();
    validate::check(nested_plan(), standard_environment()).unwrap();
}

#[test]
fn active_call_identity_and_bound_root_reject_recursion() {
    let mut nested = nested_plan();
    validate::check(nested.clone(), standard_environment()).unwrap();
    mutate_call(&mut nested, |outer_id, _, _, body| {
        if let Operation::Call { declaration, .. } = &mut body.bindings[0].operation {
            *declaration = *outer_id;
        }
    });
    assert!(validate::check(nested, standard_environment()).is_err());
    let root = callee("leaf");
    let declarations = environment(std::slice::from_ref(&root), &[]).with_root(&root);
    assert!(validate::check(valid_plan(), declarations).is_err());
}

/// Iteratively create a unique-ID chain; no recursive source helper is needed.
fn call_chain(length: usize) -> (Plan, declarations::Declarations) {
    let circuits: Vec<_> = (0..length)
        .map(|i| callee(&format!("helper_{i:04}")))
        .collect();
    let mut body = Block {
        bindings: vec![],
        result: field(2 * length),
    };
    for i in (0..length).rev() {
        let formals = vec![field(2 * i + 2), field(2 * i + 3)];
        let result = field(2 * length + 2 + i);
        let binding = Binding {
            value: result.clone(),
            operation: Operation::Call {
                declaration: CallId(i),
                arguments: vec![field(2 * i), field(2 * i + 1)],
                parameters: formals,
                body,
            },
            path: format!("call[{i}]"),
        };
        body = Block {
            bindings: vec![binding],
            result,
        };
    }
    (
        Plan {
            parameters: vec![field(0), field(1)],
            body,
            result_type: Type::Field,
        },
        environment(&circuits, &[]),
    )
}

#[test]
fn call_validation_enforces_depth_budget_independently_of_lowering() {
    let (limit, _) = crate::resource_limits::recording_call_limits();
    let (below, declarations) = call_chain(2);
    validate::check(below, declarations).expect("short acyclic chain");
    let (over, declarations) = call_chain(limit + 1);
    let error = validate::check(over, declarations).unwrap_err();
    assert!(error.detail.contains("depth"), "{error:?}");
}

#[test]
fn call_validation_enforces_cumulative_work_budget() {
    let (_, limit) = crate::resource_limits::recording_call_limits();
    let mut candidate = valid_plan();
    // Each sibling is individually small and acyclic; the complete plan must
    // be bounded, rather than resetting accounting for every helper invocation.
    let calls = limit / 4 + 1;
    candidate.body.bindings = (0..calls)
        .map(|i| call_binding(0, [0, 1], 2 + 4 * i))
        .collect();
    candidate.body.result = field(1 + 4 * calls);
    let error = validate::check(candidate, standard_environment()).unwrap_err();
    assert!(error.detail.contains("work"), "{error:?}");
}
