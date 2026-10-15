use super::*;

#[test]
fn resolve_kind_and_missing_body_guards_keep_exact_original_calls_before_children() {
    let absent = circuit(
        "root",
        vec![],
        call(
            "absent",
            vec![Expr::Parameter {
                name: "later".into(),
            }],
        ),
        Type::Field,
    );
    let trace = Trace::default();
    same_error(
        &absent,
        &registry(std::slice::from_ref(&absent)),
        &registry(std::slice::from_ref(&absent)),
        &HashMap::new(),
        &trace,
    );
    primary(&trace, RefusalSite::node(Owner::Expr(value(&absent))));

    let mut root = circuit("root", vec![], Expr::Unit, Type::Unit);
    root.return_value = StateReturn::Unit;
    root.actions = vec![StateAction::CircuitCall {
        name: "pure".into(),
        arguments: vec![Expr::Parameter {
            name: "later".into(),
        }],
    }];
    let pure = PureCircuit {
        source: None,
        name: "pure".into(),
        internal: false,
        parameters: vec![],
        result: Type::Field,
        body: field(),
    };
    let trace = Trace::default();
    same_error(
        &root,
        &registry(std::slice::from_ref(&root)),
        &registry(std::slice::from_ref(&root)),
        &HashMap::from([("pure", &pure)]),
        &trace,
    );
    primary(&trace, RefusalSite::node(Owner::Action(&root.actions[0])));

    let snapshot = circuit("callee", vec![], field(), Type::Field);
    let source = call(
        "callee",
        vec![Expr::Parameter {
            name: "later".into(),
        }],
    );
    let trace = Trace::default();
    let outcome = direct_error(
        CallSite::Expression(&source),
        "callee",
        &[],
        &registry(std::slice::from_ref(&snapshot)),
        &HashMap::new(),
        &trace,
        Seed::Ordinary,
    );
    detail(&outcome, "body is unavailable");
    primary(&trace, RefusalSite::node(Owner::Expr(&source)));
    assert_eq!(outcome.1, 11);
    assert_eq!(outcome.2, 0);
}

#[test]
fn arity_recursion_and_default_depth_stop_before_any_argument_evaluation() {
    let callee = circuit(
        "callee",
        vec![parameter("input", Type::Field)],
        field(),
        Type::Field,
    );
    let sources = registry(std::slice::from_ref(&callee));
    let source = call("callee", vec![]);
    let trace = Trace::default();
    let outcome = direct_error(
        CallSite::Expression(&source),
        "callee",
        &[],
        &sources,
        &sources,
        &trace,
        Seed::Ordinary,
    );
    detail(&outcome, "argument count");
    primary(&trace, RefusalSite::node(Owner::Expr(&source)));
    assert_eq!((outcome.1, outcome.2), (11, 0));

    let recursive = circuit("outer", vec![], field(), Type::Field);
    let sources = registry(std::slice::from_ref(&recursive));
    let source = call("outer", vec![]);
    let trace = Trace::default();
    let outcome = direct_error(
        CallSite::Expression(&source),
        "outer",
        &[],
        &sources,
        &sources,
        &trace,
        Seed::Ordinary,
    );
    detail(&outcome, "recursive");
    primary(&trace, RefusalSite::node(Owner::Expr(&source)));
    assert_eq!((outcome.1, outcome.2), (11, 0));

    let source = call(
        "callee",
        vec![Expr::Parameter {
            name: "later".into(),
        }],
    );
    let Expr::Call { arguments, .. } = &source else {
        panic!("fixture")
    };
    let sources = registry(std::slice::from_ref(&callee));
    let trace = Trace::default();
    let outcome = direct_error(
        CallSite::Expression(&source),
        "callee",
        arguments,
        &sources,
        &sources,
        &trace,
        Seed::AtDepthLimit,
    );
    detail(&outcome, "call depth exceeds");
    primary(&trace, RefusalSite::node(Owner::Expr(&source)));
    assert_eq!((outcome.1, outcome.2), (11, 0));
}

#[test]
fn expression_and_statement_argument_type_failures_keep_first_actual_argument_and_partial_ssa() {
    let callee = circuit(
        "callee",
        vec![parameter("a", Type::Field), parameter("b", Type::Field)],
        field(),
        Type::Field,
    );
    let sources = registry(std::slice::from_ref(&callee));
    let arguments = vec![
        Expr::Boolean { value: true },
        Expr::Parameter {
            name: "later".into(),
        },
    ];
    let expression = call("callee", arguments.clone());
    let action = StateAction::CircuitCall {
        name: "callee".into(),
        arguments,
    };
    let Expr::Call { arguments, .. } = &expression else {
        panic!("fixture")
    };
    let trace = Trace::default();
    let outcome = direct_error(
        CallSite::Expression(&expression),
        "callee",
        arguments,
        &sources,
        &sources,
        &trace,
        Seed::Ordinary,
    );
    detail(&outcome, "cannot coerce");
    primary(&trace, RefusalSite::node(Owner::Expr(&arguments[0])));
    assert_eq!((outcome.1, outcome.2), (12, 1));
    assert_eq!(outcome.3, vec!["outer".to_string()]);
    let StateAction::CircuitCall { arguments, .. } = &action else {
        panic!("fixture")
    };
    let trace = Trace::default();
    let outcome = direct_error(
        CallSite::Statement(&action),
        "callee",
        arguments,
        &sources,
        &sources,
        &trace,
        Seed::Ordinary,
    );
    detail(&outcome, "statement call argument type");
    primary(&trace, RefusalSite::node(Owner::Expr(&arguments[0])));
    assert_eq!((outcome.1, outcome.2), (12, 1));
    assert!(trace.related.borrow().is_empty());
}

#[test]
fn duplicate_formal_and_derived_fuel_failure_preserve_original_or_unavailable_after_arguments() {
    let duplicate = circuit(
        "callee",
        vec![parameter("x", Type::Field), parameter("x", Type::Field)],
        field(),
        Type::Field,
    );
    let sources = registry(std::slice::from_ref(&duplicate));
    let source = call("callee", vec![field(), field()]);
    let Expr::Call { arguments, .. } = &source else {
        panic!("fixture")
    };
    let trace = Trace::default();
    let outcome = direct_error(
        CallSite::Expression(&source),
        "callee",
        arguments,
        &sources,
        &sources,
        &trace,
        Seed::Ordinary,
    );
    detail(&outcome, "duplicate callee parameter");
    primary(
        &trace,
        RefusalSite::field(Owner::Parameter(&duplicate.parameters[1]), "name"),
    );
    assert_eq!((outcome.1, outcome.2), (15, 4));
    assert_eq!(outcome.3, vec!["outer".to_string()]);

    let callee = circuit(
        "callee",
        vec![parameter("x", Type::Field)],
        field(),
        Type::Field,
    );
    let sources = registry(std::slice::from_ref(&callee));
    // Seed one below the actual budget so the actual literal uses the final
    // expression unit; formal spend then refuses without a guessed owner.
    let source = call("callee", vec![field()]);
    let Expr::Call { arguments, .. } = &source else {
        panic!("fixture")
    };
    let trace = Trace::default();
    let outcome = direct_error(
        CallSite::Expression(&source),
        "callee",
        arguments,
        &sources,
        &sources,
        &trace,
        Seed::BeforeWorkLimit,
    );
    detail(&outcome, "expansion work exceeds");
    primary(&trace, RefusalSite::unavailable());
    assert_eq!(outcome.1, 12, "argument emitted before formal work refusal");
    assert_eq!(outcome.2, crate::resource_limits::recording_call_limits().1);
}
