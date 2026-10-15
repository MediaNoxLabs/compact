use super::*;

#[test]
fn snapshot_failures_are_unavailable_while_original_second_validation_keeps_actual_type() {
    for result_failure in [true, false] {
        let bad = Type::Unsigned { max: "bad".into() };
        let snapshot = circuit(
            "callee",
            if result_failure {
                vec![]
            } else {
                vec![parameter("x", bad.clone())]
            },
            field(),
            if result_failure {
                bad.clone()
            } else {
                Type::Field
            },
        );
        let original = circuit(
            "callee",
            if result_failure {
                vec![]
            } else {
                vec![parameter("x", Type::Field)]
            },
            field(),
            Type::Field,
        );
        let source = call("callee", vec![]);
        let trace = Trace::default();
        let outcome = direct_error(
            CallSite::Expression(&source),
            "callee",
            &[],
            &registry(std::slice::from_ref(&snapshot)),
            &registry(std::slice::from_ref(&original)),
            &trace,
            Seed::Ordinary,
        );
        assert!(outcome.0.as_ref().unwrap_err().detail.contains("bad"));
        primary(&trace, RefusalSite::unavailable());
        assert!(trace.events.borrow().iter().all(|event| event.0.is_none()));
        assert_eq!((outcome.1, outcome.2), (11, 0));

        let snapshot = circuit(
            "callee",
            if result_failure {
                vec![]
            } else {
                vec![parameter("x", Type::Field)]
            },
            field(),
            Type::Field,
        );
        let original = circuit(
            "callee",
            if result_failure {
                vec![]
            } else {
                vec![parameter("x", bad.clone())]
            },
            field(),
            if result_failure { bad } else { Type::Field },
        );
        let trace = Trace::default();
        let _outcome = direct_error(
            CallSite::Expression(&source),
            "callee",
            &[],
            &registry(std::slice::from_ref(&snapshot)),
            &registry(std::slice::from_ref(&original)),
            &trace,
            Seed::Ordinary,
        );
        let ty = if result_failure {
            &original.result
        } else {
            &original.parameters[0].ty
        };
        primary(&trace, RefusalSite::field(Owner::Type(ty), "max"));
    }

    let limit = crate::resource_limits::recording_byte_payload_limit();
    let original = circuit(
        "callee",
        vec![],
        field(),
        Type::Vector {
            element: Box::new(Type::Bytes { length: limit + 1 }),
            length: 1,
        },
    );
    let snapshot = circuit("callee", vec![], field(), Type::Field);
    let source = call("callee", vec![]);
    let trace = Trace::default();
    let _outcome = direct_error(
        CallSite::Expression(&source),
        "callee",
        &[],
        &registry(std::slice::from_ref(&snapshot)),
        &registry(std::slice::from_ref(&original)),
        &trace,
        Seed::Ordinary,
    );
    let Type::Vector { element, .. } = &original.result else {
        panic!("fixture")
    };
    primary(&trace, RefusalSite::node(Owner::Type(element)));
    assert!(
        trace.events.borrow().iter().all(|event| event.0.is_some()),
        "real nested layout first site survives same-root fallback"
    );
}

#[test]
fn result_before_parameters_and_snapshot_before_original_signature_relation_are_exact() {
    let snapshot = circuit(
        "callee",
        vec![parameter(
            "x",
            Type::Unsigned {
                max: "parameter_bad".into(),
            },
        )],
        field(),
        Type::Field,
    );
    let original = circuit(
        "callee",
        vec![parameter("x", Type::Field)],
        field(),
        Type::Unsigned {
            max: "result_bad".into(),
        },
    );
    let source = call("callee", vec![]);
    let trace = Trace::default();
    let outcome = direct_error(
        CallSite::Expression(&source),
        "callee",
        &[],
        &registry(std::slice::from_ref(&snapshot)),
        &registry(std::slice::from_ref(&original)),
        &trace,
        Seed::Ordinary,
    );
    detail(&outcome, "result_bad");
    primary(
        &trace,
        RefusalSite::field(Owner::Type(&original.result), "max"),
    );

    let snapshot = circuit(
        "callee",
        vec![parameter("a", Type::Field)],
        field(),
        Type::Field,
    );
    let original = circuit(
        "callee",
        vec![parameter("b", Type::Field)],
        field(),
        Type::Field,
    );
    let trace = Trace::default();
    let outcome = direct_error(
        CallSite::Expression(&source),
        "callee",
        &[],
        &registry(std::slice::from_ref(&snapshot)),
        &registry(std::slice::from_ref(&original)),
        &trace,
        Seed::Ordinary,
    );
    detail(&outcome, "signature differ");
    primary(&trace, RefusalSite::unavailable());
    assert_eq!((outcome.1, outcome.2), (11, 0));

    let snapshot = circuit(
        "callee",
        vec![
            parameter("a", Type::Field),
            parameter(
                "b",
                Type::Unsigned {
                    max: "snapshot_second".into(),
                },
            ),
        ],
        field(),
        Type::Field,
    );
    let original = circuit(
        "callee",
        vec![
            parameter(
                "a",
                Type::Unsigned {
                    max: "original_first".into(),
                },
            ),
            parameter("b", Type::Field),
        ],
        field(),
        Type::Field,
    );
    let trace = Trace::default();
    let outcome = direct_error(
        CallSite::Expression(&source),
        "callee",
        &[],
        &registry(std::slice::from_ref(&snapshot)),
        &registry(std::slice::from_ref(&original)),
        &trace,
        Seed::Ordinary,
    );
    detail(&outcome, "snapshot_second");
    primary(&trace, RefusalSite::unavailable());
}

#[test]
fn whole_original_callee_marks_unsupported_composite_without_member_guess() {
    let callee = circuit(
        "callee",
        vec![parameter("input", named_unsupported())],
        field(),
        Type::Field,
    );
    let root = circuit(
        "root",
        vec![],
        call(
            "callee",
            vec![Expr::Parameter {
                name: "not_evaluated".into(),
            }],
        ),
        Type::Field,
    );
    let sources = HashMap::from([("root", &root), ("callee", &callee)]);
    let trace = Trace::default();
    let error = same_error(&root, &sources, &sources, &HashMap::new(), &trace);
    assert!(error.detail.contains("unsupported stateful call signature"));
    primary(&trace, RefusalSite::node(Owner::Stateful(&callee)));
    assert_eq!(
        trace.events.borrow().as_slice(),
        &[
            event(RefusalSite::node(Owner::Stateful(&callee))),
            event(RefusalSite::node(Owner::Expr(value(&root)))),
        ]
    );
    assert!(trace.related.borrow().is_empty());
}
