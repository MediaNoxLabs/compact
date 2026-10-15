// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;

#[test]
fn native_control_and_unit_views_match_frozen_complete_family_outcomes() {
    let unit = Type::Unit;
    let flag = Type::Boolean;
    let params = HashMap::from([
        ("unit", (&unit, syn::parse_quote!(unit))),
        ("flag", (&flag, syn::parse_quote!(flag))),
        (
            "__compact_condition",
            (&flag, syn::parse_quote!(__compact_condition)),
        ),
    ]);
    let mark = pure("mark", Type::Unit);
    let probe = pure("probe", Type::Boolean);
    let calls = HashMap::from([("mark", &mark), ("probe", &probe)]);
    let point = Type::Struct {
        name: "UnitHolder".into(),
        fields: vec![StructField {
            name: "member".into(),
            ty: Type::Unit,
        }],
    };
    let field = Expr::StructField {
        value: Box::new(Expr::StructLiteral {
            ty: point,
            fields: vec![Expr::Unit],
        }),
        field: "member".into(),
        index: 0,
    };
    let lifted = || {
        local(
            "saved",
            Type::Boolean,
            reference("flag"),
            reference("saved"),
        )
    };
    let cases = vec![
        Expr::Unit,
        Expr::Default { ty: Type::Unit },
        reference("unit"),
        call("mark"),
        field,
        Expr::TupleIndex {
            value: Box::new(Expr::Tuple {
                elements: vec![Expr::Unit, boolean(true)],
            }),
            index: 0,
        },
        assertion(reference("flag")),
        assertion(lifted()),
        branch(reference("flag"), Expr::Unit, Expr::Unit),
        branch(call("probe"), call("mark"), call("mark")),
        branch(call("probe"), Expr::Default { ty: Type::Unit }, Expr::Unit),
        branch(reference("flag"), assertion(reference("flag")), Expr::Unit),
        branch(reference("flag"), Expr::Unit, assertion(reference("flag"))),
        branch(
            reference("flag"),
            call("mark"),
            assertion(reference("flag")),
        ),
        branch(
            reference("flag"),
            branch(reference("flag"), assertion(reference("flag")), Expr::Unit),
            Expr::Unit,
        ),
        branch(lifted(), Expr::Unit, call("mark")),
        branch(
            local(
                "saved",
                Type::Boolean,
                reference("__compact_condition"),
                reference("saved"),
            ),
            Expr::Unit,
            Expr::Unit,
        ),
        Expr::Let {
            bindings: Vec::new(),
            body: Box::new(Expr::Unit),
        },
        local("saved", Type::Unit, call("mark"), reference("saved")),
        Expr::Let {
            bindings: vec![
                LocalBinding {
                    name: "flag".into(),
                    ty: Type::Boolean,
                    value: reference("flag"),
                },
                LocalBinding {
                    name: "flag".into(),
                    ty: Type::Boolean,
                    value: boolean(false),
                },
            ],
            body: Box::new(branch(reference("flag"), call("mark"), Expr::Unit)),
        },
        Expr::Sequence {
            steps: vec![assertion(boolean(true)), call("mark")],
            value: Box::new(Expr::Unit),
        },
        Expr::Sequence {
            steps: vec![call("mark")],
            value: Box::new(Expr::FieldLiteral { value: "7".into() }),
        },
        branch(call("probe"), boolean(true), boolean(true)),
        branch(reference("flag"), boolean(true), boolean(false)),
        assertion(Expr::FieldLiteral { value: "0".into() }),
        Expr::Sequence {
            steps: vec![Expr::FieldLiteral { value: "0".into() }],
            value: Box::new(reference("missing")),
        },
        branch(
            boolean(true),
            Expr::FieldLiteral { value: "0".into() },
            reference("missing"),
        ),
        branch(
            reference("unit"),
            reference("missing"),
            reference("also_missing"),
        ),
        local(
            "not a Rust name",
            Type::Unit,
            reference("missing"),
            Expr::Unit,
        ),
        local(
            "saved",
            Type::Unsigned { max: "bad".into() },
            Expr::Unit,
            Expr::Unit,
        ),
        branch(boolean(true), boolean(true), Expr::Unit),
    ];
    for (index, source) in cases.iter().enumerate() {
        assert_eq!(
            observe(source, &params, &calls),
            old_oracle::observe(source, &params, &calls),
            "case {index}"
        );
    }
}

#[test]
fn nonunit_branch_does_not_request_unit_projection_before_later_child_failure() {
    let source = branch(
        boolean(true),
        Expr::FieldLiteral { value: "1".into() },
        reference("missing"),
    );
    assert_eq!(
        observe(&source, &HashMap::new(), &HashMap::new()),
        Err(RenderError::UnknownParameter("missing".into()))
    );
}

#[test]
fn pure_materialized_unit_views_keep_their_statement_policy() {
    let unit = Type::Unit;
    let params = HashMap::from([("unit", (&unit, syn::parse_quote!(unit)))]);
    let parameter = observe(&reference("unit"), &params, &HashMap::new()).unwrap();
    assert_eq!(parameter.1, "unit ;");
    let default = observe(&Expr::Default { ty: Type::Unit }, &params, &HashMap::new()).unwrap();
    assert!(!default.0.is_empty());
    assert!(default.1.is_empty());
}
