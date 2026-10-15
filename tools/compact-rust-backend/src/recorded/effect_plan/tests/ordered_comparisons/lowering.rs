// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;

#[test]
fn every_operator_accepts_independent_canonical_small_bounds_and_preserves_syntax() {
    for operator in OPERATORS {
        for (a, b) in [
            ("0", "0"),
            ("255", "65535"),
            ("65535", "255"),
            ("1", U128_MAX),
            (U128_MAX, U128_MAX),
        ] {
            let source = root(operator, validate::uint(a), validate::uint(b));
            let plan = checked(&source);
            assert_eq!(plan.plan().body.bindings.len(), 1);
            let binding = &plan.plan().body.bindings[0];
            assert_eq!(binding.value.ty, Type::Boolean);
            let Operation::Compare {
                operator: actual,
                left,
                right,
            } = &binding.operation
            else {
                panic!("expected comparison")
            };
            assert_eq!(*actual, operator);
            assert_eq!(left, &plan.plan().parameters[0]);
            assert_eq!(right, &plan.plan().parameters[1]);
            let steps = render(&plan).unwrap().steps;
            let syn::Stmt::Local(local) = steps.last().unwrap() else {
                panic!("SSA binding")
            };
            let syn::Expr::Binary(binary) = local.init.as_ref().unwrap().expr.as_ref() else {
                panic!("binary ordering")
            };
            assert!(matches!(
                (operator, &binary.op),
                (ComparisonOperator::Less, syn::BinOp::Lt(_))
                    | (ComparisonOperator::LessEqual, syn::BinOp::Le(_))
                    | (ComparisonOperator::Greater, syn::BinOp::Gt(_))
                    | (ComparisonOperator::GreaterEqual, syn::BinOp::Ge(_))
            ));
            for input in [&binary.left, &binary.right] {
                assert!(
                    matches!(input.as_ref(), syn::Expr::MethodCall(call) if call.method == "value" && call.args.is_empty())
                );
            }
            // This is intentionally direct typed IR: source normalization may equalize bounds.
            let parameters = source
                .parameters
                .iter()
                .map(|p| {
                    (
                        p.name.as_str(),
                        (&p.ty, crate::naming::ident(&p.name).unwrap()),
                    )
                })
                .collect();
            let StateReturn::Expression { value } = &source.return_value else {
                unreachable!()
            };
            let (_, native_type) =
                crate::expression_with_calls(value, &parameters, &HashMap::new()).unwrap();
            assert_eq!(native_type, Type::Boolean);
        }
    }
}
#[test]
fn each_operand_rejects_other_domains_and_noncanonical_or_wide_bounds() {
    let mut invalid = unsupported_types();
    invalid.extend(["", "01", "+1", "-1", " 1", "1 ", "one"].map(validate::uint));
    for operator in OPERATORS {
        for ty in &invalid {
            for (left, right) in [
                (ty.clone(), validate::uint("255")),
                (validate::uint("255"), ty.clone()),
            ] {
                let result = check(
                    &root(operator, left, right),
                    &HashMap::new(),
                    &HashMap::new(),
                );
                assert!(
                    matches!(result, Err(PlanError::Unsupported(_))),
                    "{operator:?}/{ty:?}"
                );
            }
        }
    }
}
fn witness(name: &str) -> Expr {
    Expr::WitnessCall {
        name: name.into(),
        arguments: vec![],
    }
}
fn witnesses() -> [WitnessDeclaration; 2] {
    [("left", "255"), ("right", "65535")].map(|(name, max)| WitnessDeclaration {
        source: None,
        name: name.into(),
        parameters: vec![],
        result: validate::uint(max),
    })
}
#[test]
fn witnesses_evaluate_once_left_then_right_before_each_operator() {
    for operator in OPERATORS {
        let mut source = root(operator, validate::uint("255"), validate::uint("65535"));
        source.parameters.clear();
        source.return_value = StateReturn::Expression {
            value: compare(operator, witness("left"), witness("right")),
        };
        let ws = witnesses();
        let plan = check(
            &source,
            &HashMap::new(),
            &ws.iter().map(|w| (w.name.as_str(), w)).collect(),
        )
        .unwrap();
        let bindings = &plan.plan().body.bindings;
        assert_eq!(bindings.len(), 3);
        for (at, name) in [(0, "left"), (1, "right")] {
            let Operation::Witness { declaration, .. } = bindings[at].operation else {
                panic!("witness")
            };
            assert_eq!(plan.declarations().witness(declaration).unwrap().name, name);
        }
        let Operation::Compare { left, right, .. } = &bindings[2].operation else {
            panic!("comparison")
        };
        assert_eq!(left, &bindings[0].value);
        assert_eq!(right, &bindings[1].value);
    }
}
#[test]
fn comparisons_keep_witness_operands_inside_lazy_branches() {
    let mut source = root(
        ComparisonOperator::Less,
        validate::uint("255"),
        validate::uint("65535"),
    );
    source.parameters = vec![Parameter {
        name: "selected".into(),
        ty: Type::Boolean,
    }];
    source.return_value = StateReturn::Expression {
        value: Expr::If {
            condition: Box::new(reference("selected")),
            then: Box::new(compare(
                ComparisonOperator::Less,
                witness("left"),
                witness("right"),
            )),
            otherwise: Box::new(Expr::Boolean { value: false }),
        },
    };
    let ws = witnesses();
    let plan = check(
        &source,
        &HashMap::new(),
        &ws.iter().map(|w| (w.name.as_str(), w)).collect(),
    )
    .unwrap();
    assert_eq!(plan.plan().body.bindings.len(), 1);
    let Operation::If {
        then, otherwise, ..
    } = &plan.plan().body.bindings[0].operation
    else {
        panic!("lazy branch")
    };
    assert_eq!(then.bindings.len(), 3);
    assert!(matches!(
        then.bindings[2].operation,
        Operation::Compare { .. }
    ));
    assert_eq!(otherwise.bindings.len(), 1);
    assert!(matches!(
        otherwise.bindings[0].operation,
        Operation::Literal(Literal::Boolean(false))
    ));
}
