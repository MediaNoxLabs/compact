use super::*;

#[test]
fn missing_dependency_uses_actual_first_equal_call_name_not_value_identity() {
    let sources = [source(
        "root",
        Expr::Add {
            left: Box::new(call("absent")),
            right: Box::new(call("absent")),
        },
    )];
    let Expr::Add { left, right } = &sources[0].body else {
        panic!("fixture")
    };
    assert_eq!(left, right);
    assert_ne!(
        std::ptr::from_ref(left.as_ref()),
        std::ptr::from_ref(right.as_ref())
    );
    let pure = HashMap::from([("root", &sources[0])]);
    let trace = Trace::default();
    let gap = same_error(&pure, &HashMap::new(), "root", &trace);
    assert!(gap.detail.contains("unknown pure callee"));
    let first = event(RefusalSite::field(Owner::Expr(left), "name"));
    assert_eq!(*trace.first.borrow(), Some(first));
    assert_eq!(
        trace.related.borrow().as_slice(),
        &[event(RefusalSite::node(Owner::Expr(left)))]
    );
    assert!(
        !trace
            .events
            .borrow()
            .contains(&event(RefusalSite::field(Owner::Expr(right), "name")))
    );
}

#[test]
fn ambiguous_dependency_captures_call_reference_before_callee_body_or_annotation() {
    let sources = [
        source("root", call("callee")),
        source(
            "different",
            Expr::Parameter {
                name: "later".into(),
            },
        ),
    ];
    let pure = HashMap::from([("root", &sources[0]), ("callee", &sources[1])]);
    let trace = Trace::default();
    let gap = same_error(&pure, &HashMap::new(), "root", &trace);
    assert!(gap.detail.contains("ambiguous pure/stateful"));
    assert!(!gap.detail.contains("later"));
    assert_eq!(
        *trace.first.borrow(),
        Some(event(RefusalSite::field(
            Owner::Expr(&sources[0].body),
            "name"
        )))
    );
    assert_eq!(trace.related.borrow().len(), 1);
}

#[test]
fn unknown_or_ambiguous_top_entry_without_incoming_occurrence_is_unavailable() {
    let source = source("different", literal());
    for (pure, name) in [
        (HashMap::new(), "absent"),
        (HashMap::from([("alias", &source)]), "alias"),
    ] {
        let trace = Trace::default();
        same_error(&pure, &HashMap::new(), name, &trace);
        assert_eq!(
            *trace.first.borrow(),
            Some(event(RefusalSite::unavailable()))
        );
        assert!(
            trace.related.borrow().is_empty(),
            "no invented incoming edge"
        );
    }
}

#[test]
fn normalization_collision_uses_current_original_name_after_prior_success() {
    let sources = [
        source(
            "root",
            Expr::Add {
                left: Box::new(call("helper$0")),
                right: Box::new(call("helper_0")),
            },
        ),
        source("helper$0", literal()),
        source(
            "helper_0",
            Expr::Parameter {
                name: "later_body_error".into(),
            },
        ),
    ];
    let pure = sources.iter().map(|s| (s.name.as_str(), s)).collect();
    let trace = Trace::default();
    let gap = same_error(&pure, &HashMap::new(), "root", &trace);
    assert!(
        gap.detail
            .contains("names collide after Rust normalization")
    );
    assert!(!gap.detail.contains("later_body_error"));
    assert_eq!(
        *trace.first.borrow(),
        Some(event(RefusalSite::field(Owner::Pure(&sources[2]), "name")))
    );
}
