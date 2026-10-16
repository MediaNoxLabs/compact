// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! The native addition carrier law, independently checked before rendering.
use super::*;
#[path = "wide_add/inclusion.rs"]
mod inclusion;
const MAX128: &str = "340282366920938463463374607431768211455";
const TWO128: &str = "340282366920938463463374607431768211456";
const MAX129: &str = "680564733841876926926749214863536422911";
const MAX248: &str = "452312848583266388373324160190187140051835877600158453279131187530910662655";
const TWO248: &str = "452312848583266388373324160190187140051835877600158453279131187530910662656";
fn reference(name: &str) -> Expr {
    Expr::Parameter { name: name.into() }
}
fn add(left: Expr, right: Expr, max: &str) -> Expr {
    Expr::UnsignedAdd {
        left: Box::new(left),
        right: Box::new(right),
        max: max.into(),
    }
}
fn root(left: Type, right: Type, max: &str) -> StatefulCircuit {
    StatefulCircuit {
        source: None,
        internal: false,
        name: "sum".into(),
        parameters: vec![
            Parameter {
                name: "left".into(),
                ty: left,
            },
            Parameter {
                name: "right".into(),
                ty: right,
            },
        ],
        actions: vec![StateAction::Expression {
            value: add(reference("left"), reference("right"), max),
        }],
        result: Type::Boolean,
        return_value: StateReturn::Expression {
            value: Expr::Boolean { value: true },
        },
    }
}
fn checked(source: &StatefulCircuit) -> CheckedPlan {
    check(source, &HashMap::new(), &HashMap::new()).unwrap()
}
fn authority(source: &StatefulCircuit) -> declarations::Declarations {
    let types = type_environment::TypeEnvironment::from_sources(
        source,
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
    )
    .unwrap();
    declarations::Declarations::default()
        .with_type_environment(types)
        .with_root(source)
}
fn addition(plan: &mut Plan) -> &mut Binding {
    plan.body
        .bindings
        .iter_mut()
        .find(|b| matches!(b.operation, Operation::UnsignedAdd(..)))
        .unwrap()
}
fn rendered(plan: &CheckedPlan) -> String {
    let steps = render(plan).unwrap().steps;
    quote::quote!({ #(#steps)* }).to_string()
}
#[test]
fn wide_add_accepts_exact_native_carrier_domain_and_keeps_fallible_helper() {
    for (left, right, result) in [
        ("0", "0", TWO128),
        (MAX128, MAX128, MAX129),
        (TWO128, "255", MAX129),
        ("255", TWO128, MAX129),
        (MAX248, MAX248, MAX248),
        (MAX248, MAX248, TWO128),
    ] {
        let source = root(validate::uint(left), validate::uint(right), result);
        let plan = checked(&source);
        validate::check(plan.plan().clone(), authority(&source)).unwrap();
        let text = rendered(&plan);
        assert_eq!(text.matches("add_wide_unsigned").count(), 1, "{text}");
        assert!(text.contains('?'));
        // The runtime may reject a sum exceeding the result bound. Lowering
        // must not require the static sum of maxima to fit that bound.
        assert_eq!(
            addition(&mut plan.plan().clone()).value.ty,
            validate::uint(result)
        );
    }
}
#[test]
fn wide_add_refuses_noncanonical_bounds_and_implicit_narrowing() {
    for malformed in ["", "01", "+1", "-1", "invalid", TWO248] {
        for (left, right, result) in [
            (malformed, "255", MAX248),
            ("255", malformed, MAX248),
            ("255", "255", malformed),
        ] {
            assert!(
                matches!(
                    check(
                        &root(validate::uint(left), validate::uint(right), result),
                        &HashMap::new(),
                        &HashMap::new()
                    ),
                    Err(PlanError::Unsupported(_))
                ),
                "{left}/{right}/{result}"
            );
        }
    }
    for (left, right) in [(MAX248, "255"), ("255", MAX248), (MAX248, MAX248)] {
        assert!(matches!(
            check(
                &root(validate::uint(left), validate::uint(right), MAX128),
                &HashMap::new(),
                &HashMap::new()
            ),
            Err(PlanError::Unsupported(_))
        ));
    }
    for ty in [Type::Field, Type::Boolean, Type::OpaqueBytes, Type::Unit] {
        for (left, right) in [
            (ty.clone(), validate::uint(MAX248)),
            (validate::uint(MAX248), ty),
        ] {
            assert!(matches!(
                check(&root(left, right, MAX248), &HashMap::new(), &HashMap::new()),
                Err(PlanError::Unsupported(_))
            ));
        }
    }
}
#[test]
fn wide_add_validator_checks_both_ssa_operands_and_result_independently() {
    let source = root(validate::uint(MAX248), validate::uint("255"), MAX129);
    let original = checked(&source).plan().clone();
    for side in [0, 1] {
        for mode in ["missing", "annotation", "self", "malformed"] {
            let mut forged = original.clone();
            let binding = addition(&mut forged);
            let Operation::UnsignedAdd(left, right) = &mut binding.operation else {
                unreachable!()
            };
            let value = if side == 0 { left } else { right };
            match mode {
                "missing" => value.id = 999,
                "annotation" => value.ty = validate::uint(TWO128),
                "self" => value.id = binding.value.id,
                "malformed" => value.ty = validate::uint("01"),
                _ => unreachable!(),
            }
            let gap = validate::check(forged, authority(&source)).unwrap_err();
            assert_eq!(gap.path, "actions[0].value", "{side}/{mode}: {gap:?}");
        }
    }
    for ty in [
        Type::Field,
        Type::Boolean,
        Type::Unit,
        validate::uint("255"),
        validate::uint("01"),
        validate::uint(TWO248),
    ] {
        let mut forged = original.clone();
        addition(&mut forged).value.ty = ty;
        let gap = validate::check(forged, authority(&source)).unwrap_err();
        assert_eq!(gap.path, "actions[0].value");
    }
    // An in-scope nonunsigned operand must fail the operation law, not only
    // the use-site annotation check. Keep source authority internally coherent.
    for side in [0, 1] {
        let mut source = source.clone();
        source.parameters[side].ty = Type::Field;
        let mut forged = original.clone();
        forged.parameters[side].ty = Type::Field;
        let Operation::UnsignedAdd(left, right) = &mut addition(&mut forged).operation else {
            unreachable!()
        };
        (if side == 0 { left } else { right }).ty = Type::Field;
        assert_eq!(
            validate::check(forged, authority(&source))
                .unwrap_err()
                .path,
            "actions[0].value"
        );
    }
    // Changing the root signature together with a result value cannot alter
    // the immutable declared Boolean result.
    let mut forged = original;
    forged.result_type = validate::uint(MAX129);
    forged.body.result = addition(&mut forged).value.clone();
    assert!(validate::check(forged, authority(&source)).is_err());
}
#[test]
fn wide_add_evaluates_witness_operands_once_left_then_right_before_narrowing() {
    let declarations: Vec<_> = [("left", MAX248), ("right", "255")]
        .into_iter()
        .map(|(name, max)| WitnessDeclaration {
            source: None,
            name: name.into(),
            parameters: vec![],
            result: validate::uint(max),
        })
        .collect();
    let env = declarations.iter().map(|d| (d.name.as_str(), d)).collect();
    let witness = |name: &str| Expr::WitnessCall {
        name: name.into(),
        arguments: vec![],
    };
    let mut source = root(Type::Unit, Type::Unit, MAX129);
    source.parameters.clear();
    source.actions = vec![StateAction::Expression {
        value: Expr::UnsignedCast {
            value: Box::new(add(witness("left"), witness("right"), MAX129)),
            max: MAX128.into(),
        },
    }];
    let plan = check(&source, &HashMap::new(), &env).unwrap();
    let bindings = &plan.plan().body.bindings;
    assert!(matches!(
        bindings[0].operation,
        Operation::Witness {
            declaration: WitnessId(0),
            ..
        }
    ));
    assert!(matches!(
        bindings[1].operation,
        Operation::Witness {
            declaration: WitnessId(1),
            ..
        }
    ));
    let Operation::UnsignedAdd(left, right) = &bindings[2].operation else {
        panic!("addition after both witnesses")
    };
    assert_eq!(left, &bindings[0].value);
    assert_eq!(right, &bindings[1].value);
    let Operation::Convert {
        value,
        kind: Conversion::UnsignedCast,
    } = &bindings[3].operation
    else {
        panic!("explicit narrowing after addition")
    };
    assert_eq!(value, &bindings[2].value);
    let text = rendered(&plan);
    assert_eq!(text.matches("try_witness_metered").count(), 2);
    assert!(text.rfind("try_witness_metered").unwrap() < text.find("add_wide_unsigned").unwrap());
    assert!(text.find("add_wide_unsigned").unwrap() < text.find("narrow_wide_uint").unwrap());
}
#[test]
fn wide_add_cannot_capture_a_value_allocated_only_inside_a_branch() {
    let mut source = root(Type::Unit, Type::Unit, MAX129);
    source.parameters.clear();
    let zero = || Expr::UnsignedLiteral {
        value: "0".into(),
        max: MAX248.into(),
    };
    source.actions = vec![StateAction::Expression {
        value: add(zero(), zero(), MAX129),
    }];
    let mut candidate = checked(&source).plan().clone();
    let left = candidate.body.bindings.remove(0);
    let condition = Value {
        id: 100,
        ty: Type::Boolean,
    };
    let unit = |id| Binding {
        value: Value { id, ty: Type::Unit },
        operation: Operation::Literal(Literal::Unit),
        path: format!("unit{id}"),
    };
    candidate.body.bindings.insert(
        0,
        Binding {
            value: Value {
                id: 101,
                ty: Type::Unit,
            },
            path: "branch".into(),
            operation: Operation::If {
                condition: condition.clone(),
                then: Block {
                    bindings: vec![left, unit(102)],
                    result: unit(102).value,
                },
                otherwise: Block {
                    bindings: vec![unit(103)],
                    result: unit(103).value,
                },
            },
        },
    );
    candidate.body.bindings.insert(
        0,
        Binding {
            value: condition,
            operation: Operation::Literal(Literal::Boolean(true)),
            path: "condition".into(),
        },
    );
    let gap = validate::check(candidate, authority(&source)).unwrap_err();
    assert_eq!(gap.path, "actions[0].value");
    assert!(gap.detail.contains("out of scope"));
}
