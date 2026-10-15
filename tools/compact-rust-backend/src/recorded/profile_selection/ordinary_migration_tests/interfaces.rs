// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;

#[test]
fn ordinary_declared_interfaces_reuse_canonical_public_value_authority() {
    let mut types = vec![
        Type::Unit,
        Type::Boolean,
        Type::Field,
        Type::OpaqueString,
        Type::OpaqueBytes,
        Type::Bytes { length: 32 },
        Type::JubjubPoint,
        Type::Unsigned {
            max: u128::MAX.to_string(),
        },
        Type::Unsigned {
            max: "680564733841876926926749214863536422911".into(),
        },
        Type::Enum {
            name: "Choice".into(),
            variants: vec!["Yes".into(), "No".into()],
        },
        Type::Vector {
            element: Box::new(Type::OpaqueBytes),
            length: 2,
        },
        Type::Struct {
            name: "Owned".into(),
            fields: vec![StructField {
                name: "text".into(),
                ty: Type::OpaqueString,
            }],
        },
    ];
    for length in [0, 9, 11] {
        types.push(Type::Tuple {
            elements: vec![Type::Field; length],
        });
    }
    for ty in types {
        let c = identity(ty.clone());
        assert!(ordinary(&c, "identity").unwrap().is_some(), "{ty:?}");
        // Full generation independently validates named definitions and native signatures.
        public(&c, "identity");
    }
}

#[test]
fn ordinary_effectless_parameter_binding_keeps_typed_result_and_both_public_apis() {
    let c = identity(Type::OpaqueBytes);
    let plan = ordinary(&c, "identity").unwrap().unwrap();
    // Rendering introduces a typed parameter binding even without effects.
    // Do not confuse absence of runtime operations with zero Rust statements.
    assert_eq!(plan.steps.len(), 1);
    assert!(matches!(&plan.steps[0], syn::Stmt::Local(local)
        if matches!(local.pat, syn::Pat::Type(_))));
    assert!(methods(&plan).is_empty());
    assert!(!matches!(plan.result, syn::Expr::Tuple(ref t) if t.elems.is_empty()));
    let (source, row) = public(&c, "identity");
    assert_eq!(row["body_owner"], "algebra.ordinary");
    let parsed = syn::parse_file(&source).unwrap();
    fn recorded_identity(items: &[syn::Item]) -> bool {
        items.iter().any(|item| match item {
            syn::Item::Mod(module) => module.content.as_ref().is_some_and(|(_, items)| {
                (module.ident == "recorded"
                    && items.iter().any(|item| {
                        matches!(item,
                    syn::Item::Fn(function) if function.sig.ident == "identity")
                    }))
                    || recorded_identity(items)
            }),
            _ => false,
        })
    }
    assert!(recorded_identity(&parsed.items));
}

#[test]
fn ordinary_interfaces_preserve_tuple_derive_ledger_and_budget_boundaries() {
    let tuple9 = Type::Tuple {
        elements: vec![Type::Field; 9],
    };
    let invalid = [
        Type::Tuple {
            elements: vec![Type::Field; 12],
        },
        Type::Struct {
            name: "TooWide".into(),
            fields: vec![StructField {
                name: "value".into(),
                ty: tuple9,
            }],
        },
        Type::LedgerMap {
            key: Box::new(Type::Field),
            value: Box::new(Type::Field),
        },
        Type::Vector {
            element: Box::new(Type::Tuple {
                elements: vec![Type::Field; 12],
            }),
            length: 0,
        },
        Type::Vector {
            element: Box::new(Type::Field),
            length: 32769,
        },
    ];
    for ty in invalid {
        assert!(
            ordinary(&identity(ty.clone()), "identity")
                .unwrap()
                .is_none(),
            "{ty:?}"
        );
    }
    let mut invalid = identity(Type::Field);
    invalid.stateful_circuits[0].return_value = StateReturn::Expression {
        value: Expr::Parameter {
            name: "undeclared".into(),
        },
    };
    assert!(ordinary(&invalid, "identity").unwrap().is_none());
    assert!(
        crate::render_with_capabilities(&invalid).is_err(),
        "native source authority still runs first"
    );
}
