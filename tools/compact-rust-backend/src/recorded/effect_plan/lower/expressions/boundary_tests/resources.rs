use super::*;
#[test]
fn inferred_owned_type_failure_is_unavailable_before_ancestor_context() {
    // Defensive malformed private Scope, not a source/frontend acceptance case.
    let scope = Scope::from([(
        "value".into(),
        Value {
            id: 99,
            ty: Type::Tuple {
                elements: vec![Type::Unsigned { max: "bad".into() }],
            },
        },
    )]);
    let projection = Expr::TupleIndex {
        value: Box::new(parameter("value")),
        index: 0,
    };
    let source = Expr::Add {
        left: Box::new(projection),
        right: Box::new(field("1")),
    };
    let trace = Trace::default();
    let outcome = error(&source, &scope, &trace, 0);
    assert!(outcome.result.as_ref().unwrap_err().detail.contains("bad"));
    first(&trace, None);
    assert_eq!(outcome.next, 11, "validation failed before projection emit");
    assert_eq!(
        trace
            .events
            .borrow()
            .iter()
            .filter(|event| event.is_none())
            .count(),
        1
    );
    assert!(
        trace
            .events
            .borrow()
            .contains(&Some(std::ptr::from_ref(&source) as usize))
    );
}
#[test]
fn fuel_has_one_unavailable_producer_before_ancestor_fallback() {
    let maximum = crate::resource_limits::recording_call_limits().1;
    let source = field("1");
    let trace = Trace::default();
    let outcome = error(&source, &Scope::new(), &trace, maximum);
    assert!(
        outcome
            .result
            .as_ref()
            .unwrap_err()
            .detail
            .contains("expansion work exceeds")
    );
    first(&trace, None);
    assert_eq!(trace.events.borrow().as_slice(), &[None]);
    assert_eq!((outcome.next, outcome.spent), (11, maximum));
    let source = Expr::Add {
        left: Box::new(field("1")),
        right: Box::new(field("2")),
    };
    let trace = Trace::default();
    let outcome = error(&source, &Scope::new(), &trace, maximum - 1);
    first(&trace, None);
    assert_eq!((outcome.next, outcome.spent), (11, maximum));
    assert_eq!(
        trace
            .events
            .borrow()
            .iter()
            .filter(|event| event.is_none())
            .count(),
        1
    );
}
#[test]
fn formal_spend_does_not_duplicate_the_generic_producer_event() {
    let callee = StatefulCircuit {
        source: None,
        name: "callee".into(),
        internal: true,
        parameters: vec![crate::ir::Parameter {
            name: "value".into(),
            ty: Type::Field,
        }],
        actions: vec![],
        result: Type::Field,
        return_value: StateReturn::Expression {
            value: parameter("value"),
        },
    };
    let source = Expr::Call {
        name: "callee".into(),
        arguments: vec![field("1")],
    };
    let Expr::Call { arguments, .. } = &source else {
        panic!("fixture")
    };
    let maximum = crate::resource_limits::recording_call_limits().1;
    let invoke = |observer: Option<&dyn RefusalObserver>| {
        let ledger = HashMap::new();
        let witnesses = HashMap::new();
        let pure = HashMap::new();
        let circuits = HashMap::from([("callee", &callee)]);
        let declarations = Declarations::with_calls(&ledger, &witnesses, &pure, &circuits);
        let mut lower = lower(&declarations, &circuits, &pure, observer, maximum - 1);
        let mut bindings = vec![];
        let result = lower.call(
            crate::recorded::effect_plan::lower::calls::CallSite::Expression(&source),
            "callee",
            arguments,
            &Scope::new(),
            &mut bindings,
            "call",
        );
        (
            result,
            lower.next,
            lower.expanded_steps,
            format!("{bindings:?}"),
        )
    };
    let trace = Trace::default();
    let observed = invoke(Some(&trace));
    assert_eq!(observed, invoke(None));
    assert!(observed.0.is_err());
    assert_eq!((observed.1, observed.2), (12, maximum));
    first(&trace, None);
    assert_eq!(trace.events.borrow().as_slice(), &[None]);
}
