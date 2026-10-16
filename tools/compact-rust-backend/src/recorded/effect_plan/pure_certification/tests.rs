// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Native-pure certification controls, independent of legacy profile admission.
use super::*;
use crate::ir::{LocalBinding, Parameter, Type};

fn literal() -> Expr {
    Expr::FieldLiteral { value: "1".into() }
}
fn parameter(name: &str) -> Expr {
    Expr::Parameter { name: name.into() }
}
fn call(name: &str) -> Expr {
    Expr::Call {
        name: name.into(),
        arguments: vec![],
    }
}
fn source(name: &str, body: Expr) -> PureCircuit {
    PureCircuit {
        source: None,
        name: name.into(),
        internal: true,
        parameters: vec![],
        result: Type::Field,
        body,
    }
}
fn certify(sources: &[PureCircuit], root: &str) -> Result<Certificates, RecordingGap> {
    let pure = sources.iter().map(|c| (c.name.as_str(), c)).collect();
    let stateful = HashMap::new();
    let mut certifier = Certifier::new(&pure, &stateful);
    certifier.certify(root, "root.call")?;
    Ok(certifier.into_certificates())
}

#[test]
fn reachable_native_aggregate_hash_and_fold_internals_are_certified_once() {
    let hash = source(
        "hash",
        Expr::TransientHash {
            value: Box::new(Expr::Tuple {
                elements: vec![literal(), literal()],
            }),
        },
    );
    let mut sum = source(
        "sum",
        Expr::Add {
            left: Box::new(parameter("a")),
            right: Box::new(parameter("b")),
        },
    );
    sum.parameters = vec![
        Parameter {
            name: "a".into(),
            ty: Type::Field,
        },
        Parameter {
            name: "b".into(),
            ty: Type::Field,
        },
    ];
    let fold = source(
        "fold",
        Expr::VectorFoldCall {
            name: "sum".into(),
            initial: Box::new(call("hash")),
            source: Box::new(Expr::Vector {
                element: Type::Field,
                elements: vec![literal(), literal()],
            }),
            accumulator: Type::Field,
            element: Type::Field,
            length: 2,
        },
    );
    let root = source(
        "root",
        Expr::Add {
            left: Box::new(call("fold")),
            right: Box::new(call("fold")),
        },
    );
    let unused = source(
        "unused",
        Expr::CellRead {
            field: "missing".into(),
            index: 0,
        },
    );
    let sources = vec![hash, sum, fold, root, unused];
    let certificates = certify(&sources, "root").unwrap();
    assert_eq!(certificates.iter().count(), 4);
    assert!(certificates.get("unused").is_none());
    let root = certificates.get("root").unwrap();
    assert_eq!(root.call_depth(), 3);
    assert!(root.expanded_work() > certificates.get("fold").unwrap().expanded_work() * 2);
}

#[test]
fn transitive_hidden_effects_are_checked_in_unused_bindings_and_dead_branches() {
    let bad = source(
        "bad",
        Expr::Let {
            bindings: vec![LocalBinding {
                name: "unused".into(),
                ty: Type::Field,
                value: Expr::CellRead {
                    field: "hidden".into(),
                    index: 0,
                },
            }],
            body: Box::new(literal()),
        },
    );
    let root = source(
        "root",
        Expr::If {
            condition: Box::new(Expr::Boolean { value: false }),
            then: Box::new(call("bad")),
            otherwise: Box::new(literal()),
        },
    );
    let error = certify(&[root, bad], "root").unwrap_err();
    assert!(error.path.contains("callee[bad]"), "{error:?}");
    assert!(
        error
            .detail
            .contains("stateful expression requires stateful evaluation"),
        "{error:?}"
    );
}

#[test]
fn fresh_formals_and_exact_native_types_reject_capture_and_malformed_declarations() {
    let bad = source("bad", parameter("caller_only"));
    let root = source(
        "root",
        Expr::Let {
            bindings: vec![LocalBinding {
                name: "caller_only".into(),
                ty: Type::Field,
                value: literal(),
            }],
            body: Box::new(call("bad")),
        },
    );
    assert!(certify(&[root, bad], "root").is_err());
    let mut bad = source("bad", parameter("x"));
    bad.parameters = vec![
        Parameter {
            name: "x".into(),
            ty: Type::Field
        };
        2
    ];
    assert!(certify(&[bad], "bad").is_err());
    let mut wrong_result = source("bad", literal());
    wrong_result.result = Type::Boolean;
    assert!(certify(&[wrong_result], "bad").is_err());
    let mut callee = source("callee", parameter("x"));
    callee.parameters = vec![Parameter {
        name: "x".into(),
        ty: Type::Field,
    }];
    let root = source(
        "root",
        Expr::Call {
            name: "callee".into(),
            arguments: vec![Expr::Boolean { value: true }],
        },
    );
    assert!(certify(&[root, callee], "root").is_err());
}

#[test]
fn call_and_fold_cycles_missing_names_and_ambiguous_kinds_fail() {
    assert!(certify(&[source("self", call("self"))], "self").is_err());
    assert!(certify(&[source("a", call("b")), source("b", call("a"))], "a").is_err());
    assert!(certify(&[source("a", call("missing"))], "a").is_err());
    let mut fold = source(
        "fold",
        Expr::VectorFoldCall {
            name: "fold".into(),
            initial: Box::new(parameter("a")),
            source: Box::new(Expr::Vector {
                element: Type::Field,
                elements: vec![parameter("b")],
            }),
            accumulator: Type::Field,
            element: Type::Field,
            length: 1,
        },
    );
    fold.parameters = vec![
        Parameter {
            name: "a".into(),
            ty: Type::Field,
        },
        Parameter {
            name: "b".into(),
            ty: Type::Field,
        },
    ];
    assert!(
        certify(&[fold], "fold")
            .unwrap_err()
            .detail
            .contains("recursive")
    );
    let pure_source = source("same", literal());
    let state_source = StatefulCircuit {
        source: None,
        name: "same".into(),
        internal: true,
        parameters: vec![],
        result: Type::Field,
        actions: vec![],
        return_value: crate::ir::StateReturn::Expression { value: literal() },
    };
    let pure = HashMap::from([("same", &pure_source)]);
    let stateful = HashMap::from([("same", &state_source)]);
    assert!(
        Certifier::new(&pure, &stateful)
            .certify("same", "root")
            .is_err()
    );
}

#[test]
fn certificates_bind_exact_transitive_source_not_only_signatures() {
    let mut sources = vec![source("root", call("leaf")), source("leaf", literal())];
    let certificates = certify(&sources, "root").unwrap();
    fn bindings(sources: &[PureCircuit]) -> HashMap<&str, &PureCircuit> {
        sources.iter().map(|c| (c.name.as_str(), c)).collect()
    }
    certificates.verify_bindings(&bindings(&sources)).unwrap();
    sources[1].body = Expr::FieldLiteral { value: "2".into() };
    let error = certificates
        .verify_bindings(&bindings(&sources))
        .unwrap_err();
    assert!(error.detail.contains("differs"));
    assert!(certificates.verify_bindings(&HashMap::new()).is_err());
}

fn chain(length: usize, diamond: bool) -> Vec<PureCircuit> {
    (0..length)
        .map(|i| {
            let next = || call(&format!("c{}", i + 1));
            let body = if i + 1 == length {
                literal()
            } else if diamond {
                Expr::Add {
                    left: Box::new(next()),
                    right: Box::new(next()),
                }
            } else {
                next()
            };
            source(&format!("c{i}"), body)
        })
        .collect()
}

#[test]
fn cached_dependency_depth_and_expanded_diamond_work_remain_bounded() {
    let (limit, _) = crate::resource_limits::recording_call_limits();
    let sources = chain(limit + 1, false);
    let pure = sources.iter().map(|c| (c.name.as_str(), c)).collect();
    let stateful = HashMap::new();
    let mut certifier = Certifier::new(&pure, &stateful);
    certifier.certify("c1", "first").unwrap();
    assert_eq!(certifier.get("c1").unwrap().call_depth(), limit);
    assert!(
        certifier
            .certify("c0", "second")
            .unwrap_err()
            .detail
            .contains("depth")
    );
    let error = certify(&chain(limit, true), "c0").unwrap_err();
    assert!(error.detail.contains("work"), "{error:?}");
}

#[test]
fn resource_scan_precedes_recursive_typing_clone_and_binding_comparison() {
    let mut deep = literal();
    for _ in 0..80 {
        deep = Expr::Add {
            left: Box::new(deep),
            right: Box::new(literal()),
        };
    }
    let error = certify(&[source("deep", deep.clone())], "deep").unwrap_err();
    assert!(error.detail.contains("syntax_depth"), "{error:?}");
    let certificates = certify(&[source("deep", literal())], "deep").unwrap();
    let changed = source("deep", deep);
    let error = certificates
        .verify_bindings(&HashMap::from([("deep", &changed)]))
        .unwrap_err();
    assert!(error.detail.contains("syntax_depth"), "{error:?}");
}

#[test]
fn source_alias_keys_cannot_hide_ambiguous_declaration_kinds_or_names() {
    let source = source("same", literal());
    let duplicate = source.clone();
    let pure = HashMap::from([("same", &source), ("alias", &duplicate)]);
    let stateful = HashMap::new();
    assert!(
        Certifier::new(&pure, &stateful)
            .certify("same", "root")
            .is_err()
    );

    let valid = HashMap::from([("same", &source)]);
    let mut certifier = Certifier::new(&valid, &stateful);
    certifier.certify("same", "root").unwrap();
    let certificates = certifier.into_certificates();
    assert!(certificates.verify_bindings(&pure).is_err());

    let state = StatefulCircuit {
        source: None,
        name: "same".into(),
        internal: true,
        parameters: vec![],
        result: Type::Field,
        actions: vec![],
        return_value: crate::ir::StateReturn::Expression { value: literal() },
    };
    let stateful = HashMap::from([("state_alias", &state)]);
    assert!(
        Certifier::new(&valid, &stateful)
            .certify("same", "root")
            .is_err()
    );
}

#[test]
fn callee_signature_is_resource_checked_before_native_caller_typing() {
    let mut leaf = source("leaf", literal());
    let mut result = Type::Field;
    for _ in 0..80 {
        result = Type::Vector {
            element: Box::new(result),
            length: 1,
        };
    }
    leaf.result = result;
    // The native caller renderer would clone the declared result before it
    // checked this callee's body. DFS completion must preflight the callee first.
    let root = source("root", call("leaf"));
    let error = certify(&[root, leaf], "root").unwrap_err();
    assert!(error.detail.contains("syntax_depth"), "{error:?}");
    assert!(error.path.contains("callee[leaf]"), "{error:?}");
}

#[test]
fn pure_defaults_require_tuple_default_traits_without_restricting_constructed_values() {
    let tuple = |length| Type::Tuple {
        elements: vec![Type::Field; length],
    };
    for (ty, allowed) in [
        (tuple(12), true),
        (tuple(13), false),
        (
            Type::Vector {
                element: Box::new(tuple(13)),
                length: 0,
            },
            false,
        ),
        (
            Type::Vector {
                element: Box::new(tuple(13)),
                length: 2,
            },
            false,
        ),
        (
            Type::Tuple {
                elements: vec![tuple(13)],
            },
            false,
        ),
    ] {
        let mut helper = source("defaulted", Expr::Default { ty: ty.clone() });
        helper.result = ty;
        let result = certify(&[helper], "defaulted");
        if allowed {
            result.unwrap();
        } else {
            assert!(
                result
                    .unwrap_err()
                    .detail
                    .contains("pure Default requires tuple arity")
            );
        }
    }
    let mut constructed = source(
        "constructed",
        Expr::Tuple {
            elements: vec![literal(); 13],
        },
    );
    constructed.result = tuple(13);
    certify(&[constructed], "constructed").unwrap();

    let hidden = source(
        "hidden",
        Expr::Let {
            bindings: vec![LocalBinding {
                name: "unused".into(),
                ty: tuple(13),
                value: Expr::Default { ty: tuple(13) },
            }],
            body: Box::new(literal()),
        },
    );
    let caller = source("caller", call("hidden"));
    let error = certify(&[caller, hidden], "caller").unwrap_err();
    assert!(error.detail.contains("pure Default requires tuple arity"));
}

#[test]
fn hidden_named_structs_require_representation_derives_even_without_defaults() {
    for length in [8, 9] {
        let ty = Type::Struct {
            name: "Hidden".into(),
            fields: vec![crate::ir::StructField {
                name: "items".into(),
                ty: Type::Tuple {
                    elements: vec![Type::Field; length],
                },
            }],
        };
        for value in [
            Expr::Default { ty: ty.clone() },
            Expr::StructLiteral {
                ty: ty.clone(),
                fields: vec![Expr::Tuple {
                    elements: vec![literal(); length],
                }],
            },
        ] {
            let helper = source(
                "hidden",
                Expr::Let {
                    bindings: vec![LocalBinding {
                        name: "unused".into(),
                        ty: ty.clone(),
                        value,
                    }],
                    body: Box::new(literal()),
                },
            );
            let caller = source("caller", call("hidden"));
            let result = certify(&[caller, helper], "caller");
            if length == 8 {
                result.unwrap();
            } else {
                assert!(
                    result
                        .unwrap_err()
                        .detail
                        .contains("pure named Struct requires supported representation derives")
                );
            }
        }
    }
    let mut plain_tuple = source(
        "plain",
        Expr::Tuple {
            elements: vec![literal(); 9],
        },
    );
    plain_tuple.result = Type::Tuple {
        elements: vec![Type::Field; 9],
    };
    certify(&[plain_tuple], "plain").unwrap();
}
