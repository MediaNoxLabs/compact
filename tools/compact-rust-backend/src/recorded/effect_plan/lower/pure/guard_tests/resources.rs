use super::*;
#[test]
fn combined_summaries_are_unavailable_and_preserve_old_depth_work_mutation() {
    let callees = [pure(vec![], field("1"), Type::Field)];
    let original = call("helper", vec![]);
    for seed in [Seed::AtDepth, Seed::AtWork] {
        let mut environment = env(&callees, &callees);
        environment.seed = seed;
        let trace = Trace::default();
        let outcome = error(&environment, Input::Expression(&original), &trace);
        primary(&trace, RefusalSite::unavailable());
        assert!(trace.events.borrow().iter().all(Option::is_none));
        assert_eq!(
            trace.related.borrow().as_slice(),
            &[site(expr_site(&original))]
        );
        assert_eq!(outcome.next, 11);
        assert_eq!(outcome.bindings, "[]");
        let (depth, work) = outcome
            .certificate
            .expect("same certification succeeded before budget guard");
        assert_eq!(depth, 1);
        assert!(work > 0);
        let (maximum_depth, maximum_work) = crate::resource_limits::recording_call_limits();
        match seed {
            Seed::AtDepth => {
                detail(&outcome, "combined recording/pure call depth");
                assert_eq!((outcome.stack.len(), outcome.spent), (maximum_depth, 0));
            }
            Seed::AtWork => {
                detail(&outcome, "combined recording/pure expanded work");
                assert_eq!(outcome.spent, maximum_work.saturating_add(work));
            }
            Seed::Plain => unreachable!(),
        }
    }
    let callees = [pure(
        vec![parameter("arg", Type::Field)],
        field("1"),
        Type::Field,
    )];
    let original = call("helper", vec![Expr::Boolean { value: true }]);
    let Expr::Call { arguments, .. } = &original else {
        panic!("fixture")
    };
    let mut environment = env(&callees, &callees);
    environment.seed = Seed::AtDepth;
    let trace = Trace::default();
    let outcome = error(&environment, Input::Expression(&original), &trace);
    detail(&outcome, "pure argument type differs");
    primary(&trace, expr_site(&arguments[0]));
    assert!(outcome.certificate.is_none());
    assert_eq!((outcome.next, outcome.spent), (12, 1));
}
