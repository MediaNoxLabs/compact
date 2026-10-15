use super::*;

#[test]
fn constructor_pure_witness_stateful_and_ledger_entries_capture_type_before_later_failure() {
    for lane in 0..5 {
        let mut source = contract();
        match lane {
            0 => {
                source.constructor = Some(Constructor {
                    source: None,
                    parameters: vec![Parameter {
                        name: "value".into(),
                        ty: invalid(),
                    }],
                    steps: vec![ConstructorStep::Expression {
                        value: Expr::Parameter {
                            name: "later".into(),
                        },
                    }],
                })
            }
            1 => source.circuits.push(pure(default(invalid()))),
            2 => source.witnesses.push(WitnessDeclaration {
                source: None,
                name: "read".into(),
                parameters: vec![Parameter {
                    name: "input".into(),
                    ty: invalid(),
                }],
                result: Type::Unit,
            }),
            3 => {
                let mut circuit = stateful();
                circuit.parameters.push(Parameter {
                    name: "input".into(),
                    ty: invalid(),
                });
                circuit.actions.push(StateAction::Expression {
                    value: Expr::Parameter {
                        name: "later".into(),
                    },
                });
                source.stateful_circuits.push(circuit);
            }
            4 => source.ledger_fields.push(LedgerField {
                source: None,
                id: "stored".into(),
                index: 0,
                path: vec![0],
                declaration: LedgerFieldKind::Cell { ty: invalid() },
            }),
            _ => unreachable!(),
        }
        let expected = match lane {
            0 => &source.constructor.as_ref().unwrap().parameters[0].ty,
            1 => {
                let Expr::Default { ty } = &source.circuits[0].body else {
                    panic!("fixture")
                };
                ty
            }
            2 => &source.witnesses[0].parameters[0].ty,
            3 => &source.stateful_circuits[0].parameters[0].ty,
            4 => {
                let LedgerFieldKind::Cell { ty } = &source.ledger_fields[0].declaration else {
                    panic!("fixture")
                };
                ty
            }
            _ => unreachable!(),
        };
        let trace = Trace::default();
        assert_eq!(
            same_public_error(&source, &trace),
            RenderError::InvalidIdentifier("crate".into())
        );
        one(&trace, RefusalSite::field(Owner::Type(expected), "name"));
    }
}

#[test]
fn projection_container_witness_argument_and_loop_annotation_capture_original_nested_types() {
    let cases = vec![
        Expr::StructField {
            value: Box::new(default(invalid())),
            field: "unused".into(),
            index: 99,
        },
        Expr::Tuple {
            elements: vec![Expr::Unit, default(invalid())],
        },
        Expr::WitnessCall {
            name: "not_reached".into(),
            arguments: vec![default(invalid())],
        },
        Expr::VectorMap {
            source: Box::new(Expr::Unit),
            parameter: Parameter {
                name: "item".into(),
                ty: invalid(),
            },
            body: Box::new(Expr::Unit),
            result: Type::Unit,
            length: 0,
        },
    ];
    for source in &cases {
        let expected = match source {
            Expr::StructField { value, .. } => {
                let Expr::Default { ty } = value.as_ref() else {
                    panic!("fixture")
                };
                ty
            }
            Expr::Tuple { elements } => {
                let Expr::Default { ty } = &elements[1] else {
                    panic!("fixture")
                };
                ty
            }
            Expr::WitnessCall { arguments, .. } => {
                let Expr::Default { ty } = &arguments[0] else {
                    panic!("fixture")
                };
                ty
            }
            Expr::VectorMap { parameter, .. } => &parameter.ty,
            _ => panic!("fixture"),
        };
        let trace = Trace::default();
        assert_eq!(
            collected(|s, e, o| collect_expression_types(source, s, e, o, Some(&trace))),
            collected(|s, e, o| old_reference::collect_expression_types(source, s, e, o))
        );
        one(&trace, RefusalSite::field(Owner::Type(expected), "name"));
    }
}

#[test]
fn constructor_action_and_effectful_return_keep_annotation_before_child_order() {
    let constructor = ConstructorStep::ForEach {
        binding: Parameter {
            name: "item".into(),
            ty: invalid(),
        },
        values: vec![default(enumeration("Later", &[]))],
        steps: vec![],
    };
    let ConstructorStep::ForEach { binding, .. } = &constructor else {
        panic!("fixture")
    };
    let trace = Trace::default();
    assert_eq!(
        collected(|s, e, o| collect_constructor_step_types(&constructor, s, e, o, Some(&trace))),
        collected(|s, e, o| old_reference::collect_constructor_step_types(&constructor, s, e, o))
    );
    one(&trace, RefusalSite::field(Owner::Type(&binding.ty), "name"));

    let action = StateAction::Let {
        bindings: vec![LocalBinding {
            name: "local".into(),
            ty: invalid(),
            value: default(enumeration("Later", &[])),
        }],
        action: Box::new(StateAction::Expression { value: Expr::Unit }),
    };
    let StateAction::Let { bindings, .. } = &action else {
        panic!("fixture")
    };
    let trace = Trace::default();
    assert_eq!(
        collected(|s, e, o| collect_action_types(&action, s, e, o, Some(&trace))),
        collected(|s, e, o| old_reference::collect_action_types(&action, s, e, o))
    );
    one(
        &trace,
        RefusalSite::field(Owner::Type(&bindings[0].ty), "name"),
    );

    let plan = ReturnPlan::Let {
        bindings: vec![LocalBinding {
            name: "local".into(),
            ty: invalid(),
            value: Expr::Unit,
        }],
        result: Box::new(ReturnPlan::Value {
            value: default(enumeration("Later", &[])),
        }),
    };
    let ReturnPlan::Let { bindings, .. } = &plan else {
        panic!("fixture")
    };
    let trace = Trace::default();
    assert_eq!(
        collected(|s, e, o| collect_return_plan_types(&plan, s, e, o, Some(&trace))),
        collected(|s, e, o| old_reference::collect_return_plan_types(&plan, s, e, o))
    );
    one(
        &trace,
        RefusalSite::field(Owner::Type(&bindings[0].ty), "name"),
    );
}

#[test]
fn successful_definitions_keep_exact_public_output_and_no_source_events() {
    let ty = product("Shared", vec![member("field", Type::Field)]);
    let mut source = contract();
    let mut circuit = pure(Expr::Unit);
    circuit.parameters = vec![
        Parameter {
            name: "first".into(),
            ty: ty.clone(),
        },
        Parameter {
            name: "second".into(),
            ty,
        },
    ];
    source.circuits.push(circuit);
    source.type_aliases.push(TypeAlias {
        source: None,
        name: "Alias".into(),
        ty: source.circuits[0].parameters[0].ty.clone(),
    });
    let bare = render_with_capabilities(&source).unwrap();
    let trace = Trace::default();
    let observed = render_with_native_refusal(&source, None, Some(&trace)).unwrap();
    assert_eq!(observed.source, bare.source);
    assert_eq!(
        serde_json::to_value(&observed.capabilities).unwrap(),
        serde_json::to_value(&bare.capabilities).unwrap()
    );
    assert!(trace.0.borrow().is_empty());
}
