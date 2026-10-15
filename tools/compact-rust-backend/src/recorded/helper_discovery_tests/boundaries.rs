// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;

#[test]
fn actual_unsupported_body_is_not_admitted_and_cannot_seed_final_helpers() {
    let mut c = fixture(
        Type::Field,
        StateReturn::Expression {
            value: crate::recorded::test_support::unsupported_fixed_input(),
        },
        true,
    );
    crate::recorded::test_support::register_unsupported_fold(&mut c);
    let output = crate::render_with_capabilities(&c).unwrap();
    assert!(
        !output
            .capabilities
            .circuits
            .iter()
            .find(|v| v.name == "root")
            .unwrap()
            .recorded
    );
    let (names, items) = Context::new(&c).plan(&c, false).unwrap();
    assert!(names.is_empty() && items.is_empty());
}

#[test]
fn native_missing_declarations_and_arity_errors_precede_helper_publication() {
    let mut c = fixture(Type::Unit, StateReturn::Unit, true);
    let StateAction::CircuitCall { name, .. } = &mut c.stateful_circuits[1].actions[0] else {
        unreachable!()
    };
    *name = "missing".into();
    assert!(crate::render_with_capabilities(&c).is_err());
    let mut c = fixture(Type::Unit, StateReturn::Unit, true);
    let StateAction::CircuitCall { arguments, .. } = &mut c.stateful_circuits[1].actions[0] else {
        unreachable!()
    };
    arguments.push(value(&Type::Field));
    assert!(crate::render_with_capabilities(&c).is_err());
}

#[test]
fn constructor_only_calls_do_not_seed_ordinary_recording_helpers() {
    let mut c = fixture(
        Type::Boolean,
        StateReturn::Expression {
            value: value(&Type::Boolean),
        },
        true,
    );
    c.stateful_circuits.pop();
    c.constructor = Some(crate::ir::Constructor {
        source: None,
        parameters: vec![],
        steps: vec![crate::ir::ConstructorStep::Expression {
            value: Expr::Call {
                name: "callee".into(),
                arguments: vec![],
            },
        }],
    });
    // Native validates the constructor call independently of ordinary roots.
    crate::render_with_capabilities(&c).unwrap();
    let (names, items) = Context::new(&c).plan(&c, false).unwrap();
    assert!(names.is_empty() && items.is_empty());
}
