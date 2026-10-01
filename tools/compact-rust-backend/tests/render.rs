use compact_rust_backend::ir::{
    Contract, CounterAmount, Expr, LedgerField, LedgerFieldKind, LocalBinding, Parameter,
    PureCircuit, StateAction, StateReturn, StatefulCircuit, StructField, Type, WitnessDeclaration,
};
use compact_rust_backend::{RenderError, render};

fn identity(result: Type, body: Expr) -> Contract {
    Contract {
        schema_version: 4,
        witnesses: vec![],
        ledger_fields: vec![],
        circuits: vec![PureCircuit {
            name: "identity".into(),
            parameters: vec![Parameter {
                name: "value".into(),
                ty: Type::Field,
            }],
            result,
            body,
        }],
        stateful_circuits: vec![],
    }
}

#[test]
fn emits_a_pure_field_circuit_as_parseable_rust() {
    let contract = identity(
        Type::Field,
        Expr::Parameter {
            name: "value".into(),
        },
    );
    let source = render(&contract).unwrap();
    syn::parse_file(&source).unwrap();
    assert!(source.contains("pub fn identity("));
    assert!(source.contains("value: runtime::Field"));
    assert!(source.contains("Ok(value)"));
}

#[test]
fn single_element_tuple_is_a_tuple_in_type_and_expression() {
    let contract = identity(
        Type::Tuple {
            elements: vec![Type::Field],
        },
        Expr::Tuple {
            elements: vec![Expr::Parameter {
                name: "value".into(),
            }],
        },
    );
    let source = render(&contract).unwrap();
    let file = syn::parse_file(&source).unwrap();
    let syn::Item::Mod(module) = &file.items[0] else {
        panic!("expected module")
    };
    let function = module
        .content
        .as_ref()
        .unwrap()
        .1
        .iter()
        .find_map(|item| match item {
            syn::Item::Fn(function) => Some(function),
            _ => None,
        })
        .expect("expected function");
    let syn::ReturnType::Type(_, result) = &function.sig.output else {
        panic!("expected return type")
    };
    let syn::Type::Path(result) = result.as_ref() else {
        panic!("expected Result")
    };
    let syn::PathArguments::AngleBracketed(arguments) =
        &result.path.segments.last().unwrap().arguments
    else {
        panic!("expected type arguments")
    };
    let syn::GenericArgument::Type(syn::Type::Tuple(tuple)) = &arguments.args[0] else {
        panic!("expected tuple")
    };
    assert_eq!(tuple.elems.len(), 1);
    assert!(tuple.elems.trailing_punct());
    assert!(source.contains("Ok((value,))"));
}

#[test]
fn rejects_bad_schema_and_unknown_references() {
    let mut contract = identity(
        Type::Field,
        Expr::Parameter {
            name: "value".into(),
        },
    );
    contract.schema_version = 2;
    assert_eq!(render(&contract), Err(RenderError::SchemaVersion(2)));
    contract.schema_version = 4;
    contract.circuits[0].body = Expr::Parameter {
        name: "missing".into(),
    };
    assert_eq!(
        render(&contract),
        Err(RenderError::UnknownParameter("missing".into()))
    );
}

#[test]
fn rejects_wrong_native_hash_opening_and_conversion_types() {
    let field = Expr::Parameter {
        name: "value".into(),
    };
    let contract = identity(
        Type::Bytes { length: 32 },
        Expr::PersistentCommit {
            value: Box::new(field.clone()),
            opening: Box::new(field.clone()),
        },
    );
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Bytes { length: 32 },
            actual: Type::Field,
        })
    );

    let contract = identity(
        Type::Field,
        Expr::DegradeToTransient {
            value: Box::new(field),
        },
    );
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Bytes { length: 32 },
            actual: Type::Field,
        })
    );
}

#[test]
fn rejects_field_as_jubjub_point_coordinate_input() {
    let contract = identity(
        Type::Field,
        Expr::JubjubPointX {
            value: Box::new(Expr::Parameter {
                name: "value".into(),
            }),
        },
    );
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::JubjubPoint,
            actual: Type::Field,
        })
    );
}

#[test]
fn rejects_type_mismatch_and_invalid_identifier() {
    let mut contract = identity(
        Type::Boolean,
        Expr::Parameter {
            name: "value".into(),
        },
    );
    assert!(matches!(
        render(&contract),
        Err(RenderError::TypeMismatch { .. })
    ));
    contract.circuits[0].name = "fn".into();
    assert_eq!(
        render(&contract),
        Err(RenderError::InvalidIdentifier("fn".into()))
    );
}

#[test]
fn unsigned_add_and_cast_reject_field_inputs() {
    let mut contract = identity(
        Type::Unsigned { max: "511".into() },
        Expr::UnsignedAdd {
            max: "511".into(),
            left: Box::new(Expr::Parameter {
                name: "value".into(),
            }),
            right: Box::new(Expr::UnsignedLiteral {
                value: "1".into(),
                max: "255".into(),
            }),
        },
    );
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Unsigned { max: "511".into() },
            actual: Type::Field,
        })
    );
    contract.circuits[0].body = Expr::UnsignedCast {
        max: "511".into(),
        value: Box::new(Expr::Parameter {
            name: "value".into(),
        }),
    };
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Unsigned { max: "511".into() },
            actual: Type::Field,
        })
    );
}

#[test]
fn unsigned_literal_checks_the_declared_maximum() {
    let mut contract = identity(
        Type::Unsigned { max: "255".into() },
        Expr::UnsignedLiteral {
            value: "255".into(),
            max: "255".into(),
        },
    );
    assert!(
        render(&contract)
            .unwrap()
            .contains("BoundedUint::<255>::new(255u128)")
    );
    contract.circuits[0].body = Expr::UnsignedLiteral {
        value: "256".into(),
        max: "255".into(),
    };
    assert_eq!(
        render(&contract),
        Err(RenderError::InvalidUnsignedLiteral {
            value: "256".into(),
            max: "255".into(),
        })
    );
}

#[test]
fn field_literal_requires_canonical_u128() {
    let mut contract = identity(Type::Field, Expr::FieldLiteral { value: "42".into() });
    assert!(
        render(&contract)
            .unwrap()
            .contains("runtime::Field::from(42u128)")
    );
    contract.circuits[0].body = Expr::FieldLiteral {
        value: "042".into(),
    };
    assert_eq!(
        render(&contract),
        Err(RenderError::InvalidFieldLiteral("042".into()))
    );
    contract.circuits[0].body = Expr::FieldLiteral {
        value: "340282366920938463463374607431768211456".into(),
    };
    assert_eq!(
        render(&contract),
        Err(RenderError::InvalidFieldLiteral(
            "340282366920938463463374607431768211456".into()
        ))
    );
}

#[test]
fn pure_call_checks_target_arity_and_argument_types() {
    let mut contract = identity(
        Type::Field,
        Expr::Call {
            name: "target".into(),
            arguments: vec![Expr::Parameter {
                name: "value".into(),
            }],
        },
    );
    assert_eq!(
        render(&contract),
        Err(RenderError::UnknownCircuit("target".into()))
    );
    contract.circuits.push(PureCircuit {
        name: "target".into(),
        parameters: vec![Parameter {
            name: "flag".into(),
            ty: Type::Boolean,
        }],
        result: Type::Field,
        body: Expr::Parameter {
            name: "flag".into(),
        },
    });
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Boolean,
            actual: Type::Field,
        })
    );
    contract.circuits[1].parameters[0].ty = Type::Field;
    if let Expr::Call { arguments, .. } = &mut contract.circuits[0].body {
        arguments.clear();
    }
    assert_eq!(
        render(&contract),
        Err(RenderError::ArgumentCount {
            circuit: "target".into(),
            expected: 1,
            actual: 0,
        })
    );
    if let Expr::Call { arguments, .. } = &mut contract.circuits[0].body {
        arguments.push(Expr::Parameter {
            name: "value".into(),
        });
    }
    assert!(
        render(&contract)
            .unwrap()
            .contains("crate::pure_circuits::target(value)?")
    );
}

#[test]
fn local_binding_checks_declared_type_and_scope() {
    let mut contract = identity(
        Type::Field,
        Expr::Let {
            bindings: vec![LocalBinding {
                name: "saved".into(),
                ty: Type::Boolean,
                value: Expr::Parameter {
                    name: "value".into(),
                },
            }],
            body: Box::new(Expr::Parameter {
                name: "saved".into(),
            }),
        },
    );
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Boolean,
            actual: Type::Field,
        })
    );
    let Expr::Let { bindings, .. } = &mut contract.circuits[0].body else {
        unreachable!()
    };
    bindings[0].ty = Type::Field;
    let source = render(&contract).unwrap();
    assert!(source.contains("let __compact_local_0: runtime::Field = value;"));
    assert!(source.contains("Ok({"));
}

#[test]
fn conditional_requires_boolean_condition_and_equal_branch_types() {
    let mut contract = identity(
        Type::Field,
        Expr::If {
            condition: Box::new(Expr::Parameter {
                name: "value".into(),
            }),
            then: Box::new(Expr::Parameter {
                name: "value".into(),
            }),
            otherwise: Box::new(Expr::Parameter {
                name: "value".into(),
            }),
        },
    );
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Boolean,
            actual: Type::Field,
        })
    );
    let Expr::If {
        condition,
        otherwise,
        ..
    } = &mut contract.circuits[0].body
    else {
        unreachable!()
    };
    *condition = Box::new(Expr::Boolean { value: true });
    *otherwise = Box::new(Expr::Boolean { value: false });
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Field,
            actual: Type::Boolean,
        })
    );
}

#[test]
fn refuses_to_add_a_boolean_to_a_field() {
    let contract = identity(
        Type::Field,
        Expr::Add {
            left: Box::new(Expr::Parameter {
                name: "value".into(),
            }),
            right: Box::new(Expr::Boolean { value: true }),
        },
    );
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Field,
            actual: Type::Boolean,
        })
    );
}

#[test]
fn rejects_noncanonical_or_unsupported_unsigned_maxima() {
    for max in ["08", "-1", "340282366920938463463374607431768211456"] {
        let contract = Contract {
            schema_version: 4,
            witnesses: vec![],
            ledger_fields: vec![],
            circuits: vec![PureCircuit {
                name: "id_u".into(),
                parameters: vec![Parameter {
                    name: "value".into(),
                    ty: Type::Unsigned { max: max.into() },
                }],
                result: Type::Unsigned { max: max.into() },
                body: Expr::Parameter {
                    name: "value".into(),
                },
            }],
            stateful_circuits: vec![],
        };
        assert_eq!(
            render(&contract),
            Err(RenderError::InvalidUnsignedMaximum(max.into()))
        );
    }
}

#[test]
fn unknown_json_fields_are_rejected() {
    let json = r#"{"schema_version":4,"ledger_fields":[],"circuits":[],"stateful_circuits":[],"rust_source":"panic!()"}"#;
    assert!(serde_json::from_str::<Contract>(json).is_err());

    let json = r#"{"schema_version":4,"ledger_fields":[{"id":"round","index":0,"declaration":{"kind":"counter"},"rust_source":"panic!()"}],"circuits":[],"stateful_circuits":[]}"#;
    assert!(serde_json::from_str::<Contract>(json).is_err());
}

#[test]
fn witness_calls_require_a_declared_witness_and_matching_signature() {
    let mut contract = Contract {
        schema_version: 4,
        ledger_fields: vec![],
        witnesses: vec![WitnessDeclaration {
            name: "secret".into(),
            parameters: vec![Parameter {
                name: "value".into(),
                ty: Type::Boolean,
            }],
            result: Type::Field,
        }],
        circuits: vec![],
        stateful_circuits: vec![StatefulCircuit {
            name: "read_secret".into(),
            parameters: vec![Parameter {
                name: "flag".into(),
                ty: Type::Boolean,
            }],
            actions: vec![],
            result: Type::Field,
            return_value: StateReturn::Expression {
                value: Expr::WitnessCall {
                    name: "secret".into(),
                    arguments: vec![Expr::Parameter {
                        name: "flag".into(),
                    }],
                },
            },
        }],
    };
    let source = render(&contract).unwrap();
    syn::parse_file(&source).unwrap();
    assert!(source.contains("pub trait Witnesses<Private>"));
    assert!(source.contains("private_transcript_outputs"));

    contract.witnesses.clear();
    assert_eq!(
        render(&contract),
        Err(RenderError::UnknownWitness("secret".into()))
    );
    contract.witnesses.push(WitnessDeclaration {
        name: "secret".into(),
        parameters: vec![],
        result: Type::Field,
    });
    assert_eq!(
        render(&contract),
        Err(RenderError::ArgumentCount {
            circuit: "secret".into(),
            expected: 0,
            actual: 1,
        })
    );
}

#[test]
fn state_action_must_reference_the_declared_ledger_field_and_index() {
    let mut contract = Contract {
        schema_version: 4,
        witnesses: vec![],
        ledger_fields: vec![LedgerField {
            id: "round".into(),
            index: 0,
            declaration: LedgerFieldKind::Counter,
        }],
        circuits: vec![],
        stateful_circuits: vec![StatefulCircuit {
            name: "increment".into(),
            parameters: vec![],
            result: Type::Unit,
            return_value: StateReturn::Unit,
            actions: vec![StateAction::CounterIncrement {
                field: "round".into(),
                index: 0,
                amount: CounterAmount::Literal { value: 1 },
            }],
        }],
    };
    let source = render(&contract).unwrap();
    assert!(source.contains("context.increment_counter(0, 1)?"));

    contract.stateful_circuits[0].actions[0] = StateAction::CounterDecrement {
        field: "round".into(),
        index: 0,
        amount: CounterAmount::Literal { value: 1 },
    };
    assert!(
        render(&contract)
            .unwrap()
            .contains("context.decrement_counter(0, 1)?")
    );
    contract.stateful_circuits[0].actions[0] = StateAction::CounterReset {
        field: "round".into(),
        index: 0,
    };
    assert!(
        render(&contract)
            .unwrap()
            .contains("context.write_cell(0, 0_u64)?")
    );

    contract.stateful_circuits[0].actions[0] = StateAction::CounterIncrement {
        field: "missing".into(),
        index: 0,
        amount: CounterAmount::Literal { value: 1 },
    };
    assert_eq!(
        render(&contract),
        Err(RenderError::UnknownLedgerField("missing".into()))
    );

    contract.stateful_circuits[0].actions[0] = StateAction::CounterIncrement {
        field: "round".into(),
        index: 1,
        amount: CounterAmount::Literal { value: 1 },
    };
    assert_eq!(
        render(&contract),
        Err(RenderError::UnknownLedgerField("round".into()))
    );

    contract.ledger_fields[0].index = 1;
    assert_eq!(render(&contract), Err(RenderError::InvalidLedgerIndex(1)));
}

#[test]
fn stateful_parameters_are_checked_before_cell_writes() {
    let mut contract = Contract {
        schema_version: 4,
        witnesses: vec![],
        ledger_fields: vec![LedgerField {
            id: "flag".into(),
            index: 0,
            declaration: LedgerFieldKind::Cell { ty: Type::Boolean },
        }],
        circuits: vec![],
        stateful_circuits: vec![StatefulCircuit {
            name: "set_flag".into(),
            parameters: vec![Parameter {
                name: "value".into(),
                ty: Type::Boolean,
            }],
            result: Type::Unit,
            return_value: StateReturn::Unit,
            actions: vec![StateAction::CellWrite {
                field: "flag".into(),
                index: 0,
                value: Expr::Parameter {
                    name: "value".into(),
                },
            }],
        }],
    };
    let source = render(&contract).unwrap();
    syn::parse_file(&source).unwrap();
    assert!(source.contains("__compact_param_0: bool"));
    assert!(source.contains("context.write_cell(0, __compact_param_0)?"));

    contract.stateful_circuits[0].parameters[0].ty = Type::Field;
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Boolean,
            actual: Type::Field,
        })
    );

    contract.stateful_circuits[0].parameters[0].ty = Type::Boolean;
    let duplicate = contract.stateful_circuits[0].parameters[0].clone();
    contract.stateful_circuits[0].parameters.push(duplicate);
    assert_eq!(
        render(&contract),
        Err(RenderError::DuplicateParameter("value".into()))
    );
}

#[test]
fn counter_parameter_requires_uint16_and_a_known_name() {
    let mut contract = Contract {
        schema_version: 4,
        witnesses: vec![],
        ledger_fields: vec![LedgerField {
            id: "round".into(),
            index: 0,
            declaration: LedgerFieldKind::Counter,
        }],
        circuits: vec![],
        stateful_circuits: vec![StatefulCircuit {
            name: "increment_by".into(),
            parameters: vec![Parameter {
                name: "amount".into(),
                ty: Type::Unsigned {
                    max: "65535".into(),
                },
            }],
            result: Type::Unit,
            return_value: StateReturn::Unit,
            actions: vec![StateAction::CounterIncrement {
                field: "round".into(),
                index: 0,
                amount: CounterAmount::Parameter {
                    name: "amount".into(),
                },
            }],
        }],
    };
    let source = render(&contract).unwrap();
    assert!(source.contains("__compact_param_0.value() as u16"));

    contract.stateful_circuits[0].parameters[0].ty = Type::Unsigned { max: "255".into() };
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Unsigned {
                max: "65535".into()
            },
            actual: Type::Unsigned { max: "255".into() },
        })
    );

    contract.stateful_circuits[0].parameters[0].ty = Type::Unsigned {
        max: "65535".into(),
    };
    contract.stateful_circuits[0].actions[0] = StateAction::CounterIncrement {
        field: "round".into(),
        index: 0,
        amount: CounterAmount::Parameter {
            name: "missing".into(),
        },
    };
    assert_eq!(
        render(&contract),
        Err(RenderError::UnknownParameter("missing".into()))
    );
}

#[test]
fn ledger_read_return_must_match_the_declared_cell() {
    let mut contract = Contract {
        schema_version: 4,
        witnesses: vec![],
        ledger_fields: vec![LedgerField {
            id: "flag".into(),
            index: 0,
            declaration: LedgerFieldKind::Cell { ty: Type::Boolean },
        }],
        circuits: vec![],
        stateful_circuits: vec![StatefulCircuit {
            name: "read_flag".into(),
            parameters: vec![],
            actions: vec![],
            result: Type::Boolean,
            return_value: StateReturn::CellRead {
                field: "flag".into(),
                index: 0,
            },
        }],
    };
    let source = render(&contract).unwrap();
    assert!(source.contains("context.read_cell::<bool>(0)?"));
    assert!(source.contains("let result = read_step.result"));

    contract.stateful_circuits[0].result = Type::Field;
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Field,
            actual: Type::Boolean,
        })
    );
    contract.stateful_circuits[0].result = Type::Boolean;
    contract.stateful_circuits[0].return_value = StateReturn::CellRead {
        field: "flag".into(),
        index: 1,
    };
    assert_eq!(
        render(&contract),
        Err(RenderError::UnknownLedgerField("flag".into()))
    );
}

#[test]
fn counter_read_returns_uint64() {
    let mut contract = Contract {
        schema_version: 4,
        witnesses: vec![],
        ledger_fields: vec![LedgerField {
            id: "round".into(),
            index: 0,
            declaration: LedgerFieldKind::Counter,
        }],
        circuits: vec![],
        stateful_circuits: vec![StatefulCircuit {
            name: "read_round".into(),
            parameters: vec![],
            actions: vec![],
            result: Type::Unsigned {
                max: u64::MAX.to_string(),
            },
            return_value: StateReturn::CounterRead {
                field: "round".into(),
                index: 0,
            },
        }],
    };
    let source = render(&contract).unwrap();
    assert!(source.contains("context.read_cell::<u64>(0)?"));

    contract.stateful_circuits[0].result = Type::Boolean;
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Unsigned {
                max: u64::MAX.to_string()
            },
            actual: Type::Boolean,
        })
    );
}

#[test]
fn set_actions_require_the_declared_element_type() {
    let mut contract = Contract {
        schema_version: 4,
        witnesses: vec![],
        ledger_fields: vec![LedgerField {
            id: "seen".into(),
            index: 0,
            declaration: LedgerFieldKind::Set { ty: Type::Boolean },
        }],
        circuits: vec![],
        stateful_circuits: vec![StatefulCircuit {
            name: "add".into(),
            parameters: vec![Parameter {
                name: "value".into(),
                ty: Type::Boolean,
            }],
            actions: vec![StateAction::SetInsert {
                field: "seen".into(),
                index: 0,
                value: Expr::Parameter {
                    name: "value".into(),
                },
            }],
            result: Type::Unit,
            return_value: StateReturn::Unit,
        }],
    };
    assert!(
        render(&contract)
            .unwrap()
            .contains("context.insert_set(0, __compact_param_0)?")
    );
    contract.stateful_circuits[0].parameters[0].ty = Type::Field;
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Boolean,
            actual: Type::Field,
        })
    );

    contract.stateful_circuits[0].parameters[0].ty = Type::Boolean;
    contract.stateful_circuits[0].actions = vec![StateAction::SetReset {
        field: "seen".into(),
        index: 0,
    }];
    assert!(render(&contract).unwrap().contains("context.reset_set(0)?"));
    contract.stateful_circuits[0].actions.clear();
    contract.stateful_circuits[0].return_value = StateReturn::SetSize {
        field: "seen".into(),
        index: 0,
    };
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Unsigned {
                max: u64::MAX.to_string()
            },
            actual: Type::Unit,
        })
    );
}

#[test]
fn map_insert_and_lookup_require_key_and_value_types() {
    let mut contract = Contract {
        schema_version: 4,
        witnesses: vec![],
        ledger_fields: vec![LedgerField {
            id: "table".into(),
            index: 0,
            declaration: LedgerFieldKind::Map {
                key: Type::Boolean,
                value: Type::Field,
            },
        }],
        circuits: vec![],
        stateful_circuits: vec![StatefulCircuit {
            name: "put".into(),
            parameters: vec![
                Parameter {
                    name: "key".into(),
                    ty: Type::Boolean,
                },
                Parameter {
                    name: "value".into(),
                    ty: Type::Field,
                },
            ],
            actions: vec![StateAction::MapInsert {
                field: "table".into(),
                index: 0,
                key: Expr::Parameter { name: "key".into() },
                value: Expr::Parameter {
                    name: "value".into(),
                },
            }],
            result: Type::Unit,
            return_value: StateReturn::Unit,
        }],
    };
    assert!(
        render(&contract)
            .unwrap()
            .contains("context.insert_map(0, __compact_param_0, __compact_param_1)?")
    );
    contract.stateful_circuits[0].parameters[1].ty = Type::Boolean;
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Field,
            actual: Type::Boolean,
        })
    );
    contract.stateful_circuits[0].parameters[1].ty = Type::Field;
    contract.stateful_circuits[0].actions.clear();
    contract.stateful_circuits[0].result = Type::Field;
    contract.stateful_circuits[0].return_value = StateReturn::MapLookup {
        field: "table".into(),
        index: 0,
        key: Expr::Parameter { name: "key".into() },
    };
    assert!(
        render(&contract)
            .unwrap()
            .contains("context.lookup_map::<_, runtime::Field>(0, __compact_param_0)?")
    );
    contract.stateful_circuits[0].result = Type::Boolean;
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Field,
            actual: Type::Boolean,
        })
    );
    contract.stateful_circuits[0].result = Type::Unit;
    contract.stateful_circuits[0].return_value = StateReturn::Unit;
    contract.stateful_circuits[0].actions = vec![StateAction::MapInsertDefault {
        field: "table".into(),
        index: 0,
        key: Expr::Parameter { name: "key".into() },
    }];
    assert!(
        render(&contract)
            .unwrap()
            .contains(".insert_map(0, __compact_param_0, <runtime::Field as Default>::default())?")
    );
    contract.stateful_circuits[0].parameters[0].ty = Type::Field;
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Boolean,
            actual: Type::Field,
        })
    );
    contract.stateful_circuits[0].parameters[0].ty = Type::Boolean;
    contract.stateful_circuits[0].actions = vec![StateAction::MapReset {
        field: "table".into(),
        index: 0,
    }];
    assert!(render(&contract).unwrap().contains("context.reset_map(0)?"));
    contract.stateful_circuits[0].actions.clear();
    contract.stateful_circuits[0].return_value = StateReturn::MapSize {
        field: "table".into(),
        index: 0,
    };
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Unsigned {
                max: u64::MAX.to_string()
            },
            actual: Type::Unit,
        })
    );
}

#[test]
fn list_push_front_and_length_validate_declared_types() {
    let mut contract = Contract {
        schema_version: 4,
        witnesses: vec![],
        ledger_fields: vec![LedgerField {
            id: "items".into(),
            index: 0,
            declaration: LedgerFieldKind::List { ty: Type::Field },
        }],
        circuits: vec![],
        stateful_circuits: vec![StatefulCircuit {
            name: "prepend".into(),
            parameters: vec![Parameter {
                name: "value".into(),
                ty: Type::Field,
            }],
            actions: vec![StateAction::ListPushFront {
                field: "items".into(),
                index: 0,
                value: Expr::Parameter {
                    name: "value".into(),
                },
            }],
            result: Type::Unit,
            return_value: StateReturn::Unit,
        }],
    };
    assert!(
        render(&contract)
            .unwrap()
            .contains("context.push_front_list(0, __compact_param_0)?")
    );
    contract.stateful_circuits[0].parameters[0].ty = Type::Boolean;
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Field,
            actual: Type::Boolean,
        })
    );
    contract.stateful_circuits[0].actions.clear();
    contract.stateful_circuits[0].return_value = StateReturn::ListLength {
        field: "items".into(),
        index: 0,
    };
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Unsigned {
                max: u64::MAX.to_string()
            },
            actual: Type::Unit,
        })
    );
    let maybe_field = Type::Struct {
        name: "Maybe".into(),
        fields: vec![
            StructField {
                name: "is_some".into(),
                ty: Type::Boolean,
            },
            StructField {
                name: "value".into(),
                ty: Type::Field,
            },
        ],
    };
    contract.stateful_circuits[0].return_value = StateReturn::ListHead {
        field: "items".into(),
        index: 0,
    };
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: maybe_field.clone(),
            actual: Type::Unit,
        })
    );
    contract.stateful_circuits[0].result = maybe_field;
    assert!(
        render(&contract)
            .unwrap()
            .contains("head_list::<runtime::Field, crate::types::Maybe>(0)?")
    );
}

#[test]
fn struct_definitions_are_shared_by_name_and_must_match() {
    let pair = Type::Struct {
        name: "Pair".into(),
        fields: vec![StructField {
            name: "amount".into(),
            ty: Type::Field,
        }],
    };
    let mut contract = Contract {
        schema_version: 4,
        witnesses: vec![],
        ledger_fields: vec![],
        circuits: vec![PureCircuit {
            name: "identity".into(),
            parameters: vec![Parameter {
                name: "value".into(),
                ty: pair.clone(),
            }],
            result: pair.clone(),
            body: Expr::Parameter {
                name: "value".into(),
            },
        }],
        stateful_circuits: vec![],
    };
    let source = render(&contract).unwrap();
    assert_eq!(source.matches("pub struct Pair").count(), 1);

    contract.circuits[0].result = Type::Struct {
        name: "Pair".into(),
        fields: vec![StructField {
            name: "active".into(),
            ty: Type::Boolean,
        }],
    };
    assert_eq!(
        render(&contract),
        Err(RenderError::ConflictingStruct("Pair".into()))
    );
}
