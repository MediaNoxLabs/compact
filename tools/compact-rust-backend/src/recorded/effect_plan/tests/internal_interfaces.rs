// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Local root interfaces retain immutable visibility; public FAB stays bounded.
use super::*;

#[path = "internal_interfaces/source.rs"]
mod source;

fn tuple(n: usize) -> Type {
    Type::Tuple {
        elements: vec![Type::OpaqueBytes; n],
    }
}
fn vector(element: Type, length: usize) -> Type {
    Type::Vector {
        element: Box::new(element),
        length,
    }
}
fn named(ty: Type) -> Type {
    Type::Struct {
        name: "Product".into(),
        fields: vec![StructField {
            name: "value".into(),
            ty,
        }],
    }
}
fn root(ty: Type, internal: bool) -> StatefulCircuit {
    StatefulCircuit {
        source: None,
        name: "echo".into(),
        internal,
        parameters: vec![Parameter {
            name: "input".into(),
            ty: ty.clone(),
        }],
        actions: vec![],
        result: ty,
        return_value: StateReturn::Expression {
            value: Expr::Parameter {
                name: "input".into(),
            },
        },
    }
}
fn environment(source: &StatefulCircuit) -> type_environment::TypeEnvironment {
    type_environment::TypeEnvironment::from_sources(
        source,
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
    )
    .unwrap()
}
fn authority(source: &StatefulCircuit) -> declarations::Declarations {
    declarations::Declarations::default()
        .with_type_environment(environment(source))
        .with_root(source)
}
fn checked(source: &StatefulCircuit) -> CheckedPlan {
    check(source, &HashMap::new(), &HashMap::new()).unwrap()
}
fn rewrite(plan: &Plan, ty: Type) -> Plan {
    let mut plan = plan.clone();
    plan.parameters[0].ty = ty.clone();
    plan.body.result.ty = ty.clone();
    plan.result_type = ty;
    plan
}

#[test]
fn internal_interfaces_use_local_values_without_widening_public_fab() {
    for ty in [
        tuple(12),
        tuple(13),
        vector(tuple(12), 2),
        vector(tuple(13), 0),
    ] {
        let local = root(ty.clone(), true);
        let plan = checked(&local);
        render(&plan).unwrap();
        validate::check(plan.plan().clone(), authority(&local)).unwrap();
        assert!(matches!(
            check(&root(ty, false), &HashMap::new(), &HashMap::new()),
            Err(PlanError::Unsupported(_))
        ));
    }
    for internal in [false, true] {
        render(&checked(&root(tuple(11), internal))).unwrap();
        render(&checked(&root(named(tuple(8)), internal))).unwrap();
        for ty in [
            named(tuple(9)),
            named(vector(tuple(9), 2)),
            Type::LedgerMap {
                key: Box::new(Type::Field),
                value: Box::new(Type::Field),
            },
        ] {
            assert!(matches!(
                check(&root(ty, internal), &HashMap::new(), &HashMap::new()),
                Err(PlanError::Unsupported(_))
            ));
        }
    }
}

#[test]
fn root_visibility_comes_from_retained_authority_not_the_candidate() {
    // Visibility controls parameter encoding, independently of result eligibility.
    let mut parameter = root(tuple(12), true);
    parameter.result = Type::Boolean;
    parameter.return_value = StateReturn::Expression {
        value: Expr::Boolean { value: true },
    };
    let plan = checked(&parameter);
    let mut public = parameter.clone();
    public.internal = false;
    let public_env = authority(&public);
    assert!(!public_env.root().unwrap().internal);
    assert!(validate::check(plan.plan().clone(), public_env).is_err());
    let absent =
        declarations::Declarations::default().with_type_environment(environment(&parameter));
    assert!(validate::check(plan.plan().clone(), absent).is_err());
    validate::check(plan.plan().clone(), authority(&parameter)).unwrap();
}

#[test]
fn execution_results_use_local_values_independently_of_root_visibility() {
    let mut result = root(
        Type::Tuple {
            elements: vec![Type::Field; 12],
        },
        true,
    );
    result.parameters.clear();
    result.return_value = StateReturn::Expression {
        value: Expr::Tuple {
            elements: vec![Expr::FieldLiteral { value: "1".into() }; 12],
        },
    };
    let plan = checked(&result);
    for internal in [false, true] {
        let mut source = result.clone();
        source.internal = internal;
        validate::check(plan.plan().clone(), authority(&source)).unwrap();
        checked(&source);
    }
    // Standalone validation also accepts local results without granting public
    // parameter encoding. Declared result and SSA identity have separate guards.
    let absent = declarations::Declarations::default().with_type_environment(environment(&result));
    validate::check(plan.plan().clone(), absent).unwrap();
}

#[test]
fn internal_parameters_results_and_ssa_remain_bound_to_original_source() {
    let original = root(tuple(12), true);
    let plan = checked(&original);
    for ty in [tuple(13), vector(Type::OpaqueBytes, 12)] {
        assert_eq!(
            validate::check(rewrite(plan.plan(), ty), authority(&original))
                .unwrap_err()
                .detail,
            "root plan differs from declared signature"
        );
    }
    let mut arity = plan.plan().clone();
    arity.parameters.clear();
    assert_eq!(
        validate::check(arity, authority(&original))
            .unwrap_err()
            .detail,
        "root plan differs from declared signature"
    );
    let mut escape = plan.plan().clone();
    escape.body.result.id = usize::MAX;
    assert!(
        validate::check(escape, authority(&original))
            .unwrap_err()
            .detail
            .contains("out of scope")
    );
    let mut annotation = plan.plan().clone();
    annotation.body.result.ty = tuple(13);
    assert!(
        validate::check(annotation, authority(&original))
            .unwrap_err()
            .detail
            .contains("out of scope")
    );
    let mut two = original.clone();
    two.parameters.push(Parameter {
        name: "other".into(),
        ty: tuple(12),
    });
    let mut duplicate = checked(&two).plan().clone();
    duplicate.parameters[1].id = duplicate.parameters[0].id;
    assert_eq!(
        validate::check(duplicate, authority(&two))
            .unwrap_err()
            .detail,
        "unsupported or duplicate plan parameter"
    );
    let mut duplicate_name = two.clone();
    duplicate_name.parameters[1].name = "input".into();
    assert_eq!(
        validate::check(checked(&two).plan().clone(), authority(&duplicate_name))
            .unwrap_err()
            .detail,
        "duplicate root formal name"
    );
}

#[test]
fn internal_visibility_cannot_replace_canonical_type_definitions() {
    let original = root(named(tuple(2)), true);
    let forged = root(named(tuple(3)), true);
    let env = declarations::Declarations::default()
        .with_type_environment(environment(&original))
        .with_root(&forged);
    assert_eq!(
        validate::check(rewrite(checked(&original).plan(), forged.result), env)
            .unwrap_err()
            .detail,
        "struct definition is not canonical source type"
    );
    for max in [
        "01",
        "-1",
        "452312848583266388373324160190187140051835877600158453279131187530910662656",
    ] {
        let ty = validate::uint(max);
        assert!(matches!(
            check(&root(ty.clone(), true), &HashMap::new(), &HashMap::new()),
            Err(PlanError::Unsupported(_))
        ));
        assert!(
            validate::check(rewrite(checked(&original).plan(), ty), authority(&original)).is_err()
        );
    }
}

#[test]
fn internal_tuple_vectors_preserve_static_layout_budgets() {
    let limit = crate::resource_limits::recording_fixed_layout_limit();
    let accepted = root(vector(tuple(12), (limit - 1) / 13), true);
    render(&checked(&accepted)).unwrap();
    let oversized = vector(tuple(12), limit / 13 + 1);
    let Err(PlanError::Unsupported(gap)) = check(
        &root(oversized.clone(), true),
        &HashMap::new(),
        &HashMap::new(),
    ) else {
        panic!("oversized source must refuse")
    };
    let expected = format!("fixed aggregate layout exceeds {limit} slots");
    assert_eq!(gap.detail, expected);
    let original = root(Type::Field, true);
    assert_eq!(
        validate::check(
            rewrite(checked(&original).plan(), oversized),
            authority(&original)
        )
        .unwrap_err()
        .detail,
        expected
    );
}
