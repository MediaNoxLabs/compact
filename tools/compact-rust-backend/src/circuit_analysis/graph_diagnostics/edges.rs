use super::*;
use crate::ir::{LocalBinding, Type};

#[test]
fn both_graphs_capture_actual_closing_action_before_argument_and_return_edges() {
    let mut root = circuit(
        "loop",
        vec![StateAction::CircuitCall {
            name: "loop".into(),
            arguments: vec![call("loop")],
        }],
    );
    root.return_value = StateReturn::Expression {
        value: call("loop"),
    };
    let sources = HashMap::from([("loop", &root)]);
    for kind in [Kind::Witness, Kind::Output] {
        let trace = Trace::default();
        assert_eq!(
            same(kind, &root, &sources, &HashSet::new(), &trace),
            Err(RenderError::UnsupportedStatefulCall("loop".into()))
        );
        action_site(&trace, &root.actions[0]);
    }
}

#[test]
fn equal_distinct_expr_edges_preserve_original_preorder_occurrence() {
    let root = circuit(
        "loop",
        vec![StateAction::Expression {
            value: Expr::Tuple {
                elements: vec![call("loop"), call("loop")],
            },
        }],
    );
    let StateAction::Expression {
        value: Expr::Tuple { elements },
    } = &root.actions[0]
    else {
        panic!("fixture")
    };
    assert_eq!(elements[0], elements[1]);
    assert!(!std::ptr::eq(&elements[0], &elements[1]));
    let sources = HashMap::from([("loop", &root)]);
    for kind in [Kind::Witness, Kind::Output] {
        let trace = Trace::default();
        assert!(same(kind, &root, &sources, &HashSet::new(), &trace).is_err());
        expr_site(&trace, &elements[0]);
    }
}

#[test]
fn return_plan_binding_precedes_result_without_considering_pure_action_name() {
    let mut root = circuit(
        "loop",
        vec![StateAction::PureCall {
            name: "loop".into(),
            arguments: vec![],
        }],
    );
    root.return_value = StateReturn::Effectful {
        body: ReturnPlan::Let {
            bindings: vec![LocalBinding {
                name: "x".into(),
                ty: Type::Unit,
                value: call("loop"),
            }],
            result: Box::new(ReturnPlan::Value {
                value: call("loop"),
            }),
        },
    };
    let StateReturn::Effectful {
        body: ReturnPlan::Let { bindings, .. },
    } = &root.return_value
    else {
        panic!("fixture")
    };
    let sources = HashMap::from([("loop", &root)]);
    for kind in [Kind::Witness, Kind::Output] {
        let trace = Trace::default();
        assert!(same(kind, &root, &sources, &HashSet::new(), &trace).is_err());
        expr_site(&trace, &bindings[0].value);
    }
}

#[test]
fn intrinsic_true_does_not_skip_cycles_and_success_has_no_events() {
    let cyclic = circuit(
        "loop",
        vec![
            StateAction::Expression {
                value: Expr::WitnessCall {
                    name: "w".into(),
                    arguments: vec![],
                },
            },
            StateAction::NativeWitnessCall {
                builtin: crate::ir::NativeWitnessBuiltin::OwnPublicKey,
            },
            action("loop"),
        ],
    );
    let sources = HashMap::from([("loop", &cyclic)]);
    for kind in [Kind::Witness, Kind::Output] {
        let trace = Trace::default();
        assert!(same(kind, &cyclic, &sources, &HashSet::new(), &trace).is_err());
        action_site(&trace, &cyclic.actions[2]);
    }
    let leaf = circuit(
        "leaf",
        vec![StateAction::NativeWitnessCall {
            builtin: crate::ir::NativeWitnessBuiltin::OwnPublicKey,
        }],
    );
    let root = circuit("root", vec![action("leaf")]);
    let sources = HashMap::from([("root", &root), ("leaf", &leaf)]);
    for (kind, expected) in [(Kind::Witness, false), (Kind::Output, true)] {
        let trace = Trace::default();
        assert_eq!(
            same(kind, &root, &sources, &HashSet::new(), &trace),
            Ok(expected)
        );
        assert!(trace.0.borrow().is_empty());
    }
    // Exercise retained oracle public helpers without adding production copies.
    assert_eq!(
        old_reference::expression_requires_witness(&call("leaf"), &sources).unwrap(),
        expression_requires_witness(&call("leaf"), &sources).unwrap()
    );
    assert_eq!(
        old_reference::expression_contains_stateful_call(&call("leaf"), &sources),
        expression_contains_stateful_call(&call("leaf"), &sources)
    );
}

#[test]
fn seeded_cycle_without_incoming_source_is_explicitly_unavailable() {
    let root = circuit("seeded", vec![]);
    for kind in [Kind::Witness, Kind::Output] {
        let trace = Trace::default();
        assert_eq!(
            same(
                kind,
                &root,
                &HashMap::new(),
                &HashSet::from(["seeded".into()]),
                &trace
            ),
            Err(RenderError::UnsupportedStatefulCall("seeded".into()))
        );
        assert_eq!(trace.0.borrow().as_slice(), &[(None, Slot::Node)]);
    }
}
