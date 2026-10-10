// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Owned values cross local helper boundaries, never the root ABI by implication.
use super::*;

fn parameter(name: &str) -> Expr {
    Expr::Parameter { name: name.into() }
}
fn call(value: Expr) -> Expr {
    Expr::Call {
        name: "echo".into(),
        arguments: vec![value],
    }
}
fn structure(ty: Type) -> Type {
    Type::Struct {
        name: "OwnedSource".into(),
        fields: vec![StructField {
            name: "value".into(),
            ty,
        }],
    }
}
fn initial(ty: Type) -> Expr {
    Expr::StructField {
        value: Box::new(Expr::Default { ty: structure(ty) }),
        field: "value".into(),
        index: 0,
    }
}
fn source(ty: Type) -> Vec<StatefulCircuit> {
    vec![
        StatefulCircuit {
            name: "root".into(),
            source: None,
            internal: false,
            parameters: vec![],
            result: Type::Unit,
            return_value: StateReturn::Unit,
            actions: vec![StateAction::Expression {
                value: call(initial(ty.clone())),
            }],
        },
        StatefulCircuit {
            name: "echo".into(),
            source: None,
            internal: true,
            parameters: vec![Parameter {
                name: "input".into(),
                ty: ty.clone(),
            }],
            result: ty,
            actions: vec![],
            return_value: StateReturn::Expression {
                value: parameter("input"),
            },
        },
    ]
}
fn environment(source: &[StatefulCircuit]) -> declarations::Declarations {
    let calls = source.iter().map(|c| (c.name.as_str(), c)).collect();
    let types = type_environment::TypeEnvironment::from_sources(
        &source[0],
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &calls,
    )
    .unwrap();
    declarations::Declarations::with_calls(
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &calls,
    )
    .with_type_environment(types)
    .with_root(&source[0])
}
fn checked(source: &[StatefulCircuit]) -> Result<CheckedPlan, PlanError> {
    let calls = source.iter().map(|c| (c.name.as_str(), c)).collect();
    check_with_calls(
        &source[0],
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &calls,
    )
}
fn call_binding(body: &mut Block) -> &mut Binding {
    body.bindings
        .iter_mut()
        .find(|binding| matches!(binding.operation, Operation::Call { .. }))
        .unwrap()
}
fn owned_types() -> Vec<Type> {
    vec![
        Type::OpaqueString,
        Type::OpaqueBytes,
        Type::Vector {
            element: Box::new(Type::Field),
            length: 2,
        },
        Type::Tuple {
            elements: vec![Type::OpaqueBytes, Type::Boolean],
        },
        Type::Unsigned {
            max: "340282366920938463463374607431768211456".into(),
        },
    ]
}

#[test]
fn repeated_owned_helper_arguments_are_sequential_and_keep_isolated_formals() {
    for ty in owned_types() {
        let mut source = source(ty.clone());
        source[0].actions[0] = StateAction::Expression {
            value: call(call(initial(ty))),
        };
        let checked = checked(&source).unwrap();
        let calls: Vec<_> = checked
            .plan()
            .body
            .bindings
            .iter()
            .filter(|binding| matches!(binding.operation, Operation::Call { .. }))
            .collect();
        assert_eq!(calls.len(), 2);
        let Operation::Call {
            arguments,
            parameters,
            body,
            ..
        } = &calls[1].operation
        else {
            unreachable!()
        };
        assert_eq!(arguments[0], calls[0].value);
        assert_ne!(parameters[0].id, arguments[0].id);
        assert_eq!(body.result, parameters[0]);
        validate::check(checked.plan().clone(), environment(&source)).unwrap();
        render(&checked).unwrap();
    }
}

#[test]
fn owned_helpers_keep_richer_roots_separate_from_public_opaque_interfaces() {
    for ty in owned_types() {
        let mut input = source(ty.clone());
        input[0].parameters.push(Parameter {
            name: "external".into(),
            ty: ty.clone(),
        });
        if matches!(ty, Type::OpaqueString | Type::OpaqueBytes) {
            render(&checked(&input).unwrap()).unwrap();
        } else {
            assert!(matches!(checked(&input), Err(PlanError::Unsupported(_))));
        }
        let mut output = source(ty.clone());
        output[0].result = ty.clone();
        output[0].return_value = StateReturn::Expression {
            value: call(initial(ty.clone())),
        };
        if matches!(ty, Type::OpaqueString | Type::OpaqueBytes) {
            render(&checked(&output).unwrap()).unwrap();
        } else {
            assert!(matches!(checked(&output), Err(PlanError::Unsupported(_))));
        }
    }
}

#[test]
fn independent_owned_calls_recheck_signatures_arguments_and_scope() {
    let source = source(Type::OpaqueBytes);
    let original = checked(&source).unwrap().plan().clone();
    for mode in [
        "result",
        "formal_type",
        "body_type",
        "caller_capture",
        "future_argument",
        "argument_annotation",
        "reused_formal",
        "argument_count",
        "formal_count",
        "missing",
        "recursive",
    ] {
        let mut plan = original.clone();
        let binding = call_binding(&mut plan.body);
        let Operation::Call {
            declaration,
            arguments,
            parameters,
            body,
        } = &mut binding.operation
        else {
            unreachable!()
        };
        match mode {
            "result" => binding.value.ty = Type::OpaqueString,
            "formal_type" => parameters[0].ty = Type::OpaqueString,
            "body_type" => body.result.ty = Type::OpaqueString,
            "caller_capture" => body.result = arguments[0].clone(),
            "future_argument" => arguments[0].id = usize::MAX,
            "argument_annotation" => arguments[0].ty = Type::OpaqueString,
            "reused_formal" => {
                parameters[0].id = arguments[0].id;
                body.result = parameters[0].clone();
            }
            "argument_count" => arguments.clear(),
            "formal_count" => parameters.clear(),
            "missing" => *declaration = CallId(99),
            "recursive" => *declaration = environment(&source).resolve_call("root").unwrap().0,
            _ => unreachable!(),
        }
        assert!(
            validate::check(plan, environment(&source)).is_err(),
            "{mode}"
        );
    }
    // A different authoritative helper signature must not be accepted merely
    // because the lowered argument/formal/result annotations agree with each other.
    let mut wrong = source.clone();
    wrong[1].parameters[0].ty = Type::OpaqueString;
    wrong[1].result = Type::OpaqueString;
    assert!(validate::check(original, environment(&wrong)).is_err());
}

#[test]
fn owned_named_helper_products_remain_bound_to_canonical_definitions() {
    let ty = Type::Struct {
        name: "Product".into(),
        fields: vec![StructField {
            name: "bytes".into(),
            ty: Type::OpaqueBytes,
        }],
    };
    let source = source(ty);
    let original = checked(&source).unwrap().plan().clone();
    for target in ["result", "formal", "argument", "body"] {
        let mut plan = original.clone();
        let binding = call_binding(&mut plan.body);
        let Operation::Call {
            arguments,
            parameters,
            body,
            ..
        } = &mut binding.operation
        else {
            unreachable!()
        };
        let ty = match target {
            "result" => &mut binding.value.ty,
            "formal" => &mut parameters[0].ty,
            "argument" => &mut arguments[0].ty,
            _ => &mut body.result.ty,
        };
        let Type::Struct { fields, .. } = ty else {
            unreachable!()
        };
        fields[0].name = "forged".into();
        let gap = validate::check(plan, environment(&source)).unwrap_err();
        assert_eq!(
            gap.detail, "struct definition is not canonical source type",
            "{target}"
        );
    }
}

#[test]
fn unused_recursive_owned_helper_is_still_refused() {
    let mut source = source(Type::OpaqueBytes);
    source[1].return_value = StateReturn::Expression {
        value: call(parameter("input")),
    };
    let Err(PlanError::Unsupported(gap)) = checked(&source) else {
        panic!("recursive helper must be refused")
    };
    assert!(gap.detail.contains("recursive stateful call"), "{gap:?}");
}

#[test]
fn local_owned_helper_layouts_remain_bounded_before_lowering() {
    let limit = crate::resource_limits::recording_fixed_layout_limit();
    // The source wrapper adds one slot beyond the vector's own slot.
    let ty = |length| Type::Vector {
        element: Box::new(Type::Field),
        length,
    };
    checked(&source(ty(limit - 2))).unwrap();
    let Err(PlanError::Unsupported(gap)) = checked(&source(ty(limit))) else {
        panic!("oversized local helper layout must be refused")
    };
    assert!(
        gap.detail.contains("fixed aggregate layout exceeds"),
        "{gap:?}"
    );
}

#[test]
fn current_source_owned_helper_root_has_a_complete_checked_plan() {
    let contract: Contract = serde_json::from_str(include_str!(
        "../../../../tests/effect-plan-owned-calls-schema20-ir.json"
    ))
    .unwrap();
    let root = contract
        .stateful_circuits
        .iter()
        .find(|circuit| circuit.name == "run")
        .unwrap();
    let ledger = contract
        .ledger_fields
        .iter()
        .map(|field| (field.id.as_str(), field))
        .collect();
    let witnesses = contract
        .witnesses
        .iter()
        .map(|witness| (witness.name.as_str(), witness))
        .collect();
    let pure = contract
        .circuits
        .iter()
        .map(|circuit| (circuit.name.as_str(), circuit))
        .collect();
    let stateful = contract
        .stateful_circuits
        .iter()
        .map(|circuit| (circuit.name.as_str(), circuit))
        .collect();
    let checked = check_with_calls(root, &ledger, &witnesses, &pure, &stateful)
        .expect("fresh source's whole owned helper closure must compose");
    render(&checked).expect("checked fresh source must render");
}
