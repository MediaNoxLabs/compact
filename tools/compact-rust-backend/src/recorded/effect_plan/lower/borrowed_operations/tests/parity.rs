use super::*;

#[test]
fn borrowed_actions_preserve_frozen_gaps_partial_ssa_fuel_and_order() {
    let conditions = [
        Expr::Boolean { value: true },
        Expr::FieldLiteral { value: "1".into() },
        Expr::Parameter {
            name: "missing".into(),
        },
        Expr::Equal {
            left: Box::new(Expr::FieldLiteral { value: "1".into() }),
            right: Box::new(Expr::Parameter {
                name: "later".into(),
            }),
        },
    ];
    for condition in conditions {
        let action = assertion(condition);
        for remaining in [None, Some(1), Some(2), Some(3)] {
            let trace = Trace::default();
            let old = run(Input::Action(&action), &ledger(), None, true, remaining);
            let new = run(
                Input::Action(&action),
                &ledger(),
                Some(&trace),
                false,
                remaining,
            );
            assert_eq!(new, old, "gap/SSA/fuel/paths, remaining={remaining:?}");
            if new.0.is_ok() {
                assert!(trace.events.borrow().is_empty());
            }
        }
    }
}

#[test]
fn all_four_leaf_returns_preserve_frozen_query_checks_and_partial_state() {
    let fields = ledger();
    for (name, index) in [
        ("cell", 0),
        ("counter", 1),
        ("set", 2),
        ("absent", 0),
        ("cell", 7),
    ] {
        for returned in returns(name, index) {
            for remaining in [None, Some(0), Some(1)] {
                let trace = Trace::default();
                let old = run(Input::Return(&returned), &fields, None, true, remaining);
                let new = run(
                    Input::Return(&returned),
                    &fields,
                    Some(&trace),
                    false,
                    remaining,
                );
                assert_eq!(new, old, "same query error/emit/work, {returned:?}");
                if new.0.is_ok() {
                    let (_, work_limit) = crate::resource_limits::recording_call_limits();
                    let seeded = remaining.map_or(0, |remaining| work_limit - remaining);
                    assert_eq!(
                        new.2,
                        seeded + 1,
                        "one increment after the exact initial work seed"
                    );
                    assert_eq!(new.1, 18, "one original operation emit");
                    assert!(trace.events.borrow().is_empty());
                }
            }
        }
    }
}

#[test]
fn shared_original_type_checks_still_precede_kind_support_and_emit() {
    let mut fields = ledger();
    fields[0].declaration = LedgerFieldKind::Cell {
        ty: Type::Unsigned {
            max: "not-a-bound".into(),
        },
    };
    fields[2].declaration = LedgerFieldKind::Set {
        ty: Type::OpaqueString,
    };
    // OpaqueString is an established supported storage value. Preserve the
    // original mistaken negative fixture as an explicit positive oracle control.
    let accepted = returns("set", 2).into_iter().nth(2).unwrap();
    let plain = run(Input::Return(&accepted), &fields, None, true, None);
    let trace = Trace::default();
    let observed = run(Input::Return(&accepted), &fields, Some(&trace), false, None);
    assert_eq!(observed, plain);
    assert!(plain.0.is_ok());
    assert!(trace.events.borrow().is_empty());
    fields[2].declaration = LedgerFieldKind::Set {
        ty: Type::Bytes {
            length: crate::resource_limits::recording_byte_payload_limit() + 1,
        },
    };
    for returned in [
        returns("cell", 0).into_iter().next().unwrap(),
        returns("set", 2).into_iter().nth(2).unwrap(),
    ] {
        let trace = Trace::default();
        let old = run(Input::Return(&returned), &fields, None, true, None);
        let new = run(Input::Return(&returned), &fields, Some(&trace), false, None);
        assert_eq!(new, old);
        assert!(new.0.is_err());
        assert_eq!(new.1, 17, "failure occurs before emit");
    }
}
