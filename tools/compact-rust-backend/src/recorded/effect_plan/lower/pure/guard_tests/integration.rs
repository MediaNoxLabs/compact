use super::*;
#[test]
fn successful_pure_calls_keep_complete_checked_output_and_cache_without_events() {
    let callees = [pure(vec![], field("1"), Type::Field)];
    let original = root(
        Expr::Add {
            left: Box::new(call("helper", vec![])),
            right: Box::new(call("helper", vec![])),
        },
        Type::Field,
    );
    let trace = Trace::default();
    let observed = checked(&original, &callees, Some(&trace)).unwrap();
    assert_eq!(observed, checked(&original, &callees, None).unwrap());
    assert_eq!(
        observed.2, 1,
        "same retained certificate for repeated calls"
    );
    assert!(observed.1.contains("helper"));
    assert!(trace.primary.borrow().is_none());
    assert!(trace.related.borrow().is_empty());
    let callees = [pure(vec![], Expr::Unit, Type::Unit)];
    let mut original = root(Expr::Unit, Type::Unit);
    original.actions = vec![StateAction::PureCall {
        name: "helper".into(),
        arguments: vec![],
    }];
    original.return_value = StateReturn::Unit;
    let trace = Trace::default();
    let observed = checked(&original, &callees, Some(&trace)).unwrap();
    assert_eq!(observed, checked(&original, &callees, None).unwrap());
    assert_eq!(observed.2, 1);
    assert!(trace.events.borrow().is_empty());
    assert!(trace.related.borrow().is_empty());
}
#[test]
fn actual_recording_recorder_preserves_unavailable_gap_and_resets_success_session() {
    // No transport/binder: this exercises actual session/typed-gap lifecycle,
    // not source issuance or physical range availability.
    let bad = [pure(
        vec![],
        field("1"),
        Type::Unsigned { max: "bad".into() },
    )];
    let circuit = root(call("helper", vec![]), Type::Field);
    let StateReturn::Expression { value: original } = &circuit.return_value else {
        panic!("fixture")
    };
    let recorder =
        crate::origins::recording::Recorder::new(None, None, None, std::path::Path::new("."));
    recorder.begin_recording_root(&circuit);
    let observed = run(
        &env(&bad, &bad),
        Input::Dispatch(original),
        &Scope::new(),
        Some(&recorder),
    );
    let bare = run(
        &env(&bad, &bad),
        Input::Dispatch(original),
        &Scope::new(),
        None,
    );
    assert_eq!(observed, bare);
    let gap = observed.result.as_ref().unwrap_err();
    recorder.finish_recording_root(&circuit, Some(gap));
    let mut completed = recorder.take_completed();
    assert_eq!(completed.len(), 1);
    let diagnostic = completed.pop().unwrap();
    assert_eq!(diagnostic.gap, *gap);
    assert_eq!(diagnostic.circuit, "root");
    assert!(diagnostic.primary.is_none());
    assert!(diagnostic.related_call_sites.is_empty());
    let good = [pure(vec![], field("1"), Type::Field)];
    recorder.begin_recording_root(&circuit);
    let observed = run(
        &env(&good, &good),
        Input::Expression(original),
        &Scope::new(),
        Some(&recorder),
    );
    assert!(observed.result.is_ok());
    assert_eq!(
        observed,
        run(
            &env(&good, &good),
            Input::Expression(original),
            &Scope::new(),
            None
        )
    );
    recorder.finish_recording_root(&circuit, None);
    assert!(recorder.take_completed().is_empty());
}
