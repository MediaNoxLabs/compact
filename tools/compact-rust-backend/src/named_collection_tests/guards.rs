use super::*;

#[test]
fn all_nine_named_guards_keep_current_original_and_old_partial_definitions() {
    let cases = vec![
        (
            enumeration("Shared", &["One"]),
            product("Shared", vec![]),
            RenderError::ConflictingStruct("Shared".into()),
        ),
        (
            product("Shared", vec![member("a", Type::Field)]),
            product("Shared", vec![]),
            RenderError::ConflictingStruct("Shared".into()),
        ),
        (
            product("Shared", vec![]),
            product("r#Shared", vec![]),
            RenderError::ConflictingStruct("r#Shared".into()),
        ),
        (
            Type::Unit,
            product(
                "Shared",
                vec![member("a", Type::Field), member("a", Type::Boolean)],
            ),
            RenderError::DuplicateStructField("a".into()),
        ),
        (
            product("Shared", vec![]),
            enumeration("Shared", &["One"]),
            RenderError::ConflictingEnum("Shared".into()),
        ),
        (
            Type::Unit,
            enumeration("Shared", &[]),
            RenderError::EmptyEnum("Shared".into()),
        ),
        (
            Type::Unit,
            enumeration("Shared", &["One", "One"]),
            RenderError::DuplicateEnumVariant("One".into()),
        ),
        (
            enumeration("Shared", &["One"]),
            enumeration("Shared", &["Two"]),
            RenderError::ConflictingEnum("Shared".into()),
        ),
        (
            enumeration("Shared", &["One"]),
            enumeration("r#Shared", &["One"]),
            RenderError::ConflictingEnum("r#Shared".into()),
        ),
    ];
    for (first, later, expected) in &cases {
        let old = collected(|s, e, o| {
            old_reference::collect_named_types_with_origins(first, s, e, Some(o))?;
            old_reference::collect_named_types_with_origins(later, s, e, Some(o))
        });
        let trace = Trace::default();
        let current = collected(|s, e, o| {
            collect_named_types_observed(first, s, e, Some(&trace), Some(o))?;
            collect_named_types_observed(later, s, e, Some(&trace), Some(o))
        });
        assert_eq!(
            current, old,
            "same cause/partial insertion/first-source authority"
        );
        assert_eq!(current.0.as_ref().err(), Some(expected));
        let site = match (expected, later) {
            (RenderError::DuplicateStructField(_), Type::Struct { fields, .. }) => {
                RefusalSite::field(Owner::StructField(&fields[1]), "name")
            }
            (RenderError::EmptyEnum(_), _) => RefusalSite::field(Owner::Type(later), "variants"),
            (RenderError::DuplicateEnumVariant(_), _) => {
                RefusalSite::element(Owner::Type(later), "variants", 1)
            }
            _ => RefusalSite::node(Owner::Type(later)),
        };
        one(&trace, site);
    }
}

#[test]
fn early_conflict_precedes_child_names_and_preserves_first_successful_origin() {
    let first = product("Shared", vec![member("value", Type::Field)]);
    let repeated = first.clone();
    let later = product("Shared", vec![member("crate", invalid())]);
    let mut structs = BTreeMap::new();
    let mut enums = BTreeMap::new();
    let mut origins = type_declarations::DefinitionOrigins::new();
    let trace = Trace::default();
    collect_named_types_observed(
        &first,
        &mut structs,
        &mut enums,
        Some(&trace),
        Some(&mut origins),
    )
    .unwrap();
    collect_named_types_observed(
        &repeated,
        &mut structs,
        &mut enums,
        Some(&trace),
        Some(&mut origins),
    )
    .unwrap();
    assert!(std::ptr::eq(origins["Shared"], &first));
    assert!(!std::ptr::eq(origins["Shared"], &repeated));
    assert!(trace.0.borrow().is_empty());
    assert_eq!(
        collect_named_types_observed(
            &later,
            &mut structs,
            &mut enums,
            Some(&trace),
            Some(&mut origins)
        ),
        Err(RenderError::ConflictingStruct("Shared".into()))
    );
    one(&trace, RefusalSite::node(Owner::Type(&later)));
    assert!(std::ptr::eq(origins["Shared"], &first));
    let outer = product("Outer", vec![member("child", invalid())]);
    let Type::Struct { fields, .. } = &outer else {
        panic!("fixture")
    };
    let trace = Trace::default();
    let current =
        collected(|s, e, o| collect_named_types_observed(&outer, s, e, Some(&trace), Some(o)));
    let old =
        collected(|s, e, o| old_reference::collect_named_types_with_origins(&outer, s, e, Some(o)));
    assert_eq!(current, old);
    assert!(
        current.1.contains_key("Outer"),
        "old insert precedes child failure"
    );
    one(
        &trace,
        RefusalSite::field(Owner::Type(&fields[0].ty), "name"),
    );
}

#[test]
fn tuple_zero_vector_and_ledger_map_keep_original_leaf_and_key_first_order() {
    let ty = Type::Tuple {
        elements: vec![Type::Vector {
            length: 0,
            element: Box::new(Type::LedgerMap {
                key: Box::new(invalid()),
                value: Box::new(enumeration("Later", &[])),
            }),
        }],
    };
    let Type::Tuple { elements } = &ty else {
        panic!("fixture")
    };
    let Type::Vector { element, .. } = &elements[0] else {
        panic!("fixture")
    };
    let Type::LedgerMap { key, .. } = element.as_ref() else {
        panic!("fixture")
    };
    let trace = Trace::default();
    assert_eq!(
        collected(|s, e, o| collect_named_types_observed(&ty, s, e, Some(&trace), Some(o))),
        collected(|s, e, o| old_reference::collect_named_types_with_origins(&ty, s, e, Some(o)))
    );
    one(&trace, RefusalSite::field(Owner::Type(key), "name"));
}

#[test]
fn builtin_temporary_conflict_is_unavailable_without_manufactured_origin() {
    let source = Expr::Tuple {
        elements: vec![
            default(product(
                "ZswapCoinPublicKey",
                vec![member("different", Type::Field)],
            )),
            Expr::NativeWitnessCall {
                builtin: ir::NativeWitnessBuiltin::OwnPublicKey,
            },
        ],
    };
    let trace = Trace::default();
    let current = collected(|s, e, o| collect_expression_types(&source, s, e, o, Some(&trace)));
    let old = collected(|s, e, o| old_reference::collect_expression_types(&source, s, e, o));
    assert_eq!(current, old);
    assert_eq!(
        current.0,
        Err(RenderError::ConflictingStruct("ZswapCoinPublicKey".into()))
    );
    one(&trace, RefusalSite::unavailable());
}
