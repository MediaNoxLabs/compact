use super::*;

#[test]
fn unsigned_canonical_struct_enum_and_ledger_map_keep_exact_original_producers() {
    let bad = Type::Unsigned { max: "bad".into() };
    let gap = validation(&bad, RefusalSite::field(Owner::Type(&bad), "max"));
    assert!(gap.resource.is_none());
    assert_eq!(gap.path, "actual.path");
    for ty in [
        structure("Missing", vec![]),
        enumeration("Missing", &["One"]),
        Type::LedgerMap {
            key: Box::new(Type::Field),
            value: Box::new(Type::Field),
        },
    ] {
        validation(&ty, RefusalSite::node(Owner::Type(&ty)));
    }
}

#[test]
fn noncanonical_equal_distinct_snapshots_keep_supplied_identity_and_none_stays_originless() {
    let root = source(structure("Record", vec![member("original", Type::Field)]));
    let env = env(&root, None).unwrap();
    let old = old_env(&root).unwrap();
    let first = structure("Record", vec![member("changed", Type::Field)]);
    let second = first.clone();
    assert_eq!(first, second);
    assert!(!std::ptr::eq(&first, &second));
    for current in [&first, &second] {
        let trace = Trace::default();
        let observed = env.validate_at_observed(current, "snapshot", Some(&trace));
        assert_eq!(observed, old.validate_at(current, "snapshot"));
        assert!(observed.is_err());
        one(&trace, RefusalSite::node(Owner::Type(current)));
        assert_eq!(env.validate_at(current, "snapshot"), observed);
    }
    let canonical_copy = root.parameters[0].ty.clone();
    let trace = Trace::default();
    env.validate_at_observed(&canonical_copy, "snapshot", Some(&trace))
        .unwrap();
    none(&trace);
    // An explicit observer is the caller's original-reference contract. Owned
    // production callers still use validate_at(None), not pointer inference.
}

#[test]
fn fixed_layout_and_borrowed_type_order_still_precede_scalar_and_canonical_checks() {
    let ty = Type::Tuple {
        elements: vec![
            Type::Unsigned {
                max: "first_bad".into(),
            },
            Type::Unsigned {
                max: "last_bad".into(),
            },
        ],
    };
    let Type::Tuple { elements } = &ty else {
        panic!("fixture")
    };
    // Existing bounded visitor uses LIFO children; no new semantic traversal.
    let nodes = crate::resource_limits::recording_type_nodes(&ty).unwrap();
    assert!(std::ptr::eq(nodes[1], &elements[1]));
    let gap = validation(&ty, RefusalSite::field(Owner::Type(&elements[1]), "max"));
    assert!(gap.detail.contains("last_bad"));
    let excessive = Type::Tuple {
        elements: vec![
            Type::Bytes {
                length: crate::resource_limits::recording_byte_payload_limit() + 1,
            },
            Type::Unsigned {
                max: "later_bad".into(),
            },
        ],
    };
    let Type::Tuple { elements } = &excessive else {
        panic!("fixture")
    };
    let gap = validation(&excessive, RefusalSite::node(Owner::Type(&elements[0])));
    assert!(
        gap.resource.is_some(),
        "existing fixed layout phase precedes unsigned_maximum"
    );
}

#[test]
fn global_type_preflight_failure_stays_unavailable_before_any_recursive_checks() {
    let mut deep = Type::Unsigned {
        max: "later_bad".into(),
    };
    for _ in 0..80 {
        deep = Type::Vector {
            element: Box::new(deep),
            length: 0,
        };
    }
    let gap = validation(&deep, RefusalSite::unavailable());
    assert!(gap.detail.contains("syntax_depth"));
    let source = source(deep);
    let trace = Trace::default();
    let current = env(&source, Some(&trace)).expect_err("old source preflight");
    assert_eq!(Some(&current), old_env(&source).as_ref().err());
    one(&trace, RefusalSite::unavailable());
}
