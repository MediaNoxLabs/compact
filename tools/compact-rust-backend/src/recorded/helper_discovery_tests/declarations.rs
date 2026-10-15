// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;

#[test]
fn both_complete_old_eligible_forms_keep_exact_selected_names_and_items() {
    for (ty, returned) in [
        (Type::Unit, StateReturn::Unit),
        (
            Type::Field,
            StateReturn::Expression {
                value: value(&Type::Field),
            },
        ),
    ] {
        let c = fixture(ty, returned, true);
        crate::render_with_capabilities(&c).unwrap();
        let cx = Context::new(&c);
        let old = cx.plan(&c, true).unwrap();
        let current = cx.plan(&c, false).unwrap();
        assert_eq!(old.0, HashSet::from(["callee".into()]));
        assert_eq!(old.0, current.0);
        assert_eq!(
            old.1.iter().map(normalized).collect::<Vec<_>>(),
            current.1.iter().map(normalized).collect::<Vec<_>>()
        );
    }
}

#[test]
fn complete_checked_declarations_replace_result_and_return_shape_filters() {
    let product = Type::Struct {
        name: "Answer".into(),
        fields: vec![StructField {
            name: "field".into(),
            ty: Type::Field,
        }],
    };
    for ty in [
        Type::Unit,
        Type::Boolean,
        Type::Bytes { length: 32 },
        Type::JubjubPoint,
        Type::Tuple {
            elements: vec![Type::Field; 2],
        },
        product,
    ] {
        let c = fixture(
            ty.clone(),
            StateReturn::Expression { value: value(&ty) },
            true,
        );
        let output = crate::render_with_capabilities(&c).unwrap();
        assert!(
            output
                .capabilities
                .circuits
                .iter()
                .all(|v| v.recorded && v.observed_call)
        );
        let cx = Context::new(&c);
        assert!(
            cx.plan(&c, true).unwrap().0.is_empty(),
            "old shape unexpectedly allowed {ty:?}"
        );
        let (names, items) = cx.plan(&c, false).unwrap();
        assert_eq!(names, HashSet::from(["callee".into()]));
        assert_eq!(items.len(), 1);
        // Checked inlining leaves the internal optional definition unreferenced.
        assert!(!recorded_functions(&c).iter().any(|f| {
            f.sig
                .ident
                .to_string()
                .starts_with("__compact_recorded_body_")
        }));
    }
}

#[test]
fn actionful_field_public_callee_becomes_a_live_wrapper_without_new_interface() {
    let mut c = fixture(
        Type::Field,
        StateReturn::Expression {
            value: value(&Type::Field),
        },
        false,
    );
    c.ledger_fields.push(LedgerField {
        source: None,
        id: "cell".into(),
        index: 0,
        path: vec![],
        declaration: LedgerFieldKind::Cell { ty: Type::Field },
    });
    c.stateful_circuits[0].actions.push(StateAction::CellWrite {
        field: "cell".into(),
        index: 0,
        value: Expr::FieldLiteral { value: "1".into() },
    });
    let cx = Context::new(&c);
    assert!(cx.plan(&c, true).unwrap().0.is_empty());
    assert!(cx.plan(&c, false).unwrap().0.contains("callee"));
    let funcs = recorded_functions(&c);
    let body = funcs
        .iter()
        .find(|f| f.sig.ident == "__compact_recorded_body_callee")
        .unwrap();
    assert!(matches!(body.vis, syn::Visibility::Inherited));
    let public = funcs.iter().find(|f| f.sig.ident == "callee").unwrap();
    assert!(matches!(public.vis, syn::Visibility::Public(_)));
    assert!(
        public
            .to_token_stream()
            .to_string()
            .contains("__compact_recorded_body_callee (frame")
    );
}

#[test]
fn local_call_support_does_not_relabel_a_public_parameter_interface() {
    let wide = Type::Tuple {
        elements: vec![Type::Field; 12],
    };
    let mut c = fixture(Type::Unit, StateReturn::Unit, false);
    c.stateful_circuits[0].parameters.push(Parameter {
        name: "wide".into(),
        ty: wide.clone(),
    });
    let StateAction::CircuitCall { arguments, .. } = &mut c.stateful_circuits[1].actions[0] else {
        unreachable!()
    };
    *arguments = vec![Expr::Tuple {
        elements: vec![value(&Type::Field); 12],
    }];
    let output = crate::render_with_capabilities(&c).unwrap();
    assert!(
        !output
            .capabilities
            .circuits
            .iter()
            .find(|v| v.name == "callee")
            .unwrap()
            .recorded
    );
    assert!(
        output
            .capabilities
            .circuits
            .iter()
            .find(|v| v.name == "root")
            .unwrap()
            .recorded
    );
    assert!(
        !Context::new(&c)
            .plan(&c, false)
            .unwrap()
            .0
            .contains("callee")
    );
}
