use super::*;

#[test]
fn all_four_state_expression_sites_capture_first_actual_child() {
    let target = ledger("stored", LedgerFieldKind::Cell { ty: Type::Field });
    let fields = HashMap::from([("stored", &target)]);
    let bad = || Expr::Add {
        left: Box::new(parameter("first")),
        right: Box::new(parameter("later")),
    };
    let steps = [
        ConstructorStep::Expression { value: bad() },
        ConstructorStep::Let {
            bindings: vec![LocalBinding {
                name: "local".into(),
                ty: Type::Field,
                value: bad(),
            }],
            step: Box::new(ConstructorStep::Expression { value: Expr::Unit }),
        },
        ConstructorStep::Assert {
            condition: bad(),
            message: "assert".into(),
        },
        ConstructorStep::CellWrite {
            field: "stored".into(),
            index: 0,
            value: bad(),
        },
    ];
    for step in &steps {
        let original = match step {
            ConstructorStep::Expression { value } | ConstructorStep::CellWrite { value, .. } => {
                value
            }
            ConstructorStep::Assert { condition, .. } => condition,
            ConstructorStep::Let { bindings, .. } => &bindings[0].value,
            _ => panic!("fixture"),
        };
        let Expr::Add { left, right } = original else {
            panic!("fixture Add")
        };
        let trace = Trace::default();
        assert_eq!(
            same_error(std::slice::from_ref(step), &fields, &trace),
            RenderError::UnknownParameter("first".into())
        );
        first_expr(&trace, left);
        assert!(
            trace
                .0
                .borrow()
                .iter()
                .all(|(_, pointer, _)| *pointer != std::ptr::from_ref(right.as_ref()) as usize)
        );
    }
}

#[test]
fn restricted_seven_pure_sites_keep_guards_empty_callees_and_original_operands() {
    let set = ledger("set", LedgerFieldKind::Set { ty: Type::Field });
    let list = ledger("list", LedgerFieldKind::List { ty: Type::Field });
    let map = ledger(
        "map",
        LedgerFieldKind::Map {
            key: Type::Field,
            value: Type::Field,
        },
    );
    let fields = HashMap::from([("set", &set), ("list", &list), ("map", &map)]);
    let cases = [
        ConstructorStep::SetInsert {
            field: "set".into(),
            index: 0,
            value: parameter("missing"),
        },
        ConstructorStep::ListPushFront {
            field: "list".into(),
            index: 0,
            value: parameter("missing"),
        },
        ConstructorStep::MapInsert {
            field: "map".into(),
            index: 0,
            key: parameter("missing"),
            value: field("1"),
        },
        ConstructorStep::MapInsert {
            field: "map".into(),
            index: 0,
            key: field("1"),
            value: parameter("missing"),
        },
        ConstructorStep::MapRemove {
            field: "map".into(),
            index: 0,
            key: parameter("missing"),
        },
        ConstructorStep::ForEach {
            binding: Parameter {
                name: "item".into(),
                ty: Type::Field,
            },
            values: vec![parameter("missing")],
            steps: vec![],
        },
        ConstructorStep::If {
            condition: parameter("missing"),
            then_steps: vec![],
            otherwise_steps: vec![],
        },
    ];
    for step in &cases {
        let failed = match step {
            ConstructorStep::SetInsert { value, .. }
            | ConstructorStep::ListPushFront { value, .. } => value,
            ConstructorStep::MapInsert { key, value, .. } => {
                if matches!(key, Expr::Parameter { .. }) {
                    key
                } else {
                    value
                }
            }
            ConstructorStep::MapRemove { key, .. } => key,
            ConstructorStep::ForEach { values, .. } => &values[0],
            ConstructorStep::If { condition, .. } => condition,
            _ => panic!("fixture"),
        };
        let trace = Trace::default();
        assert_eq!(
            same_error(std::slice::from_ref(step), &fields, &trace),
            RenderError::UnknownParameter("missing".into())
        );
        first_expr(&trace, failed);
    }
    let global = PureCircuit {
        source: None,
        name: "global".into(),
        internal: false,
        parameters: vec![],
        result: Type::Field,
        body: field("1"),
    };
    let circuits = HashMap::from([("global", &global)]);
    let call = Expr::Call {
        name: "global".into(),
        arguments: vec![],
    };
    let guarded = ConstructorStep::SetInsert {
        field: "set".into(),
        index: 0,
        value: call.clone(),
    };
    let trace = Trace::default();
    let bare = run(
        std::slice::from_ref(&guarded),
        &fields,
        &HashMap::new(),
        &circuits,
        None,
    )
    .unwrap_err();
    assert_eq!(
        bare,
        RenderError::InvalidConstructorInitializer("set".into())
    );
    assert_eq!(
        run(
            std::slice::from_ref(&guarded),
            &fields,
            &HashMap::new(),
            &circuits,
            Some(&trace)
        )
        .unwrap_err(),
        bare
    );
    let ConstructorStep::SetInsert { value, .. } = &guarded else {
        panic!("fixture")
    };
    assert_eq!(
        trace.0.borrow().as_slice(),
        &[("expr", std::ptr::from_ref(value) as usize, Slot::Node)]
    );
    // A separate pure_expression invocation has its own observer trace.
    let trace = Trace::default();
    assert_eq!(
        super::super::pure_expression(&call, &HashMap::new(), Some(&trace))
            .err()
            .unwrap(),
        RenderError::UnknownCircuit("global".into())
    );
    first_expr(&trace, &call);
}

#[test]
fn cell_target_guards_capture_original_sites_and_other_pending_guards_remain_unmapped() {
    let cell = ledger("stored", LedgerFieldKind::Cell { ty: Type::Field });
    let map = ledger(
        "map",
        LedgerFieldKind::Map {
            key: Type::Field,
            value: Type::Field,
        },
    );
    let fields = HashMap::from([("stored", &cell), ("map", &map)]);
    let cases = [
        (
            ConstructorStep::CellWrite {
                field: "absent".into(),
                index: 0,
                value: parameter("later"),
            },
            RenderError::UnknownLedgerField("absent".into()),
        ),
        (
            ConstructorStep::CellWrite {
                field: "stored".into(),
                index: 1,
                value: parameter("later"),
            },
            RenderError::InvalidLedgerIndex(1),
        ),
        (
            ConstructorStep::CellWrite {
                field: "map".into(),
                index: 0,
                value: parameter("later"),
            },
            RenderError::InvalidConstructorInitializer("map".into()),
        ),
        (
            ConstructorStep::MapInsert {
                field: "map".into(),
                index: 0,
                key: Expr::Boolean { value: true },
                value: parameter("later"),
            },
            RenderError::TypeMismatch {
                expected: Type::Field,
                actual: Type::Boolean,
            },
        ),
        (
            ConstructorStep::Assert {
                condition: field("1"),
                message: "assert".into(),
            },
            RenderError::TypeMismatch {
                expected: Type::Boolean,
                actual: Type::Field,
            },
        ),
    ];
    for (step, expected) in &cases {
        let trace = Trace::default();
        assert_eq!(
            &same_error(std::slice::from_ref(step), &fields, &trace),
            expected
        );
        if let ConstructorStep::CellWrite { index, .. } = step {
            let slot = if *index == 1 {
                Slot::Field("index")
            } else {
                Slot::Field("field")
            };
            assert_eq!(
                trace.0.borrow().as_slice(),
                &[("step", std::ptr::from_ref(step) as usize, slot)]
            );
        } else if let ConstructorStep::Assert { condition, .. } = step {
            first_expr(&trace, condition);
            assert_eq!(trace.0.borrow().len(), 1, "existing assertion type guard");
        } else if let ConstructorStep::MapInsert { key, .. } = step {
            assert_eq!(
                trace.0.borrow().as_slice(),
                &[("expr", std::ptr::from_ref(key) as usize, Slot::Node)]
            );
        } else {
            assert!(
                trace.0.borrow().is_empty(),
                "unrelated pending guards stay unmapped"
            );
        }
    }
}
