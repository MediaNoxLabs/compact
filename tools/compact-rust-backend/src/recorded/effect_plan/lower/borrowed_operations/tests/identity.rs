use super::*;

#[test]
fn assertion_outer_mismatch_is_original_action_not_a_temporary_expression() {
    let action = assertion(Expr::FieldLiteral { value: "1".into() });
    let trace = Trace::default();
    let outcome = run(Input::Action(&action), &ledger(), Some(&trace), false, None);
    assert!(
        outcome
            .0
            .unwrap_err()
            .detail
            .contains("assert requires Boolean")
    );
    assert_eq!(
        *trace.primary.borrow(),
        Some(event(RefusalSite::node(Owner::Action(&action))))
    );
}

#[test]
fn leaf_failures_keep_each_original_return_occurrence_and_no_success_event() {
    let a = returns("absent", 0);
    let b = returns("absent", 0);
    for (left, right) in a.iter().zip(&b) {
        assert_eq!(
            left, right,
            "semantic equality is deliberately not identity"
        );
        let trace = Trace::default();
        let result = run(Input::Return(left), &ledger(), Some(&trace), false, None);
        assert!(result.0.is_err());
        assert_eq!(
            *trace.primary.borrow(),
            Some(event(RefusalSite::node(Owner::StateReturn(left))))
        );
        assert_ne!(
            *trace.primary.borrow(),
            Some(event(RefusalSite::node(Owner::StateReturn(right))))
        );
    }
    for returned in [
        returns("cell", 0).into_iter().next().unwrap(),
        returns("counter", 1).into_iter().nth(1).unwrap(),
        returns("set", 2).into_iter().nth(2).unwrap(),
        returns("set", 2).into_iter().nth(3).unwrap(),
    ] {
        let trace = Trace::default();
        assert!(
            run(
                Input::Return(&returned),
                &ledger(),
                Some(&trace),
                false,
                None
            )
            .0
            .is_ok()
        );
        assert!(trace.primary.borrow().is_none());
    }
}
