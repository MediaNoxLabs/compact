use super::*;

#[test]
fn let_name_precedes_initializer_and_scope_shadowing_restores_outer_keys() {
    let bad = ConstructorStep::Let {
        bindings: vec![LocalBinding {
            name: "crate".into(),
            ty: Type::Field,
            value: parameter("not_reached"),
        }],
        step: Box::new(ConstructorStep::Expression { value: Expr::Unit }),
    };
    let ConstructorStep::Let { bindings, .. } = &bad else {
        panic!("fixture")
    };
    let trace = Trace::default();
    assert_eq!(
        same_error(std::slice::from_ref(&bad), &HashMap::new(), &trace),
        RenderError::InvalidIdentifier("crate".into())
    );
    assert_eq!(
        trace.0.borrow().as_slice(),
        &[(
            "binding",
            std::ptr::from_ref(&bindings[0]) as usize,
            Slot::Field("name")
        )]
    );

    let sequence = ConstructorStep::Sequence {
        steps: vec![
            ConstructorStep::Let {
                bindings: vec![
                    LocalBinding {
                        name: "value".into(),
                        ty: Type::Field,
                        value: field("1"),
                    },
                    LocalBinding {
                        name: "value".into(),
                        ty: Type::Field,
                        value: parameter("value"),
                    },
                ],
                step: Box::new(ConstructorStep::Expression {
                    value: parameter("value"),
                }),
            },
            ConstructorStep::Expression {
                value: parameter("value"),
            },
        ],
    };
    let ConstructorStep::Sequence { steps } = &sequence else {
        panic!("fixture")
    };
    let ConstructorStep::Expression { value: outside } = &steps[1] else {
        panic!("fixture")
    };
    let trace = Trace::default();
    assert_eq!(
        same_error(std::slice::from_ref(&sequence), &HashMap::new(), &trace),
        RenderError::UnknownParameter("value".into())
    );
    first_expr(&trace, outside);
}

#[test]
fn foreach_vector_keeps_original_source_guards_and_no_new_binding_identifier_check() {
    let binding = Parameter {
        name: "crate".into(),
        ty: Type::Field,
    };
    for (source, expected) in [
        (
            field("1"),
            RenderError::InvalidConstructorInitializer("crate".into()),
        ),
        (
            parameter("absent"),
            RenderError::UnknownParameter("absent".into()),
        ),
    ] {
        let step = ConstructorStep::ForEachVector {
            binding: binding.clone(),
            source,
            length: 1,
            steps: vec![],
        };
        let trace = Trace::default();
        assert_eq!(
            same_error(std::slice::from_ref(&step), &HashMap::new(), &trace),
            expected
        );
        let ConstructorStep::ForEachVector { source, .. } = &step else {
            panic!("fixture ForEachVector")
        };
        first_expr(&trace, source);
        assert_eq!(trace.0.borrow().len(), 1, "direct guard, no extra fold");
    }
    let vector = Type::Vector {
        element: Box::new(Type::Field),
        length: 1,
    };
    let parameters = HashMap::from([("values", (&vector, syn::parse_quote!(values)))]);
    let step = ConstructorStep::ForEachVector {
        binding,
        source: parameter("values"),
        length: 1,
        steps: vec![ConstructorStep::Expression {
            value: parameter("crate"),
        }],
    };
    let trace = Trace::default();
    let bare = run(
        std::slice::from_ref(&step),
        &HashMap::new(),
        &parameters,
        &HashMap::new(),
        None,
    )
    .unwrap();
    let observed = run(
        std::slice::from_ref(&step),
        &HashMap::new(),
        &parameters,
        &HashMap::new(),
        Some(&trace),
    )
    .unwrap();
    assert_eq!(observed, bare);
    assert_eq!(observed.1, 8);
    assert!(
        trace.0.borrow().is_empty(),
        "loop binding has no old identifier check"
    );
}

#[test]
fn successful_recursive_steps_preserve_syntax_counters_and_original_pure_call_scope() {
    let stored = ledger("stored", LedgerFieldKind::Cell { ty: Type::Field });
    let fields = HashMap::from([("stored", &stored)]);
    let vector = Type::Vector {
        element: Box::new(Type::Field),
        length: 1,
    };
    let parameters = HashMap::from([("values", (&vector, syn::parse_quote!(values)))]);
    let global = PureCircuit {
        source: None,
        name: "global".into(),
        internal: false,
        parameters: vec![],
        result: Type::Field,
        body: field("1"),
    };
    let circuits = HashMap::from([("global", &global)]);
    let steps = vec![ConstructorStep::Sequence {
        steps: vec![
            ConstructorStep::Expression {
                value: Expr::Call {
                    name: "global".into(),
                    arguments: vec![],
                },
            },
            ConstructorStep::Let {
                bindings: vec![LocalBinding {
                    name: "local".into(),
                    ty: Type::Field,
                    value: field("1"),
                }],
                step: Box::new(ConstructorStep::CellWrite {
                    field: "stored".into(),
                    index: 0,
                    value: parameter("local"),
                }),
            },
            ConstructorStep::Assert {
                condition: Expr::Boolean { value: true },
                message: "ok".into(),
            },
            ConstructorStep::ForEach {
                binding: Parameter {
                    name: "item".into(),
                    ty: Type::Field,
                },
                values: vec![field("1")],
                steps: vec![ConstructorStep::Expression {
                    value: parameter("item"),
                }],
            },
            ConstructorStep::ForEachVector {
                binding: Parameter {
                    name: "item".into(),
                    ty: Type::Field,
                },
                source: parameter("values"),
                length: 1,
                steps: vec![ConstructorStep::Expression {
                    value: parameter("item"),
                }],
            },
            ConstructorStep::If {
                condition: Expr::Boolean { value: true },
                then_steps: vec![ConstructorStep::CellWrite {
                    field: "stored".into(),
                    index: 0,
                    value: field("1"),
                }],
                otherwise_steps: vec![ConstructorStep::CellWrite {
                    field: "stored".into(),
                    index: 0,
                    value: field("2"),
                }],
            },
        ],
    }];
    let trace = Trace::default();
    let observed = run(&steps, &fields, &parameters, &circuits, Some(&trace)).unwrap();
    assert_eq!(
        observed,
        run(&steps, &fields, &parameters, &circuits, None).unwrap()
    );
    assert_eq!(observed.1, 9);
    assert_eq!(observed.2, 10);
    assert!(trace.0.borrow().is_empty());
}
