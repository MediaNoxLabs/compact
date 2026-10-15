// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Forged calls bypass lowering to check the independent certificate boundary.
use super::super::pure_certification::{Certificates, Certifier};
use super::*;

fn identity(name: &str) -> PureCircuit {
    PureCircuit {
        source: None,
        name: name.into(),
        internal: true,
        parameters: vec![Parameter {
            name: "input".into(),
            ty: Type::Field,
        }],
        result: Type::Field,
        body: Expr::Parameter {
            name: "input".into(),
        },
    }
}

fn certificates(sources: &[PureCircuit], name: &str) -> Certificates {
    let pure = sources
        .iter()
        .map(|source| (source.name.as_str(), source))
        .collect();
    let stateful = HashMap::new();
    let mut certifier = Certifier::new(&pure, &stateful);
    certifier.certify(name, "test.certificate").unwrap();
    certifier.into_certificates()
}

fn environment(sources: &[PureCircuit], certificates: Certificates) -> declarations::Declarations {
    let pure = sources
        .iter()
        .map(|source| (source.name.as_str(), source))
        .collect();
    declarations::Declarations::with_calls(&HashMap::new(), &HashMap::new(), &pure, &HashMap::new())
        .with_certificates(certificates, &pure)
        .unwrap()
}

fn pure_binding(id: usize, argument: Value, result: Value) -> Binding {
    Binding {
        value: result,
        operation: Operation::PureCall {
            declaration: CallId(id),
            arguments: vec![argument],
        },
        path: "test.pure_call".into(),
    }
}

fn candidate() -> Plan {
    Plan {
        parameters: vec![field(0)],
        body: Block {
            bindings: vec![pure_binding(0, field(0), field(1))],
            result: field(1),
        },
        result_type: Type::Field,
    }
}

#[test]
fn pure_signature_without_matching_certificate_cannot_authorize_invocation() {
    let sources = [identity("a"), identity("b")];
    assert!(validate::check(candidate(), environment(&sources, Certificates::default())).is_err());
    assert!(
        validate::check(
            candidate(),
            environment(&sources, certificates(&sources, "b"))
        )
        .is_err()
    );
    validate::check(
        candidate(),
        environment(&sources, certificates(&sources, "a")),
    )
    .unwrap();
}

#[test]
fn stateful_identity_cannot_borrow_an_unrelated_pure_certificate() {
    let sources = [identity("z_pure")];
    let stateful = StatefulCircuit {
        source: None,
        name: "a_stateful".into(),
        internal: true,
        parameters: sources[0].parameters.clone(),
        result: Type::Field,
        actions: vec![],
        return_value: StateReturn::Expression {
            value: sources[0].body.clone(),
        },
    };
    let pure = HashMap::from([("z_pure", &sources[0])]);
    let env = declarations::Declarations::with_calls(
        &HashMap::new(),
        &HashMap::new(),
        &pure,
        &HashMap::from([("a_stateful", &stateful)]),
    )
    .with_certificates(certificates(&sources, "z_pure"), &pure)
    .unwrap();
    assert_eq!(env.resolve_call("a_stateful").unwrap().0, CallId(0));
    let error = validate::check(candidate(), env).unwrap_err();
    assert!(error.detail.contains("certificate"), "{error:?}");
}

#[test]
fn stale_same_signature_body_cannot_attach_to_current_declarations() {
    let original = [identity("identity")];
    let certified = certificates(&original, "identity");
    let mut changed = original.clone();
    changed[0].body = Expr::FieldLiteral { value: "2".into() };
    let pure = changed
        .iter()
        .map(|source| (source.name.as_str(), source))
        .collect();
    let declarations = declarations::Declarations::with_calls(
        &HashMap::new(),
        &HashMap::new(),
        &pure,
        &HashMap::new(),
    );
    let error = declarations
        .with_certificates(certified, &pure)
        .unwrap_err();
    assert!(error.detail.contains("differs"), "{error:?}");
}

#[test]
fn attachment_checks_transitive_identities_in_destination_registry() {
    let mut root = identity("a_root");
    root.body = Expr::Call {
        name: "b_leaf".into(),
        arguments: vec![Expr::Parameter {
            name: "input".into(),
        }],
    };
    let sources = [root, identity("b_leaf")];
    let pure = sources
        .iter()
        .map(|source| (source.name.as_str(), source))
        .collect();
    let stateful = StatefulCircuit {
        source: None,
        name: "b_leaf".into(),
        internal: true,
        parameters: sources[1].parameters.clone(),
        result: Type::Field,
        actions: vec![],
        return_value: StateReturn::Expression {
            value: sources[1].body.clone(),
        },
    };
    let ambiguous = declarations::Declarations::with_calls(
        &HashMap::new(),
        &HashMap::new(),
        &pure,
        &HashMap::from([("b_leaf", &stateful)]),
    );
    assert!(
        ambiguous
            .with_certificates(certificates(&sources, "a_root"), &pure)
            .is_err()
    );
    // Supplying complete source bodies is insufficient if the destination's
    // independent registry does not actually declare the transitive callee.
    let missing = declarations::Declarations::with_calls(
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::from([("a_root", &sources[0])]),
        &HashMap::new(),
    );
    assert!(
        missing
            .with_certificates(certificates(&sources, "a_root"), &pure)
            .is_err()
    );
    let mut changed_leaf = sources[1].clone();
    changed_leaf.parameters[0].ty = Type::Boolean;
    let mismatched = declarations::Declarations::with_calls(
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::from([("a_root", &sources[0]), ("b_leaf", &changed_leaf)]),
        &HashMap::new(),
    );
    let error = mismatched
        .with_certificates(certificates(&sources, "a_root"), &pure)
        .unwrap_err();
    assert!(error.detail.contains("signature"), "{error:?}");
}

#[test]
fn forged_identity_arity_type_result_and_scope_fail_independently() {
    let sources = [identity("identity")];
    let mut variants = Vec::new();
    let mut unknown = candidate();
    if let Operation::PureCall { declaration, .. } = &mut unknown.body.bindings[0].operation {
        *declaration = CallId(99);
    }
    variants.push(unknown);
    let mut arity = candidate();
    if let Operation::PureCall { arguments, .. } = &mut arity.body.bindings[0].operation {
        arguments.clear();
    }
    variants.push(arity);
    let mut unsigned = candidate();
    unsigned.parameters[0].ty = Type::Unsigned { max: "255".into() };
    if let Operation::PureCall { arguments, .. } = &mut unsigned.body.bindings[0].operation {
        arguments[0] = unsigned.parameters[0].clone();
    }
    variants.push(unsigned); // Field coercion must be an explicit preceding operation.
    let mut result = candidate();
    result.body.bindings[0].value.ty = Type::Boolean;
    result.body.result.ty = Type::Boolean;
    result.result_type = Type::Boolean;
    variants.push(result);
    let mut scope = candidate();
    if let Operation::PureCall { arguments, .. } = &mut scope.body.bindings[0].operation {
        arguments[0] = field(99);
    }
    variants.push(scope);
    for forged in variants {
        let error = validate::check(
            forged,
            environment(&sources, certificates(&sources, "identity")),
        )
        .unwrap_err();
        assert_eq!(error.path, "test.pure_call");
    }
}

#[test]
fn explicit_unsigned_to_field_conversion_is_accepted() {
    let sources = [identity("identity")];
    let input = Value {
        id: 0,
        ty: Type::Unsigned { max: "255".into() },
    };
    let plan = Plan {
        parameters: vec![input.clone()],
        body: Block {
            bindings: vec![
                Binding {
                    value: field(1),
                    operation: Operation::Convert {
                        value: input,
                        kind: Conversion::Coerce,
                    },
                    path: "test.coerce".into(),
                },
                pure_binding(0, field(1), field(2)),
            ],
            result: field(2),
        },
        result_type: Type::Field,
    };
    validate::check(
        plan,
        environment(&sources, certificates(&sources, "identity")),
    )
    .unwrap();
}

fn chain(length: usize) -> Vec<PureCircuit> {
    (0..length)
        .map(|index| {
            let mut source = identity(&format!("p{index:02}"));
            if index + 1 < length {
                source.body = Expr::Call {
                    name: format!("p{:02}", index + 1),
                    arguments: vec![source.body],
                };
            }
            source
        })
        .collect()
}

#[test]
fn validator_combines_bound_root_with_certified_pure_call_depth() {
    let limit = crate::resource_limits::recording_call_limits().0;
    let root = StatefulCircuit {
        source: None,
        name: "root".into(),
        internal: false,
        parameters: vec![Parameter {
            name: "input".into(),
            ty: Type::Field,
        }],
        result: Type::Field,
        actions: vec![],
        return_value: StateReturn::Expression {
            value: Expr::Parameter {
                name: "input".into(),
            },
        },
    };
    for length in [limit - 1, limit] {
        let sources = chain(length);
        let env = environment(&sources, certificates(&sources, "p00")).with_root(&root);
        let checked = validate::check(candidate(), env);
        if length < limit {
            checked.unwrap();
        } else {
            let error = checked.unwrap_err();
            assert!(
                error.detail.contains("combined") && error.detail.contains("depth"),
                "{error:?}"
            );
        }
    }
}

#[test]
fn unit_pure_call_is_retained_as_a_fallible_invocation() {
    use quote::ToTokens;
    let mut source = identity("assertion");
    source.parameters.clear();
    source.result = Type::Unit;
    source.body = Expr::Assert {
        condition: Box::new(Expr::Boolean { value: true }),
        message: "must execute".into(),
    };
    let sources = [source];
    let output = Value {
        id: 0,
        ty: Type::Unit,
    };
    let plan = Plan {
        parameters: vec![],
        body: Block {
            bindings: vec![Binding {
                value: output.clone(),
                operation: Operation::PureCall {
                    declaration: CallId(0),
                    arguments: vec![],
                },
                path: "test.unit".into(),
            }],
            result: output,
        },
        result_type: Type::Unit,
    };
    let checked = validate::check(
        plan,
        environment(&sources, certificates(&sources, "assertion")),
    )
    .unwrap();
    let rendered = render::render(&checked).unwrap();
    let statements = rendered
        .steps
        .iter()
        .map(ToTokens::to_token_stream)
        .map(|tokens| tokens.to_string())
        .collect::<Vec<_>>()
        .join(" ");
    assert_eq!(statements.matches("pure_circuits :: assertion").count(), 1);
    assert!(statements.contains("assertion () ?"), "{statements}");
}
