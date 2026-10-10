// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;

#[test]
fn independent_owned_witness_validation_rechecks_identity_arity_types_and_scope() {
    let root = root(text());
    let witness = consumer(Type::OpaqueString);
    let original = checked(&root, &witness).unwrap().plan().clone();
    for mode in ["id", "arity", "future", "annotation", "result", "escape"] {
        let mut plan = original.clone();
        let binding = operation(&mut plan);
        let Operation::Witness {
            declaration,
            arguments,
        } = &mut binding.operation
        else {
            unreachable!()
        };
        match mode {
            "id" => *declaration = WitnessId(999),
            "arity" => arguments.clear(),
            "future" => arguments[0].id = usize::MAX,
            "annotation" => arguments[0].ty = Type::OpaqueBytes,
            "result" => binding.value.ty = Type::OpaqueString,
            "escape" => plan.body.result.id = usize::MAX,
            _ => unreachable!(),
        }
        assert!(
            validate::check(plan, authority(&root, &witness)).is_err(),
            "{mode}"
        );
    }
    let mut wrong_formal = witness.clone();
    wrong_formal.parameters[0].ty = Type::OpaqueBytes;
    assert!(validate::check(original.clone(), authority(&root, &wrong_formal)).is_err());
    let mut wrong_result = witness;
    wrong_result.result = Type::OpaqueString;
    let mut coordinated = original;
    operation(&mut coordinated).value.ty = Type::OpaqueString;
    assert!(validate::check(coordinated, authority(&root, &wrong_result)).is_err());
}

#[test]
fn changed_named_witness_formal_cannot_replace_the_original_canonical_environment() {
    let ty = structure(Type::OpaqueString);
    let root = root(Expr::StructLiteral {
        ty: ty.clone(),
        fields: vec![text()],
    });
    let witness = consumer(ty);
    let original = checked(&root, &witness).unwrap().plan().clone();
    for mode in ["member", "type", "name"] {
        let mut forged = witness.clone();
        let Type::Struct { name, fields } = &mut forged.parameters[0].ty else {
            unreachable!()
        };
        match mode {
            "member" => fields[0].name = "renamed".into(),
            "type" => fields[0].ty = Type::OpaqueBytes,
            "name" => *name = "Unknown".into(),
            _ => unreachable!(),
        }
        let originals = HashMap::from([("consume", &witness)]);
        let types = type_environment::TypeEnvironment::from_sources(
            &root,
            &HashMap::new(),
            &originals,
            &HashMap::new(),
            &HashMap::new(),
        )
        .unwrap();
        let declarations = declarations::Declarations::new(
            &HashMap::new(),
            &HashMap::from([("consume", &forged)]),
        )
        .with_type_environment(types)
        .with_root(&root);
        let gap = validate::check(original.clone(), declarations).unwrap_err();
        assert!(
            gap.detail.contains("canonical source type"),
            "{mode}: {gap:?}"
        );
    }
}

#[test]
fn owned_witness_formals_keep_static_resource_limits() {
    let root = root(text());
    let witness = consumer(Type::OpaqueString);
    let original = checked(&root, &witness).unwrap().plan().clone();
    let mut oversized = witness.clone();
    oversized.parameters[0].ty = Type::Vector {
        element: Box::new(Type::OpaqueBytes),
        length: usize::MAX,
    };
    let Err(PlanError::Unsupported(gap)) = checked(&root, &oversized) else {
        panic!("oversized formal must refuse")
    };
    assert_eq!(
        gap.detail,
        format!(
            "fixed aggregate layout exceeds {} slots",
            crate::resource_limits::recording_fixed_layout_limit()
        )
    );
    // Preserve original authoritative named definitions; the oversized formal
    // must be bounded by the independent validator, not by test setup.
    let original_witnesses = HashMap::from([("consume", &witness)]);
    let types = type_environment::TypeEnvironment::from_sources(
        &root,
        &HashMap::new(),
        &original_witnesses,
        &HashMap::new(),
        &HashMap::new(),
    )
    .unwrap();
    let declarations =
        declarations::Declarations::new(&HashMap::new(), &HashMap::from([("consume", &oversized)]))
            .with_type_environment(types)
            .with_root(&root);
    let gap = validate::check(original, declarations).unwrap_err();
    assert_eq!(
        gap.detail,
        format!(
            "fixed aggregate layout exceeds {} slots",
            crate::resource_limits::recording_fixed_layout_limit()
        )
    );
}

#[test]
fn owned_witness_arguments_cannot_capture_a_sibling_or_completed_branch() {
    let producer = WitnessDeclaration {
        source: None,
        name: "produce".into(),
        parameters: vec![],
        result: Type::OpaqueBytes,
    };
    let consumer = consumer(Type::OpaqueBytes);
    let bytes = structure(Type::OpaqueBytes);
    let mut root = root(Expr::StructField {
        value: Box::new(Expr::Default { ty: bytes }),
        field: "value".into(),
        index: 0,
    });
    root.actions = vec![StateAction::If {
        condition: Expr::Boolean { value: true },
        then: Box::new(StateAction::Expression {
            value: Expr::WitnessCall {
                name: "produce".into(),
                arguments: vec![],
            },
        }),
        otherwise: Box::new(StateAction::Sequence { actions: vec![] }),
    }];
    let witnesses = HashMap::from([("produce", &producer), ("consume", &consumer)]);
    let original = check(&root, &HashMap::new(), &witnesses)
        .unwrap()
        .plan()
        .clone();
    let environment = || {
        let types = type_environment::TypeEnvironment::from_sources(
            &root,
            &HashMap::new(),
            &witnesses,
            &HashMap::new(),
            &HashMap::new(),
        )
        .unwrap();
        declarations::Declarations::new(&HashMap::new(), &witnesses)
            .with_type_environment(types)
            .with_root(&root)
    };
    let mut forged = original.clone();
    let branch_index = forged
        .body
        .bindings
        .iter()
        .position(|b| matches!(b.operation, Operation::If { .. }))
        .unwrap();
    let Operation::If { then, .. } = &forged.body.bindings[branch_index].operation else {
        unreachable!()
    };
    let hidden = then
        .bindings
        .iter()
        .find(|b| matches!(b.operation, Operation::Witness { .. }))
        .unwrap()
        .value
        .clone();
    let Operation::Witness { arguments, .. } = &mut operation(&mut forged).operation else {
        unreachable!()
    };
    arguments[0] = hidden.clone();
    assert!(
        validate::check(forged, environment())
            .unwrap_err()
            .detail
            .contains("out of scope")
    );
    let mut sibling = original;
    let mut call = operation(&mut sibling).clone();
    call.value.id = usize::MAX - 1;
    let Operation::Witness { arguments, .. } = &mut call.operation else {
        unreachable!()
    };
    arguments[0] = hidden;
    let Operation::If { otherwise, .. } = &mut sibling.body.bindings[branch_index].operation else {
        unreachable!()
    };
    otherwise.bindings.push(call);
    assert!(
        validate::check(sibling, environment())
            .unwrap_err()
            .detail
            .contains("out of scope")
    );
}
