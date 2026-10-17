// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Native full-fold control ordering, source visits, scopes and Unit views.
use super::*;

fn field(value: &str) -> Expr {
    Expr::FieldLiteral {
        value: value.into(),
    }
}

fn sequence(steps: Vec<Expr>, value: Expr) -> Expr {
    Expr::Sequence {
        steps,
        value: Box::new(value),
    }
}

fn binding(name: &str, ty: Type, value: Expr) -> LocalBinding {
    LocalBinding {
        name: name.into(),
        ty,
        value,
    }
}

#[test]
fn assertion_and_conditional_errors_keep_child_boolean_and_arm_order() {
    let cases = [
        (
            assertion(reference("condition_missing")),
            RenderError::UnknownParameter("condition_missing".into()),
        ),
        (
            assertion(field("0")),
            RenderError::TypeMismatch {
                expected: Type::Boolean,
                actual: Type::Field,
            },
        ),
        (
            branch(
                field("0"),
                reference("then_missing"),
                reference("else_missing"),
            ),
            RenderError::TypeMismatch {
                expected: Type::Boolean,
                actual: Type::Field,
            },
        ),
        (
            branch(
                boolean(true),
                reference("then_missing"),
                reference("else_missing"),
            ),
            RenderError::UnknownParameter("then_missing".into()),
        ),
        (
            branch(boolean(true), field("1"), reference("else_missing")),
            RenderError::UnknownParameter("else_missing".into()),
        ),
        (
            branch(boolean(true), boolean(false), Expr::Unit),
            RenderError::TypeMismatch {
                expected: Type::Boolean,
                actual: Type::Unit,
            },
        ),
    ];
    for (source, expected) in cases {
        assert_eq!(
            observe(&source, &HashMap::new(), &HashMap::new()),
            Err(expected)
        );
    }
    let source = branch(
        field("0"),
        reference("then_missing"),
        reference("else_missing"),
    );
    let (outcome, visits) =
        visitation::observe(|| observe(&source, &HashMap::new(), &HashMap::new()));
    assert!(matches!(
        outcome,
        Err(RenderError::TypeMismatch {
            expected: Type::Boolean,
            actual: Type::Field
        })
    ));
    let Expr::If {
        condition,
        then,
        otherwise,
    } = &source
    else {
        unreachable!()
    };
    assert_eq!(
        visits[&(std::ptr::from_ref(condition.as_ref()) as usize)],
        1
    );
    assert!(!visits.contains_key(&(std::ptr::from_ref(then.as_ref()) as usize)));
    assert!(!visits.contains_key(&(std::ptr::from_ref(otherwise.as_ref()) as usize)));
}

#[test]
fn sequence_and_let_refuse_before_later_children_or_carrier_work() {
    let bad = Type::Unsigned { max: "bad".into() };
    let cases = [
        (
            sequence(
                vec![field("1"), reference("later_missing")],
                reference("tail_missing"),
            ),
            RenderError::TypeMismatch {
                expected: Type::Unit,
                actual: Type::Field,
            },
        ),
        (
            sequence(
                vec![reference("first_missing"), field("1")],
                reference("tail_missing"),
            ),
            RenderError::UnknownParameter("first_missing".into()),
        ),
        (
            local(
                "bad-name",
                Type::Field,
                reference("initializer_missing"),
                reference("body_missing"),
            ),
            RenderError::InvalidIdentifier("bad-name".into()),
        ),
        (
            local(
                "x",
                bad.clone(),
                reference("initializer_missing"),
                reference("body_missing"),
            ),
            RenderError::UnknownParameter("initializer_missing".into()),
        ),
        (
            local("x", bad.clone(), boolean(true), reference("body_missing")),
            RenderError::TypeMismatch {
                expected: bad.clone(),
                actual: Type::Boolean,
            },
        ),
    ];
    for (source, expected) in cases {
        assert_eq!(
            observe(&source, &HashMap::new(), &HashMap::new()),
            Err(expected)
        );
    }
    let params = HashMap::from([("prior", (&bad, syn::parse_quote!(prior)))]);
    assert_eq!(
        observe(
            &local(
                "x",
                bad.clone(),
                reference("prior"),
                reference("body_missing")
            ),
            &params,
            &HashMap::new()
        ),
        Err(RenderError::InvalidUnsignedMaximum("bad".into()))
    );
    let source = sequence(
        vec![field("1"), reference("later_missing")],
        reference("tail_missing"),
    );
    let (_, visits) = visitation::observe(|| observe(&source, &HashMap::new(), &HashMap::new()));
    let Expr::Sequence { steps, value } = &source else {
        unreachable!()
    };
    assert_eq!(visits[&(std::ptr::from_ref(&steps[0]) as usize)], 1);
    assert!(!visits.contains_key(&(std::ptr::from_ref(&steps[1]) as usize)));
    assert!(!visits.contains_key(&(std::ptr::from_ref(value.as_ref()) as usize)));
}

#[test]
fn equal_nested_arms_check_both_source_occurrences_once_before_output_coalescing() {
    let arm = sequence(vec![assertion(boolean(true))], Expr::Unit);
    let source = branch(call("probe"), arm.clone(), arm);
    let probe = pure("probe", Type::Boolean);
    let circuits = HashMap::from([("probe", &probe)]);
    let (outcome, visits) = visitation::observe(|| observe(&source, &HashMap::new(), &circuits));
    assert_eq!(
        outcome,
        old_oracle::observe(&source, &HashMap::new(), &circuits)
    );
    let Expr::If {
        condition,
        then,
        otherwise,
    } = &source
    else {
        unreachable!()
    };
    assert_ne!(
        std::ptr::from_ref(then.as_ref()),
        std::ptr::from_ref(otherwise.as_ref())
    );
    let mut nodes = vec![&source, condition.as_ref()];
    for arm in [then.as_ref(), otherwise.as_ref()] {
        let Expr::Sequence { steps, value } = arm else {
            unreachable!()
        };
        let Expr::Assert { condition, .. } = &steps[0] else {
            unreachable!()
        };
        nodes.extend([arm, &steps[0], condition.as_ref(), value.as_ref()]);
    }
    assert_eq!(visits.len(), nodes.len());
    for node in nodes {
        assert_eq!(visits[&(std::ptr::from_ref(node) as usize)], 1);
    }
}

#[test]
fn sequential_checked_raw_key_shadows_read_prior_scope_and_restore_the_outer_binding() {
    let field_ty = Type::Field;
    let parameters = HashMap::from([("r#value", (&field_ty, syn::parse_quote!(incoming_value)))]);
    let inner = Expr::Let {
        bindings: vec![
            binding("r#value", Type::Field, reference("r#value")),
            binding("captured", Type::Field, reference("r#value")),
            binding(
                "r#value",
                Type::Field,
                Expr::Add {
                    left: Box::new(reference("r#value")),
                    right: Box::new(field("1")),
                },
            ),
        ],
        body: Box::new(Expr::Tuple {
            elements: vec![
                reference("r#value"),
                reference("captured"),
                local("r#value", Type::Field, field("2"), reference("r#value")),
            ],
        }),
    };
    let source = Expr::Tuple {
        elements: vec![inner, reference("r#value")],
    };
    let (result, visits) =
        visitation::observe(|| crate::expression_with_calls(&source, &parameters, &HashMap::new()));
    let (syntax, ty) = result.unwrap();
    assert_eq!(
        ty,
        Type::Tuple {
            elements: vec![
                Type::Tuple {
                    elements: vec![Type::Field; 3]
                },
                Type::Field
            ]
        }
    );
    assert!(visits.values().all(|count| *count == 1));
    let syn::Expr::Tuple(tuple) = syntax else {
        panic!("outer tuple")
    };
    assert_eq!(
        tuple.elems.last().unwrap().to_token_stream().to_string(),
        "incoming_value"
    );
    let syn::Expr::Block(inner) = tuple.elems.first().unwrap() else {
        panic!("Let block")
    };
    let locals: Vec<_> = inner
        .block
        .stmts
        .iter()
        .filter_map(|statement| match statement {
            syn::Stmt::Local(local) => Some(local),
            _ => None,
        })
        .collect();
    assert_eq!(locals.len(), 3);
    assert_eq!(
        locals[0]
            .init
            .as_ref()
            .unwrap()
            .expr
            .to_token_stream()
            .to_string(),
        "incoming_value"
    );
    assert_eq!(
        locals[1]
            .init
            .as_ref()
            .unwrap()
            .expr
            .to_token_stream()
            .to_string(),
        "__compact_local_value"
    );
    assert!(
        locals[2]
            .init
            .as_ref()
            .unwrap()
            .expr
            .to_token_stream()
            .to_string()
            .starts_with("__compact_local_value +")
    );
    let body = inner.block.stmts.last().unwrap();
    let syn::Stmt::Expr(syn::Expr::Tuple(body), _) = body else {
        panic!("Let tuple tail")
    };
    assert_eq!(
        body.elems[0].to_token_stream().to_string(),
        "__compact_local_value"
    );
    assert_eq!(
        body.elems[1].to_token_stream().to_string(),
        "__compact_local_captured"
    );
    assert_eq!(parameters["r#value"].1.to_string(), "incoming_value");
}

#[test]
fn inferred_unit_parameter_projections_calls_and_coercions_keep_statement_fallback() {
    let unit = Type::Unit;
    let parameters = HashMap::from([("unit", (&unit, syn::parse_quote!(unit)))]);
    let mark = pure("mark", Type::Unit);
    let circuits = HashMap::from([("mark", &mark)]);
    let holder = Type::Struct {
        name: "UnitHolder".into(),
        fields: vec![StructField {
            name: "member".into(),
            ty: Type::Unit,
        }],
    };
    let cases = [
        reference("unit"),
        Expr::StructField {
            value: Box::new(Expr::StructLiteral {
                ty: holder,
                fields: vec![Expr::Unit],
            }),
            field: "member".into(),
            index: 0,
        },
        Expr::TupleIndex {
            value: Box::new(Expr::Tuple {
                elements: vec![Expr::Unit, boolean(true)],
            }),
            index: 0,
        },
        call("mark"),
        Expr::Coerce {
            value: Box::new(reference("unit")),
            ty: Type::Unit,
        },
    ];
    for source in cases {
        let (outcome, visits) = visitation::observe(|| observe(&source, &parameters, &circuits));
        assert_eq!(
            outcome,
            old_oracle::observe(&source, &parameters, &circuits)
        );
        let (_, statements, ty) = outcome.unwrap();
        assert_eq!(ty, Type::Unit);
        assert!(
            statements.ends_with(';'),
            "ordinary inferred Unit needs an expression statement: {statements}"
        );
        assert!(visits.values().all(|count| *count == 1));
        let (fragment, _) =
            crate::native_fragment_with_calls(&source, &parameters, &circuits).unwrap();
        assert_eq!(fragment.into_views().1.len(), 1);
    }
    for source in [Expr::Unit, Expr::Default { ty: Type::Unit }] {
        assert!(
            observe(&source, &parameters, &circuits)
                .unwrap()
                .1
                .is_empty()
        );
    }
}

#[test]
fn map_body_controls_use_one_fold_and_keep_occupied_local_symbols_and_outer_scope() {
    let field_ty = Type::Field;
    let parameters = HashMap::from([("outside", (&field_ty, syn::parse_quote!(outside)))]);
    let mapped = Expr::VectorMap {
        parameter: Parameter {
            name: "__compact_local_x".into(),
            ty: Type::Field,
        },
        source: Box::new(Expr::Vector {
            element: Type::Field,
            elements: vec![field("1")],
        }),
        body: Box::new(sequence(
            vec![assertion(boolean(true))],
            Expr::Let {
                bindings: vec![
                    binding("x", Type::Field, reference("__compact_local_x")),
                    binding(
                        "x",
                        Type::Field,
                        Expr::Add {
                            left: Box::new(reference("x")),
                            right: Box::new(field("2")),
                        },
                    ),
                ],
                body: Box::new(Expr::Tuple {
                    elements: vec![reference("__compact_local_x"), reference("x")],
                }),
            },
        )),
        result: Type::Tuple {
            elements: vec![Type::Field, Type::Field],
        },
        length: 1,
    };
    let source = Expr::Tuple {
        elements: vec![mapped, reference("outside")],
    };
    let (result, visits) = visitation::observe(|| observe(&source, &parameters, &HashMap::new()));
    let (expression, _, ty) = result.unwrap();
    assert_eq!(
        ty,
        Type::Tuple {
            elements: vec![
                Type::Vector {
                    element: Box::new(Type::Tuple {
                        elements: vec![Type::Field, Type::Field]
                    }),
                    length: 1
                },
                Type::Field
            ]
        }
    );
    assert_eq!(visits.len(), 16);
    assert!(visits.values().all(|count| *count == 1));
    assert_eq!(expression.matches("let __compact_local_x_1").count(), 2);
    assert!(expression.contains("| __compact_local_x |"));
    assert!(expression.contains("(__compact_local_x , __compact_local_x_1)"));
    assert_eq!(parameters["outside"].1.to_string(), "outside");
}
