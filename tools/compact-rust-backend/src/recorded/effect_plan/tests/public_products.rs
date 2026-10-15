// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Public encoding policy, immutable signatures, and whole-old product controls.
use super::*;

#[path = "public_products/inclusion.rs"]
mod inclusion;

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
fn root(ty: Type) -> StatefulCircuit {
    StatefulCircuit {
        source: None,
        name: "echo".into(),
        internal: false,
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
fn checked(root: &StatefulCircuit) -> CheckedPlan {
    check(root, &HashMap::new(), &HashMap::new()).unwrap()
}
fn environment(root: &StatefulCircuit) -> type_environment::TypeEnvironment {
    type_environment::TypeEnvironment::from_sources(
        root,
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
    )
    .unwrap()
}
fn authority(root: &StatefulCircuit) -> declarations::Declarations {
    declarations::Declarations::default()
        .with_type_environment(environment(root))
        .with_root(root)
}
fn rewrite(plan: &Plan, ty: Type) -> Plan {
    let mut p = plan.clone();
    p.parameters[0].ty = ty.clone();
    p.body.result.ty = ty.clone();
    p.result_type = ty;
    p
}

#[test]
fn public_products_follow_official_encoding_and_named_derive_boundaries() {
    for ty in [
        tuple(0),
        tuple(1),
        tuple(9),
        tuple(11),
        vector(Type::OpaqueBytes, 0),
        vector(tuple(9), 2),
        vector(vector(Type::OpaqueString, 2), 3),
        named(tuple(8)),
        named(vector(tuple(8), 2)),
        Type::Struct {
            name: "Empty".into(),
            fields: vec![],
        },
        validate::uint("18446744073709551616"),
        validate::uint("340282366920938463463374607431768211455"),
        validate::uint("340282366920938463463374607431768211456"),
        validate::uint(
            "452312848583266388373324160190187140051835877600158453279131187530910662655",
        ),
    ] {
        assert!(validate::public_interface_type(&ty), "{ty:?}");
        let source = root(ty);
        let plan = checked(&source);
        validate::check(plan.plan().clone(), authority(&source)).unwrap();
        render(&plan).unwrap();
    }
    for ty in [
        tuple(12),
        vector(tuple(12), 0),
        named(tuple(9)),
        named(vector(tuple(9), 2)),
        Type::LedgerMap {
            key: Box::new(Type::Field),
            value: Box::new(Type::Field),
        },
    ] {
        assert!(!validate::public_interface_type(&ty), "{ty:?}");
        assert!(matches!(
            check(&root(ty), &HashMap::new(), &HashMap::new()),
            Err(PlanError::Unsupported(_))
        ));
    }
    // Witness outputs now share the bounded aligned representation; storage
    // retains its own narrower boundary, including tuple9 refusal.
    assert!(!validate::cell_type(&tuple(9)));
    assert!(validate::witness_result_type(&tuple(9)));
    assert!(!validate::witness_result_type(&tuple(12)));
    assert!(!validate::witness_result_type(&named(tuple(9))));
    assert!(validate::cell_type(&validate::uint(
        "340282366920938463463374607431768211456"
    )));
    assert!(validate::witness_result_type(&Type::OpaqueString));
}

#[test]
fn malformed_public_unsigned_bounds_are_refused_at_source_and_plan_boundaries() {
    let original = root(Type::Field);
    let plan = checked(&original);
    for bound in [
        "",
        "-1",
        "01",
        "abc",
        "452312848583266388373324160190187140051835877600158453279131187530910662656",
    ] {
        let ty = validate::uint(bound);
        assert!(!validate::public_interface_type(&ty), "{bound}");
        assert!(matches!(
            check(&root(ty.clone()), &HashMap::new(), &HashMap::new()),
            Err(PlanError::Unsupported(_))
        ));
        assert!(validate::check(rewrite(plan.plan(), ty), authority(&original)).is_err());
    }
}

#[test]
fn product_annotations_cannot_replace_immutable_root_signatures_or_scope() {
    for original in [root(tuple(2)), root(vector(Type::OpaqueBytes, 2))] {
        let plan = checked(&original);
        for forged_type in [
            tuple(3),
            vector(Type::OpaqueString, 2),
            vector(Type::OpaqueBytes, 3),
        ] {
            let gap = validate::check(rewrite(plan.plan(), forged_type), authority(&original))
                .unwrap_err();
            assert_eq!(gap.detail, "root plan differs from declared signature");
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
    }
    let original = root(named(vector(tuple(2), 2)));
    let plan = checked(&original);
    let forged_type = named(vector(tuple(3), 2));
    let forged_root = root(forged_type.clone());
    let retained_authority = declarations::Declarations::default()
        .with_type_environment(environment(&original))
        .with_root(&forged_root);
    assert_eq!(
        validate::check(rewrite(plan.plan(), forged_type), retained_authority)
            .unwrap_err()
            .detail,
        "struct definition is not canonical source type"
    );
}

#[test]
fn public_vector_layout_is_bounded_before_lowering_and_independent_validation() {
    let limit = crate::resource_limits::recording_fixed_layout_limit();
    let accepted = root(vector(vector(Type::Field, 2), (limit - 1) / 3));
    render(&checked(&accepted)).unwrap();
    let oversized = vector(vector(Type::Field, 2), limit / 3 + 1);
    let Err(PlanError::Unsupported(gap)) =
        check(&root(oversized.clone()), &HashMap::new(), &HashMap::new())
    else {
        panic!("source vector multiplicity must refuse")
    };
    let expected = format!("fixed aggregate layout exceeds {limit} slots");
    assert_eq!(gap.detail, expected);
    let original = root(Type::Field);
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
