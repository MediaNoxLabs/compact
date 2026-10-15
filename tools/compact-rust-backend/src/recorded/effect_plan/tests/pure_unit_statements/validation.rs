// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Statement-specific controls; shared certificate tests cover the complete auditor.
use super::*;

fn reference(name: &str) -> Expr {
    Expr::Parameter { name: name.into() }
}
fn guard() -> PureCircuit {
    PureCircuit {
        source: None,
        name: "guard".into(),
        internal: true,
        parameters: vec![Parameter {
            name: "condition".into(),
            ty: Type::Boolean,
        }],
        result: Type::Unit,
        body: Expr::Assert {
            condition: Box::new(reference("condition")),
            message: "guard failed".into(),
        },
    }
}
fn root() -> StatefulCircuit {
    StatefulCircuit {
        source: None,
        name: "run".into(),
        internal: false,
        parameters: vec![Parameter {
            name: "selected".into(),
            ty: Type::Boolean,
        }],
        result: Type::Unit,
        actions: vec![StateAction::PureCall {
            name: "guard".into(),
            arguments: vec![reference("selected")],
        }],
        return_value: StateReturn::Unit,
    }
}
fn candidate(
    root: &StatefulCircuit,
    pure: &[PureCircuit],
    stateful: &[StatefulCircuit],
) -> Result<CheckedPlan, PlanError> {
    let pure = pure.iter().map(|v| (v.name.as_str(), v)).collect();
    let stateful = stateful.iter().map(|v| (v.name.as_str(), v)).collect();
    check_with_calls(root, &HashMap::new(), &HashMap::new(), &pure, &stateful)
}
fn unsupported(result: Result<CheckedPlan, PlanError>) -> RecordingGap {
    match result {
        Err(PlanError::Unsupported(gap)) => gap,
        other => panic!("expected source Unsupported, got {other:?}"),
    }
}

#[test]
fn pure_statements_require_unique_pure_unit_declarations() {
    let root = root();
    let pure = guard();
    candidate(&root, std::slice::from_ref(&pure), &[]).unwrap();
    let missing = unsupported(candidate(&root, &[], &[]));
    assert_eq!(missing.ir_node, "StateAction::PureCall");
    let mut stateful = root.clone();
    stateful.name = "guard".into();
    stateful.actions.clear();
    let wrong_kind = unsupported(candidate(&root, &[], std::slice::from_ref(&stateful)));
    assert!(
        wrong_kind.detail.contains("pure Unit declaration"),
        "{wrong_kind:?}"
    );
    unsupported(candidate(&root, std::slice::from_ref(&pure), &[stateful]));
    let mut non_unit = pure;
    non_unit.result = Type::Boolean;
    non_unit.body = reference("condition");
    let non_unit = unsupported(candidate(&root, &[non_unit], &[]));
    assert!(
        non_unit.detail.contains("pure Unit declaration"),
        "{non_unit:?}"
    );
}

#[test]
fn statement_arguments_preserve_exact_signature_and_source_scope() {
    for (arguments, detail) in [
        (vec![], "argument count"),
        (
            vec![Expr::FieldLiteral { value: "1".into() }],
            "argument type",
        ),
        (vec![reference("escaped")], "unbound value"),
    ] {
        let mut root = root();
        root.actions[0] = StateAction::PureCall {
            name: "guard".into(),
            arguments,
        };
        let error = unsupported(candidate(&root, &[guard()], &[]));
        assert!(error.detail.to_lowercase().contains(detail), "{error:?}");
    }
}

#[test]
fn a_pure_statement_cannot_relax_the_stateful_call_kind_boundary() {
    let mut root = root();
    root.actions[0] = StateAction::CircuitCall {
        name: "guard".into(),
        arguments: vec![reference("selected")],
    };
    unsupported(candidate(&root, &[guard()], &[]));
}

#[test]
fn lowered_unit_statements_revalidate_against_original_immutable_authority() {
    let checked = candidate(&root(), &[guard()], &[]).unwrap();
    let original = checked.plan();
    validate::validate(original, checked.declarations()).unwrap();
    let index = original
        .body
        .bindings
        .iter()
        .position(|b| matches!(b.operation, Operation::PureCall { .. }))
        .unwrap();
    for change in 0..3 {
        let mut forged = original.clone();
        let binding = &mut forged.body.bindings[index];
        let Operation::PureCall {
            declaration,
            arguments,
        } = &mut binding.operation
        else {
            unreachable!()
        };
        let expected = match change {
            0 => {
                *declaration = CallId(usize::MAX);
                "unknown pure call declaration"
            }
            1 => {
                arguments[0].id = usize::MAX;
                "scope"
            }
            _ => {
                binding.value.ty = Type::Boolean;
                "certified signature"
            }
        };
        let error = validate::validate(&forged, checked.declarations()).unwrap_err();
        assert!(error.detail.contains(expected), "{change}: {error:?}");
    }
}
