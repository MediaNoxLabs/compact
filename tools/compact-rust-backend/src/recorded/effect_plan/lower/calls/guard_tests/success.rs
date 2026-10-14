use super::*;

#[test]
fn supported_pipeline_keeps_ssa_paths_syntax_and_distinct_equal_call_identity() {
    let callee = circuit(
        "callee",
        vec![parameter("input", Type::Field)],
        Expr::Parameter {
            name: "input".into(),
        },
        Type::Field,
    );
    let root = circuit("root", vec![], call("callee", vec![field()]), Type::Field);
    let sources = HashMap::from([("root", &root), ("callee", &callee)]);
    let trace = Trace::default();
    assert_eq!(
        run(&root, &sources, &sources, &HashMap::new(), Some(&trace)).unwrap(),
        run(&root, &sources, &sources, &HashMap::new(), None).unwrap()
    );
    assert!(trace.primary.borrow().is_none());
    assert!(trace.events.borrow().is_empty());
    assert!(trace.related.borrow().is_empty());

    let original = call("absent", vec![]);
    let equal = [original.clone(), original];
    let mut identities = vec![];
    for source in &equal {
        let trace = Trace::default();
        let _outcome = direct_error(
            CallSite::Expression(source),
            "absent",
            &[],
            &HashMap::new(),
            &HashMap::new(),
            &trace,
            Seed::Ordinary,
        );
        primary(&trace, RefusalSite::node(Owner::Expr(source)));
        identities.push(std::ptr::from_ref(source));
    }
    assert_ne!(identities[0], identities[1]);
}

#[test]
fn unavailable_snapshot_first_write_survives_real_nested_body_join() {
    let original_bad = circuit("bad", vec![], field(), Type::Field);
    let snapshot_bad = circuit("bad", vec![], field(), Type::Unsigned { max: "bad".into() });
    let middle = circuit("mid", vec![], call("bad", vec![]), Type::Field);
    let root = circuit("root", vec![], call("mid", vec![]), Type::Field);
    let snapshots = HashMap::from([("root", &root), ("mid", &middle), ("bad", &snapshot_bad)]);
    let live = HashMap::from([("root", &root), ("mid", &middle), ("bad", &original_bad)]);
    let trace = Trace::default();
    let error = same_error(&root, &snapshots, &live, &HashMap::new(), &trace);
    assert!(error.detail.contains("bad"));
    primary(&trace, RefusalSite::unavailable());
    assert_eq!(
        trace.related.borrow().as_slice(),
        &[event(RefusalSite::node(Owner::Expr(value(&root))))]
    );
    assert_eq!(
        trace.events.borrow().as_slice(),
        &[
            event(RefusalSite::unavailable()),
            event(RefusalSite::node(Owner::Expr(value(&middle)))),
            event(RefusalSite::node(Owner::Expr(value(&root)))),
        ],
        "snapshot remains unavailable before original enclosing-call fallback"
    );
}
