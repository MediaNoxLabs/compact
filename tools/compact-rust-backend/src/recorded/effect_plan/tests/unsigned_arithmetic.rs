// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Small unsigned arithmetic laws are shared with native emission, not inferred
//! from fixture shapes. These are compiler controls, not runtime overflow tests.
use super::*;
#[path = "unsigned_arithmetic/validation.rs"]
mod validation;

#[derive(Clone, Copy, Debug)]
enum Arithmetic {
    Add,
    Subtract,
    Multiply,
}
const OPS: [Arithmetic; 3] = [Arithmetic::Add, Arithmetic::Subtract, Arithmetic::Multiply];
const WIDE: &str = "340282366920938463463374607431768211456";
impl Arithmetic {
    fn expression(self, left: Expr, right: Expr, max: &str) -> Expr {
        let (left, right, max) = (Box::new(left), Box::new(right), max.into());
        match self {
            Self::Add => Expr::UnsignedAdd { left, right, max },
            Self::Subtract => Expr::UnsignedSubtract { left, right, max },
            Self::Multiply => Expr::UnsignedMultiply { left, right, max },
        }
    }
    fn helper(self) -> &'static str {
        match self {
            Self::Add => "add_unsigned",
            Self::Subtract => "subtract_unsigned",
            Self::Multiply => "multiply_unsigned",
        }
    }
}
fn unsigned(max: &str) -> Type {
    Type::Unsigned { max: max.into() }
}
fn zero(max: &str) -> Expr {
    Expr::UnsignedLiteral {
        value: "0".into(),
        max: max.into(),
    }
}
fn root(expression: Expr) -> StatefulCircuit {
    StatefulCircuit {
        source: None,
        internal: false,
        name: "calculate".into(),
        parameters: vec![],
        result: Type::Unit,
        return_value: StateReturn::Unit,
        actions: vec![StateAction::Expression { value: expression }],
    }
}
fn checked(expression: Expr) -> Result<CheckedPlan, PlanError> {
    check(&root(expression), &HashMap::new(), &HashMap::new())
}
fn operands(operation: &mut Operation) -> Option<(&mut Value, &mut Value)> {
    match operation {
        Operation::UnsignedAdd(a, b)
        | Operation::UnsignedSubtract(a, b)
        | Operation::UnsignedMultiply(a, b) => Some((a, b)),
        _ => None,
    }
}
fn independent(plan: Plan) -> Result<CheckedPlan, RecordingGap> {
    validate::check(
        plan,
        declarations::Declarations::new(&HashMap::new(), &HashMap::new()),
    )
}
fn arithmetic_position(plan: &Plan) -> usize {
    plan.body
        .bindings
        .iter()
        .position(|b| {
            matches!(
                b.operation,
                Operation::UnsignedAdd(..)
                    | Operation::UnsignedSubtract(..)
                    | Operation::UnsignedMultiply(..)
            )
        })
        .unwrap()
}
fn rendered_constants(checked: &CheckedPlan, helper: &str) -> Vec<String> {
    let rendered = render(checked).unwrap();
    let matching: Vec<_> = rendered
        .steps
        .iter()
        .filter_map(|statement| {
            let syn::Stmt::Local(local) = statement else {
                return None;
            };
            let syn::Expr::Try(tried) = local.init.as_ref()?.expr.as_ref() else {
                return None;
            };
            let syn::Expr::Call(call) = tried.expr.as_ref() else {
                return None;
            };
            let syn::Expr::Path(path) = call.func.as_ref() else {
                return None;
            };
            let segment = path.path.segments.last()?;
            if segment.ident != helper {
                return None;
            }
            let syn::PathArguments::AngleBracketed(arguments) = &segment.arguments else {
                panic!("bounded helper const arguments")
            };
            Some(
                arguments
                    .args
                    .iter()
                    .map(|argument| {
                        let syn::GenericArgument::Const(syn::Expr::Lit(literal)) = argument else {
                            panic!("integer const bound expected")
                        };
                        let syn::Lit::Int(integer) = &literal.lit else {
                            panic!("integer bound")
                        };
                        integer.base10_digits().to_owned()
                    })
                    .collect::<Vec<_>>(),
            )
        })
        .collect();
    assert_eq!(matching.len(), 1, "one fallible {helper} invocation");
    matching.into_iter().next().unwrap()
}

#[test]
fn independent_small_bounds_including_zero_and_u128_max_reach_exact_runtime_helpers() {
    let maximum = u128::MAX.to_string();
    let cases = [
        ("0", "0", "0"),
        ("3", "255", "1"),
        ("255", "3", "65535"),
        (maximum.as_str(), "1", maximum.as_str()),
        ("1", maximum.as_str(), "0"),
    ];
    for op in OPS {
        for (left, right, result) in cases {
            let checked = checked(op.expression(zero(left), zero(right), result)).unwrap();
            let plan = checked.plan();
            assert_eq!(
                plan.body.bindings[arithmetic_position(plan)].value.ty,
                unsigned(result)
            );
            independent(plan.clone()).unwrap();
            assert_eq!(
                rendered_constants(&checked, op.helper()),
                [left, right, result]
            );
        }
    }
}

#[test]
fn operands_are_ordered_once_and_no_static_range_rule_replaces_runtime_failure() {
    let witnesses: Vec<_> = ["left", "right"]
        .into_iter()
        .map(|name| WitnessDeclaration {
            name: name.into(),
            source: None,
            parameters: vec![],
            result: unsigned("255"),
        })
        .collect();
    let env = witnesses.iter().map(|w| (w.name.as_str(), w)).collect();
    for op in OPS {
        let circuit = root(op.expression(
            Expr::WitnessCall {
                name: "left".into(),
                arguments: vec![],
            },
            Expr::WitnessCall {
                name: "right".into(),
                arguments: vec![],
            },
            "0",
        ));
        let checked = check(&circuit, &HashMap::new(), &env).unwrap();
        let plan = checked.plan();
        let mut operation = plan.body.bindings[arithmetic_position(plan)]
            .operation
            .clone();
        let (left, right) = operands(&mut operation).unwrap();
        let calls: Vec<_> = plan
            .body
            .bindings
            .iter()
            .filter_map(|b| match &b.operation {
                Operation::Witness { declaration, .. } => Some((declaration.0, b.value.id)),
                _ => None,
            })
            .collect();
        assert_eq!(calls, [(0, left.id), (1, right.id)]);
        assert_eq!(
            rendered_constants(&checked, op.helper()),
            ["255", "255", "0"]
        );
    }
}

#[test]
fn arithmetic_keeps_native_wide_operand_and_result_boundaries() {
    for op in OPS {
        for (left, right, result) in [
            (WIDE, "255", "255"),
            ("255", WIDE, "255"),
            ("255", "255", WIDE),
        ] {
            let outcome = checked(op.expression(zero(left), zero(right), result));
            if matches!(op, Arithmetic::Add) && result == WIDE {
                let plan = outcome.expect("native wide-result addition");
                render(&plan).unwrap();
            } else {
                assert!(
                    matches!(outcome, Err(PlanError::Unsupported(_))),
                    "{op:?}: {left}/{right}/{result}"
                );
            }
        }
        for bad in [
            Expr::Boolean { value: true },
            Expr::FieldLiteral { value: "0".into() },
        ] {
            assert!(matches!(
                checked(op.expression(bad.clone(), zero("255"), "255")),
                Err(PlanError::Unsupported(_))
            ));
            assert!(matches!(
                checked(op.expression(zero("255"), bad, "255")),
                Err(PlanError::Unsupported(_))
            ));
        }
    }
}
