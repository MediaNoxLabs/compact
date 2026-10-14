use super::*;
#[test]
fn literal_parameter_query_and_aggregate_errors_keep_actual_checked_producer() {
    for source in [
        field("bad"),
        parameter("missing"),
        Expr::CellRead {
            field: "absent".into(),
            index: 0,
        },
        Expr::TupleIndex {
            value: Box::new(Expr::Boolean { value: true }),
            index: 0,
        },
    ] {
        let trace = Trace::default();
        let outcome = error(&source, &Scope::new(), &trace, 0);
        first(&trace, Some(&source));
        assert_eq!(outcome.stack, Vec::<String>::new());
    }
}
#[test]
fn relation_after_successful_child_uses_checked_context_without_operand_guess() {
    let source = Expr::Vector {
        element: Type::Field,
        elements: vec![Expr::Boolean { value: true }, parameter("later")],
    };
    let Expr::Vector { elements, .. } = &source else {
        panic!("fixture")
    };
    let trace = Trace::default();
    let outcome = error(&source, &Scope::new(), &trace, 0);
    assert!(
        outcome
            .result
            .as_ref()
            .unwrap_err()
            .detail
            .contains("element differs")
    );
    assert_eq!(outcome.next, 12);
    first(&trace, Some(&source));
    for child in elements {
        assert!(
            !trace
                .events
                .borrow()
                .contains(&Some(std::ptr::from_ref(child) as usize))
        );
    }
}
#[test]
fn first_child_and_missing_origin_cannot_be_promoted_by_parent_fallback() {
    let source = Expr::Add {
        left: Box::new(field("bad")),
        right: Box::new(parameter("later")),
    };
    let Expr::Add { left, right } = &source else {
        panic!("fixture")
    };
    let trace = Trace::default();
    let outcome = error(&source, &Scope::new(), &trace, 0);
    first(&trace, Some(left));
    assert_eq!((outcome.next, outcome.spent), (11, 2));
    assert!(
        !trace
            .events
            .borrow()
            .contains(&Some(std::ptr::from_ref(right.as_ref()) as usize))
    );
    let trace = Trace {
        available: Some(vec![std::ptr::from_ref(&source) as usize]),
        ..Trace::default()
    };
    let _outcome = error(&source, &Scope::new(), &trace, 0);
    first(&trace, None);
    assert!(
        trace
            .events
            .borrow()
            .contains(&Some(std::ptr::from_ref(&source) as usize))
    );
}
#[test]
fn successful_early_and_common_results_keep_ssa_without_refusal_events() {
    let scope = Scope::from([(
        "value".into(),
        Value {
            id: 99,
            ty: Type::Field,
        },
    )]);
    for source in [
        parameter("value"),
        Expr::Coerce {
            value: Box::new(parameter("value")),
            ty: Type::Field,
        },
        Expr::Add {
            left: Box::new(field("1")),
            right: Box::new(field("2")),
        },
        Expr::Tuple {
            elements: vec![field("1"), Expr::Boolean { value: true }],
        },
    ] {
        let trace = Trace::default();
        let observed = run(&source, &scope, Some(&trace), 0);
        assert!(observed.result.is_ok());
        assert_eq!(observed, run(&source, &scope, None, 0));
        assert!(trace.primary.borrow().is_none());
        assert!(trace.events.borrow().is_empty());
    }
}
#[test]
fn borrowed_assert_keeps_original_action_outer_error_and_child_first_capture() {
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum AssertSite {
        Expr(usize),
        Action(usize),
    }
    #[derive(Default)]
    struct AssertTrace {
        primary: RefCell<Option<Option<AssertSite>>>,
        events: RefCell<Vec<Option<AssertSite>>>,
        available: Option<Vec<AssertSite>>,
    }
    impl RefusalObserver for AssertTrace {
        fn refused(&self, site: RefusalSite<'_>) {
            assert_eq!(site.slot, Slot::Node);
            let actual = match site.owner {
                Some(Owner::Expr(value)) => {
                    Some(AssertSite::Expr(std::ptr::from_ref(value) as usize))
                }
                Some(Owner::Action(value)) => {
                    Some(AssertSite::Action(std::ptr::from_ref(value) as usize))
                }
                None => None,
                _ => panic!("assert boundary invented a different owner"),
            };
            let actual =
                actual.filter(|site| self.available.as_ref().is_none_or(|ids| ids.contains(site)));
            if self.primary.borrow().is_none() {
                *self.primary.borrow_mut() = Some(actual);
            }
            self.events.borrow_mut().push(actual);
        }
    }
    let invoke = |action: &StateAction, observer: Option<&dyn RefusalObserver>| {
        let declarations = Declarations::default();
        let pure = HashMap::new();
        let circuits = HashMap::new();
        let mut lower = lower(&declarations, &circuits, &pure, observer, 0);
        let mut bindings = vec![];
        let result = lower.action(action, &Scope::new(), &mut bindings, "action");
        (
            result,
            lower.next,
            lower.expanded_steps,
            format!("{bindings:?}"),
        )
    };
    // Exact former synthetic-fence fixture retained. R2 now uses the original
    // borrowed condition and Action; the relation error belongs to the Action.
    let action = StateAction::Assert {
        condition: field("1"),
        message: "runtime".into(),
    };
    let trace = AssertTrace::default();
    let observed = invoke(&action, Some(&trace));
    assert_eq!(observed, invoke(&action, None));
    assert!(observed.0.is_err());
    assert_eq!(
        observed.0.as_ref().unwrap_err().detail,
        "assert requires Boolean"
    );
    assert_eq!(
        (observed.1, observed.2),
        (12, 3),
        "same action/assert/child spends and child-only SSA"
    );
    let action_site = AssertSite::Action(std::ptr::from_ref(&action) as usize);
    assert_eq!(*trace.primary.borrow(), Some(Some(action_site)));
    assert_eq!(trace.events.borrow().as_slice(), &[Some(action_site)]);

    let action = StateAction::Assert {
        condition: field("bad"),
        message: "runtime".into(),
    };
    let StateAction::Assert { condition, .. } = &action else {
        panic!("fixture")
    };
    let child_site = AssertSite::Expr(std::ptr::from_ref(condition) as usize);
    let action_site = AssertSite::Action(std::ptr::from_ref(&action) as usize);
    let trace = AssertTrace::default();
    let observed = invoke(&action, Some(&trace));
    assert_eq!(observed, invoke(&action, None));
    assert!(observed.0.is_err());
    assert!(observed.0.as_ref().unwrap_err().detail.contains("bad"));
    assert_eq!((observed.1, observed.2), (11, 3));
    assert_eq!(*trace.primary.borrow(), Some(Some(child_site)));
    assert_eq!(
        trace.events.borrow().as_slice(),
        &[Some(child_site), Some(action_site)]
    );
    // The actual parent callback can supply context after a missing child range;
    // first-write unavailability remains primary rather than choosing an ancestor.
    let trace = AssertTrace {
        available: Some(vec![action_site]),
        ..AssertTrace::default()
    };
    assert_eq!(invoke(&action, Some(&trace)), observed);
    assert_eq!(*trace.primary.borrow(), Some(None));
    assert_eq!(trace.events.borrow().as_slice(), &[None, Some(action_site)]);
}
