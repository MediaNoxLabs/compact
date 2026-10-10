// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Witness outputs have an encoding domain separate from public interfaces.
use super::*;

fn witness(result: Type) -> WitnessDeclaration {
    WitnessDeclaration {
        source: None,
        name: "observe".into(),
        parameters: vec![Parameter {
            name: "input".into(),
            ty: Type::Field,
        }],
        result,
    }
}
fn call(argument: Expr) -> Expr {
    Expr::WitnessCall {
        name: "observe".into(),
        arguments: vec![argument],
    }
}
fn field_value() -> Expr {
    Expr::FieldLiteral { value: "1".into() }
}
fn circuit() -> StatefulCircuit {
    StatefulCircuit {
        source: None,
        name: "root".into(),
        internal: false,
        parameters: vec![],
        actions: vec![StateAction::Expression {
            value: call(field_value()),
        }],
        result: Type::Unit,
        return_value: StateReturn::Unit,
    }
}
fn checked(
    root: &StatefulCircuit,
    declaration: &WitnessDeclaration,
) -> Result<CheckedPlan, PlanError> {
    check(
        root,
        &HashMap::new(),
        &HashMap::from([(declaration.name.as_str(), declaration)]),
    )
}
fn declarations(
    root: &StatefulCircuit,
    witness: &WitnessDeclaration,
) -> declarations::Declarations {
    let witnesses = HashMap::from([(witness.name.as_str(), witness)]);
    let types = type_environment::TypeEnvironment::from_sources(
        root,
        &HashMap::new(),
        &witnesses,
        &HashMap::new(),
        &HashMap::new(),
    )
    .unwrap();
    declarations::Declarations::new(&HashMap::new(), &witnesses)
        .with_type_environment(types)
        .with_root(root)
}
fn structure(member: Type) -> Type {
    Type::Struct {
        name: "Receipt".into(),
        fields: vec![StructField {
            name: "value".into(),
            ty: member,
        }],
    }
}
fn tuple(length: usize) -> Type {
    Type::Tuple {
        elements: vec![Type::Field; length],
    }
}
fn vector(element: Type) -> Type {
    Type::Vector {
        element: Box::new(element),
        length: 3,
    }
}
fn operation(plan: &mut Plan) -> &mut Binding {
    plan.body
        .bindings
        .iter_mut()
        .find(|b| matches!(b.operation, Operation::Witness { .. }))
        .unwrap()
}

#[test]
fn existing_and_legacy_witness_results_have_a_distinct_bounded_domain() {
    let root = circuit();
    let wide = Type::Unsigned {
        max: "340282366920938463463374607431768211456".into(),
    };
    for result in [
        Type::Unit,
        Type::Field,
        Type::Boolean,
        Type::JubjubPoint,
        Type::OpaqueBytes,
        Type::Bytes { length: 0 },
        Type::Bytes { length: 32 },
        wide.clone(),
        vector(wide),
        tuple(1),
        tuple(8),
        vector(tuple(8)),
        structure(tuple(8)),
        structure(vector(Type::Field)),
        structure(Type::Unit),
        Type::Enum {
            name: "Status".into(),
            variants: vec!["Ready".into()],
        },
    ] {
        let declaration = witness(result.clone());
        let plan = checked(&root, &declaration).unwrap();
        validate::check(plan.plan().clone(), declarations(&root, &declaration)).unwrap();
        render(&plan).unwrap();
        assert!(validate::witness_result_type(&result), "{result:?}");
    }
    for result in [
        Type::OpaqueString,
        vector(Type::OpaqueBytes),
        structure(Type::OpaqueBytes),
        tuple(0),
        tuple(9),
        vector(tuple(9)),
        structure(tuple(9)),
        Type::Unsigned {
            max: "not-a-bound".into(),
        },
    ] {
        assert!(matches!(
            checked(&root, &witness(result)),
            Err(PlanError::Unsupported(_))
        ));
    }
}

#[test]
fn owned_witness_arguments_still_match_exact_types_and_public_product_limits() {
    let mut declaration = witness(Type::OpaqueBytes);
    declaration.parameters[0].ty = Type::OpaqueBytes;
    // The call still supplies Field: owned argument eligibility cannot coerce it to Bytes.
    assert!(matches!(
        checked(&circuit(), &declaration),
        Err(PlanError::Unsupported(_))
    ));
    for ty in [
        Type::OpaqueBytes,
        tuple(2),
        vector(Type::Field),
        Type::Unsigned {
            max: "340282366920938463463374607431768211456".into(),
        },
    ] {
        let mut root = circuit();
        root.parameters.push(Parameter {
            name: "external".into(),
            ty: ty.clone(),
        });
        if ty == Type::OpaqueBytes {
            render(&checked(&root, &witness(Type::OpaqueBytes)).unwrap()).unwrap();
        } else {
            assert!(matches!(
                checked(&root, &witness(Type::OpaqueBytes)),
                Err(PlanError::Unsupported(_))
            ));
        }
        let mut root = circuit();
        root.result = ty.clone();
        root.return_value = StateReturn::Expression {
            value: call(field_value()),
        };
        if ty == Type::OpaqueBytes {
            render(&checked(&root, &witness(Type::OpaqueBytes)).unwrap()).unwrap();
        } else {
            assert!(matches!(
                checked(&root, &witness(Type::OpaqueBytes)),
                Err(PlanError::Unsupported(_))
            ));
        }
    }
}

#[test]
fn independent_validation_rechecks_result_declaration_and_argument_definitions() {
    let root = circuit();
    let declaration = witness(Type::OpaqueBytes);
    let original = checked(&root, &declaration).unwrap().plan().clone();
    for mode in [
        "missing",
        "result",
        "source_result",
        "both_results",
        "arity",
        "source_parameter",
        "future",
        "annotation",
        "definition",
    ] {
        let mut plan = original.clone();
        let mut source = declaration.clone();
        let binding = operation(&mut plan);
        let Operation::Witness {
            declaration: id,
            arguments,
        } = &mut binding.operation
        else {
            unreachable!()
        };
        match mode {
            "missing" => *id = WitnessId(99),
            "result" => binding.value.ty = Type::Field,
            "source_result" => source.result = Type::Field,
            "both_results" => {
                source.result = Type::OpaqueString;
                binding.value.ty = Type::OpaqueString;
            }
            "arity" => arguments.clear(),
            "source_parameter" => source.parameters[0].ty = Type::Boolean,
            "future" => arguments[0].id = usize::MAX,
            "annotation" => arguments[0].ty = Type::Boolean,
            "definition" => {
                let argument_id = arguments[0].id;
                arguments[0].ty = Type::Boolean;
                let definition = plan
                    .body
                    .bindings
                    .iter_mut()
                    .find(|b| b.value.id == argument_id)
                    .unwrap();
                definition.value.ty = Type::Boolean;
                definition.operation = Operation::Literal(Literal::Boolean(true));
            }
            _ => unreachable!(),
        }
        assert!(
            validate::check(plan, declarations(&root, &source)).is_err(),
            "{mode}"
        );
    }
}

#[test]
fn canonical_result_products_and_resources_are_checked_before_admission() {
    let root = circuit();
    let declaration = witness(structure(tuple(2)));
    let original = checked(&root, &declaration).unwrap().plan().clone();
    let mut plan = original.clone();
    let Type::Struct { fields, .. } = &mut operation(&mut plan).value.ty else {
        unreachable!()
    };
    fields[0].name = "forged".into();
    assert!(validate::check(plan, declarations(&root, &declaration)).is_err());
    let mut forged_source = declaration.clone();
    let Type::Struct { fields, .. } = &mut forged_source.result else {
        unreachable!()
    };
    fields[0].name = "forged".into();
    assert!(validate::check(original, declarations(&root, &forged_source)).is_err());
    let excessive = witness(Type::Bytes {
        length: crate::resource_limits::recording_byte_payload_limit() + 1,
    });
    assert!(matches!(
        checked(&root, &excessive),
        Err(PlanError::Unsupported(_))
    ));
}

#[test]
fn witness_arguments_cannot_reference_same_typed_sibling_branch_definitions() {
    let declaration = witness(Type::OpaqueBytes);
    let mut root = circuit();
    root.actions = vec![StateAction::If {
        condition: Expr::Boolean { value: true },
        then: Box::new(StateAction::Expression {
            value: call(field_value()),
        }),
        otherwise: Box::new(StateAction::Expression {
            value: call(field_value()),
        }),
    }];
    let mut plan = checked(&root, &declaration).unwrap().plan().clone();
    let branch = plan
        .body
        .bindings
        .iter_mut()
        .find(|b| matches!(b.operation, Operation::If { .. }))
        .unwrap();
    let Operation::If {
        then, otherwise, ..
    } = &mut branch.operation
    else {
        unreachable!()
    };
    let hidden = then
        .bindings
        .iter()
        .find_map(|b| match &b.operation {
            Operation::Witness { arguments, .. } => Some(arguments[0].clone()),
            _ => None,
        })
        .unwrap();
    let sibling = otherwise
        .bindings
        .iter_mut()
        .find(|b| matches!(b.operation, Operation::Witness { .. }))
        .unwrap();
    let Operation::Witness { arguments, .. } = &mut sibling.operation else {
        unreachable!()
    };
    arguments[0] = hidden;
    assert!(validate::check(plan, declarations(&root, &declaration)).is_err());
}

#[test]
fn forged_witness_declaration_cannot_replace_the_original_canonical_environment() {
    let root = circuit();
    let original = witness(structure(tuple(2)));
    let plan = checked(&root, &original).unwrap().plan().clone();
    let witnesses = HashMap::from([(original.name.as_str(), &original)]);
    let types = type_environment::TypeEnvironment::from_sources(
        &root,
        &HashMap::new(),
        &witnesses,
        &HashMap::new(),
        &HashMap::new(),
    )
    .unwrap();
    let mut forged = original.clone();
    let Type::Struct { fields, .. } = &mut forged.result else {
        unreachable!()
    };
    fields[0].name = "forged".into();
    // Keep the plan and canonical definitions untouched. Only the declaration
    // payload is replaced, so its own annotation must be independently checked.
    let declarations = declarations::Declarations::new(
        &HashMap::new(),
        &HashMap::from([(forged.name.as_str(), &forged)]),
    )
    .with_type_environment(types)
    .with_root(&root);
    let gap = validate::check(plan, declarations).unwrap_err();
    assert_eq!(gap.path, "actions[0].value");
    assert_eq!(gap.detail, "struct definition is not canonical source type");
}

#[test]
fn witness_result_vectors_charge_expanded_slots_and_nested_byte_payloads() {
    let root = circuit();
    let slots = crate::resource_limits::recording_fixed_layout_limit();
    let fields = |length| Type::Vector {
        element: Box::new(Type::Field),
        length,
    };
    checked(&root, &witness(fields(slots - 1))).unwrap();
    let Err(PlanError::Unsupported(gap)) = checked(&root, &witness(fields(slots))) else {
        panic!("oversized witness vector must be refused during lowering");
    };
    assert_eq!(gap.path, "actions[0].value");
    assert!(
        gap.detail.contains("fixed aggregate layout exceeds"),
        "{gap:?}"
    );

    let bytes = crate::resource_limits::recording_byte_payload_limit();
    let nested = |outer| Type::Vector {
        element: Box::new(Type::Vector {
            element: Box::new(Type::Bytes { length: bytes / 4 }),
            length: 2,
        }),
        length: outer,
    };
    checked(&root, &witness(nested(2))).unwrap();
    let Err(PlanError::Unsupported(gap)) = checked(&root, &witness(nested(3))) else {
        panic!("nested witness vector multiplicity must charge its byte leaves");
    };
    assert_eq!(gap.path, "actions[0].value");
    assert!(
        gap.detail.contains("fixed-byte aggregate payload exceeds"),
        "{gap:?}"
    );
}
