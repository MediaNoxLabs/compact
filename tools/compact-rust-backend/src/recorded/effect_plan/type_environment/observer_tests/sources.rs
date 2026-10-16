use super::*;

#[test]
fn borrowed_named_collection_keeps_original_guard_before_later_annotation() {
    let root = source(structure(
        "Outer",
        vec![
            member("a", Type::Unsigned { max: "bad".into() }),
            member("a", enumeration("Empty", &[])),
        ],
    ));
    let Type::Struct { fields, .. } = &root.parameters[0].ty else {
        panic!("fixture")
    };
    let trace = Trace::default();
    let current = env(&root, Some(&trace)).expect_err("existing named duplicate");
    assert_eq!(Some(&current), old_env(&root).as_ref().err());
    assert!(current.detail.contains("a"));
    one(
        &trace,
        RefusalSite::field(Owner::StructField(&fields[1]), "name"),
    );
    let trace = Trace::default();
    let cloned = root.clone();
    let current = env(&cloned, Some(&trace)).expect_err("same old guard");
    assert_eq!(Some(&current), old_env(&cloned).as_ref().err());
    let Type::Struct { fields: copied, .. } = &cloned.parameters[0].ty else {
        panic!("fixture")
    };
    assert_eq!(fields, copied);
    assert!(!std::ptr::eq(&fields[1], &copied[1]));
    one(
        &trace,
        RefusalSite::field(Owner::StructField(&copied[1]), "name"),
    );
}

#[test]
fn late_owned_member_collisions_remain_unavailable_and_source_collection_wins_first() {
    for ty in [
        structure(
            "Record",
            vec![member("a$b", Type::Field), member("a_b", Type::Boolean)],
        ),
        enumeration("Status", &["a$b", "a_b"]),
    ] {
        let root = source(ty);
        let trace = Trace::default();
        let current = env(&root, Some(&trace)).expect_err("old late names phase");
        assert_eq!(Some(&current), old_env(&root).as_ref().err());
        assert_eq!(
            current.detail,
            "named members collide after Rust identifier normalization"
        );
        one(&trace, RefusalSite::unavailable());
    }
    let root = source(Type::Tuple {
        elements: vec![
            structure(
                "Late",
                vec![member("a$b", Type::Field), member("a_b", Type::Boolean)],
            ),
            enumeration("Early", &[]),
        ],
    });
    let Type::Tuple { elements } = &root.parameters[0].ty else {
        panic!("fixture")
    };
    let trace = Trace::default();
    let current = env(&root, Some(&trace)).expect_err("source before late tables");
    assert_eq!(Some(&current), old_env(&root).as_ref().err());
    one(
        &trace,
        RefusalSite::field(Owner::Type(&elements[1]), "variants"),
    );
}

#[test]
fn temporary_builtin_conflict_is_unavailable_and_successful_sources_keep_exact_definitions() {
    let mut root = source(structure(
        "ZswapCoinPublicKey",
        vec![member("different", Type::Field)],
    ));
    root.actions = vec![StateAction::NativeWitnessCall {
        builtin: NativeWitnessBuiltin::OwnPublicKey,
    }];
    let trace = Trace::default();
    let current = env(&root, Some(&trace)).expect_err("old builtin merge");
    assert_eq!(Some(&current), old_env(&root).as_ref().err());
    one(&trace, RefusalSite::unavailable());
    let mut root = source(structure(
        "Record",
        vec![member("value", enumeration("Status", &["Ready", "Done"]))],
    ));
    root.actions = vec![StateAction::Let {
        bindings: vec![LocalBinding {
            name: "unused".into(),
            ty: root.parameters[0].ty.clone(),
            value: Expr::Default {
                ty: root.parameters[0].ty.clone(),
            },
        }],
        action: Box::new(StateAction::Expression { value: Expr::Unit }),
    }];
    let trace = Trace::default();
    let current = env(&root, Some(&trace)).unwrap();
    let old = old_env(&root).unwrap();
    assert_eq!(format!("{current:?}"), format!("{old:?}"));
    current.validate(&root.parameters[0].ty).unwrap();
    old.validate(&root.parameters[0].ty).unwrap();
    none(&trace);
}
