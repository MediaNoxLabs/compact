use super::*;

#[test]
fn borrowed_resource_scan_preserves_actual_failed_nested_type_before_unavailable_fallback() {
    let mut ty = Type::Field;
    for _ in 0..80 {
        ty = Type::Vector {
            length: 1,
            element: Box::new(ty),
        };
    }
    let mut helper = source("deep", literal());
    helper.parameters.push(Parameter {
        name: "unused".into(),
        ty,
    });
    let mut originals = vec![];
    let mut current = &helper.parameters[0].ty;
    loop {
        originals.push(event(RefusalSite::node(Owner::Type(current))));
        let Type::Vector { element, .. } = current else {
            break;
        };
        current = element;
    }
    let trace = Trace::default();
    let expected = PureBodyBudget::default().check(&helper).unwrap_err();
    let actual = PureBodyBudget::default()
        .check_observed(&helper, Some(&trace))
        .unwrap_err();
    assert_eq!(actual, expected);
    assert!(matches!(
        actual,
        crate::RenderError::ResourceLimit {
            resource: "syntax_depth",
            ..
        }
    ));
    let first = trace.first.borrow().unwrap();
    assert!(
        originals.contains(&first),
        "selected by the same resource traversal, not guessed root"
    );
    assert_ne!(first, originals[0]);
    assert_eq!(
        trace.events.borrow().last(),
        Some(&event(RefusalSite::unavailable()))
    );
    let pure = HashMap::from([("deep", &helper)]);
    let certifier_trace = Trace::default();
    let gap = same_error(&pure, &HashMap::new(), "deep", &certifier_trace);
    assert!(gap.detail.contains("compiler resource"));
    assert_eq!(*certifier_trace.first.borrow(), Some(first));
}

#[test]
fn cumulative_work_refusal_stays_unavailable_with_only_actual_unwind_edges() {
    // Private Certifier input: public source preflight may reject expanded work earlier.
    // Fifteen levels stay under the unchanged call-depth limit; shared dependencies
    // repeatedly contribute existing summaries without expanding or cloning their bodies.
    let sources: Vec<_> = (0..15)
        .map(|index| {
            source(
                &format!("helper{index}"),
                if index == 14 {
                    literal()
                } else {
                    Expr::Add {
                        left: Box::new(call(&format!("helper{}", index + 1))),
                        right: Box::new(call(&format!("helper{}", index + 1))),
                    }
                },
            )
        })
        .collect();
    let pure = sources.iter().map(|s| (s.name.as_str(), s)).collect();
    let trace = Trace::default();
    let gap = same_error(&pure, &HashMap::new(), "helper0", &trace);
    assert!(
        gap.detail.contains("depth or expanded work limit exceeded"),
        "{gap:?}"
    );
    assert_eq!(
        *trace.first.borrow(),
        Some(event(RefusalSite::unavailable()))
    );
    assert_eq!(
        trace.events.borrow().as_slice(),
        &[event(RefusalSite::unavailable())],
        "the aggregate producer emits one unavailable primary"
    );
    assert!(
        trace.related.borrow().is_empty(),
        "root summary has no incoming edge"
    );

    // The unchanged guard executes after successful descendants. Establish the
    // actual root-summary case rather than assuming a child refusal.
    let stateful = HashMap::new();
    let mut certifier = Certifier::new(&pure, &stateful);
    assert_eq!(certifier.certify("helper0", "root.call").unwrap_err(), gap);
    assert!(certifier.get("helper0").is_none());
    let child = certifier
        .get("helper1")
        .expect("descendant summary completed");
    let (depth_limit, work_limit) = crate::resource_limits::recording_call_limits();
    assert!(child.expanded_work() <= work_limit);
    assert!(child.call_depth().saturating_add(1) <= depth_limit);
    let local = PureBodyBudget::default().check(&sources[0]).unwrap();
    assert!(
        local.saturating_add(child.expanded_work().saturating_mul(2)) > work_limit,
        "actual successful child summaries exceed work only when added at root"
    );
}

#[test]
fn cumulative_declaration_budget_fails_before_later_body_without_invented_source_leaf() {
    let sources: Vec<_> = (0..257)
        .map(|index| {
            source(
                &format!("helper{index}"),
                if index == 256 {
                    Expr::Parameter {
                        name: "later_body".into(),
                    }
                } else {
                    literal()
                },
            )
        })
        .collect();
    let pure = sources.iter().map(|s| (s.name.as_str(), s)).collect();
    let stateful = HashMap::new();
    let mut plain = Certifier::new(&pure, &stateful);
    let mut observed = Certifier::new(&pure, &stateful);
    let trace = Trace::default();
    for source in &sources[..256] {
        plain.certify(&source.name, "root.call").unwrap();
        observed
            .certify_observed(&source.name, "root.call", Some(&trace))
            .unwrap();
    }
    assert!(trace.events.borrow().is_empty());
    let expected = plain.certify("helper256", "root.call").unwrap_err();
    let actual = observed
        .certify_observed("helper256", "root.call", Some(&trace))
        .unwrap_err();
    assert_eq!(actual, expected);
    assert!(actual.detail.contains("declarations"));
    assert!(!actual.detail.contains("later_body"));
    assert_eq!(
        *trace.first.borrow(),
        Some(event(RefusalSite::unavailable()))
    );
    assert!(trace.related.borrow().is_empty());
}
