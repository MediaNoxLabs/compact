// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! CoPS030: old refusals are retained evidence, current failures are checked.
use super::*;

#[test]
fn action_and_return_refusals_publish_actual_fold_children_without_fallback() {
    for actionful in [false, true] {
        for result in [Type::Field, Type::Bytes { length: 32 }] {
            let mut c = root(
                result.clone(),
                StateReturn::Expression {
                    value: if result == Type::Field {
                        Expr::TransientHash {
                            value: Box::new(crate::recorded::test_support::unsupported_commit()),
                        }
                    } else {
                        crate::recorded::test_support::unsupported_commit()
                    },
                },
                if actionful {
                    vec![StateAction::Expression {
                        value: crate::recorded::test_support::unsupported_commit(),
                    }]
                } else {
                    vec![]
                },
            );
            crate::recorded::test_support::register_unsupported_fold(&mut c);
            let r = &c.stateful_circuits[0];
            crate::recorded::test_support::assert_checked_fold_refusal(
                &c,
                r,
                if actionful {
                    RecordingGap::action(&r.actions[0], "actions[0]".into())
                } else {
                    RecordingGap::returned(&r.return_value)
                },
                if actionful {
                    "actions[0].value.value"
                } else if result == Type::Field {
                    "return_value.value.value.value"
                } else {
                    "return_value.value.value"
                },
            );
            let (_, rows) = trace::capture_json(|| crate::render_with_capabilities(&c));
            let rows = rows.as_array().unwrap();
            assert!(rows.iter().all(|r| r["return_dispatch"].is_null()));
            assert!(rows.iter().all(|r| r["body_owner"].is_null()));
        }
    }
}

#[test]
fn effectful_prepass_publishes_concrete_refusal_before_internal_exclusion() {
    let mut c = unit(vec![]);
    let r = &mut c.stateful_circuits[0];
    r.internal = true;
    r.return_value = StateReturn::Effectful {
        body: crate::ir::ReturnPlan::Sequence {
            actions: vec![StateAction::Expression {
                value: crate::recorded::test_support::opaque_commit_policy(),
            }],
            result: Box::new(crate::ir::ReturnPlan::Value { value: Expr::Unit }),
        },
    };
    let cx = Context::new(&c);
    let result = render_recorded_circuit(
        &c.stateful_circuits[0],
        &cx.ledger,
        &cx.witnesses,
        &cx.pure,
        &cx.circuits,
        &HashSet::new(),
    )
    .unwrap();
    let RecordingOutcome::Unsupported(gap) = result else {
        panic!("expected policy refusal")
    };
    assert_eq!(gap.ir_node, "Expr::PersistentCommit");
    assert_eq!(gap.path, "return_value.body.actions[0].value");
    assert_eq!(gap.code, RecordingGapCode::UnsupportedExpression);
}
