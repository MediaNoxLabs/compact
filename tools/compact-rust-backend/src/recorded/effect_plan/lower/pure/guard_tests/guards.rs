use super::*;
#[test]
fn pure_unit_statement_kind_and_result_precede_arity_and_bad_child() {
    let callees = [pure(
        vec![parameter("arg", Type::Field)],
        field("1"),
        Type::Field,
    )];
    let original = StateAction::PureCall {
        name: "helper".into(),
        arguments: vec![field("bad"), field("bad")],
    };
    let trace = Trace::default();
    let outcome = error(
        &env(&callees, &callees),
        Input::Statement(&original),
        &trace,
    );
    detail(&outcome, "pure Unit statement requires");
    primary(&trace, action_site(&original));
    assert_eq!((outcome.next, outcome.spent), (11, 0));
    assert!(trace.related.borrow().is_empty());
    let mut state = root(Expr::Unit, Type::Unit);
    state.name = "helper".into();
    state.return_value = StateReturn::Unit;
    let states = [state];
    let mut environment = env(&[], &[]);
    environment.stateful = &states;
    let trace = Trace::default();
    let outcome = error(&environment, Input::Statement(&original), &trace);
    detail(&outcome, "pure Unit statement requires");
    primary(&trace, action_site(&original));
    assert_eq!((outcome.next, outcome.spent), (11, 0));
    let missing = StateAction::PureCall {
        name: "missing".into(),
        arguments: vec![field("bad")],
    };
    let trace = Trace::default();
    let _outcome = error(&environment, Input::Statement(&missing), &trace);
    primary(&trace, action_site(&missing));
    assert!(trace.related.borrow().is_empty());
}
#[test]
fn arity_missing_id_and_unsupported_interface_use_actual_whole_call() {
    let callees = [pure(vec![], field("1"), Type::Field)];
    let original = call("helper", vec![field("bad")]);
    let trace = Trace::default();
    let outcome = error(
        &env(&callees, &callees),
        Input::Expression(&original),
        &trace,
    );
    detail(&outcome, "pure argument count");
    primary(&trace, expr_site(&original));
    assert_eq!((outcome.next, outcome.spent), (11, 0));
    assert_eq!(
        trace.related.borrow().as_slice(),
        &[site(expr_site(&original))]
    );
    let missing = call("helper", vec![]);
    let trace = Trace::default();
    let outcome = error(&env(&[], &[]), Input::InvalidId(&missing), &trace);
    detail(&outcome, "unknown pure call declaration");
    primary(&trace, expr_site(&missing));
    assert!(trace.related.borrow().is_empty());
    let wide = Type::Struct {
        name: "WideMember".into(),
        fields: vec![StructField {
            name: "member".into(),
            ty: Type::Tuple {
                elements: vec![Type::Field; 9],
            },
        }],
    };
    let callees = [pure(vec![parameter("arg", wide)], field("1"), Type::Field)];
    let original = call("helper", vec![field("bad")]);
    let mut environment = env(&callees, &callees);
    environment.canonical = true;
    let trace = Trace::default();
    let outcome = error(&environment, Input::Expression(&original), &trace);
    detail(&outcome, "unsupported pure call interface");
    primary(&trace, expr_site(&original));
    assert_eq!((outcome.next, outcome.spent), (11, 0));
    assert!(outcome.certificate.is_none());
}
#[test]
fn cloned_signature_result_then_parameters_refuse_unavailable_before_child() {
    let bad_result = Type::Unsigned {
        max: "result_bad".into(),
    };
    let bad_parameter = Type::Unsigned {
        max: "parameter_bad".into(),
    };
    let snapshots = [pure(
        vec![parameter("arg", bad_parameter.clone())],
        field("1"),
        bad_result,
    )];
    let live = [pure(
        vec![parameter("arg", Type::Field)],
        field("1"),
        Type::Field,
    )];
    let original = call("helper", vec![field("bad")]);
    let trace = Trace::default();
    let outcome = error(
        &env(&snapshots, &live),
        Input::Expression(&original),
        &trace,
    );
    detail(&outcome, "result_bad");
    primary(&trace, RefusalSite::unavailable());
    assert_eq!((outcome.next, outcome.spent), (11, 0));
    assert!(trace.events.borrow().iter().all(Option::is_none));
    let snapshots = [pure(
        vec![
            parameter(
                "first",
                Type::Unsigned {
                    max: "first_bad".into(),
                },
            ),
            parameter("second", bad_parameter),
        ],
        field("1"),
        Type::Field,
    )];
    let original = call("helper", vec![field("bad"), field("bad")]);
    let trace = Trace::default();
    let outcome = error(
        &env(&snapshots, &live),
        Input::Expression(&original),
        &trace,
    );
    detail(&outcome, "first_bad");
    primary(&trace, RefusalSite::unavailable());
    assert_eq!((outcome.next, outcome.spent), (11, 0));
    assert!(trace.events.borrow().iter().all(Option::is_none));
}
#[test]
fn exact_argument_type_uses_original_operand_after_prior_ssa_and_before_certification() {
    let callees = [pure(
        vec![
            parameter("first", Type::Field),
            parameter("second", Type::Field),
        ],
        field("bad"),
        Type::Field,
    )];
    let original = call("helper", vec![field("1"), Expr::Boolean { value: true }]);
    let Expr::Call { arguments, .. } = &original else {
        panic!("fixture")
    };
    let trace = Trace::default();
    let outcome = error(
        &env(&callees, &callees),
        Input::Expression(&original),
        &trace,
    );
    detail(&outcome, "pure argument type differs");
    primary(&trace, expr_site(&arguments[1]));
    assert_eq!((outcome.next, outcome.spent), (13, 2));
    assert!(outcome.certificate.is_none());
    assert!(
        !trace
            .events
            .borrow()
            .contains(&site(expr_site(&callees[0].body)))
    );
    let original = call("helper", vec![field("bad"), Expr::Boolean { value: true }]);
    let Expr::Call { arguments, .. } = &original else {
        panic!("fixture")
    };
    let trace = Trace::default();
    let outcome = error(
        &env(&callees, &callees),
        Input::Expression(&original),
        &trace,
    );
    primary(&trace, expr_site(&arguments[0]));
    assert_eq!((outcome.next, outcome.spent), (11, 1));
    assert!(outcome.certificate.is_none());
}
#[test]
fn certification_keeps_real_callee_child_and_exact_incoming_call_relation() {
    let callees = [pure(vec![], field("bad"), Type::Field)];
    let original = call("helper", vec![]);
    let trace = Trace::default();
    let outcome = error(&env(&callees, &callees), Input::Dispatch(&original), &trace);
    detail(&outcome, "pure helper");
    primary(&trace, expr_site(&callees[0].body));
    assert_eq!(
        trace.related.borrow().as_slice(),
        &[site(expr_site(&original))]
    );
    assert!(outcome.certificate.is_none());
    let trace = Trace {
        available: Some(vec![site(expr_site(&original)).unwrap()]),
        ..Trace::default()
    };
    let _outcome = error(&env(&callees, &callees), Input::Dispatch(&original), &trace);
    primary(&trace, RefusalSite::unavailable());
    assert!(trace.events.borrow().contains(&site(expr_site(&original))));
    assert_eq!(
        trace.related.borrow().as_slice(),
        &[site(expr_site(&original))]
    );
}
#[test]
fn equal_call_and_argument_occurrences_do_not_collapse_identity() {
    let callees = [pure(
        vec![parameter("arg", Type::Field)],
        field("1"),
        Type::Field,
    )];
    let original = call("helper", vec![Expr::Boolean { value: true }]);
    let calls = [original.clone(), original];
    let mut sites = vec![];
    for original in &calls {
        let Expr::Call { arguments, .. } = original else {
            panic!("fixture")
        };
        let trace = Trace::default();
        let _outcome = error(
            &env(&callees, &callees),
            Input::Expression(original),
            &trace,
        );
        primary(&trace, expr_site(&arguments[0]));
        sites.push(site(expr_site(&arguments[0])));
    }
    assert_eq!(calls[0], calls[1]);
    assert_ne!(sites[0], sites[1]);
    let callees = [pure(vec![], field("1"), Type::Field)];
    let mut sites = vec![];
    for original in &calls {
        let trace = Trace::default();
        let _outcome = error(
            &env(&callees, &callees),
            Input::Expression(original),
            &trace,
        );
        primary(&trace, expr_site(original));
        sites.push(site(expr_site(original)));
    }
    assert_ne!(sites[0], sites[1]);
}
