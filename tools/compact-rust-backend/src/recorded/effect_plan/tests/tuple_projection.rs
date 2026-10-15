// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Tuple projection is a bounded typed operation, not arbitrary indexing.
use super::*;

fn tuple() -> Type {
    Type::Tuple {
        elements: vec![Type::OpaqueBytes, Type::Field],
    }
}
fn reference(name: &str) -> Expr {
    Expr::Parameter { name: name.into() }
}
fn projection(value: Expr, index: usize) -> Expr {
    Expr::TupleIndex {
        value: Box::new(value),
        index,
    }
}
fn root(value: Expr) -> StatefulCircuit {
    StatefulCircuit {
        source: None,
        name: "project".into(),
        internal: false,
        parameters: vec![Parameter {
            name: "input".into(),
            ty: tuple(),
        }],
        actions: vec![StateAction::Expression { value }],
        result: Type::Boolean,
        return_value: StateReturn::Expression {
            value: Expr::Boolean { value: true },
        },
    }
}
fn checked(root: &StatefulCircuit) -> CheckedPlan {
    check(root, &HashMap::new(), &HashMap::new()).unwrap()
}
fn authority(root: &StatefulCircuit) -> declarations::Declarations {
    let types = type_environment::TypeEnvironment::from_sources(
        root,
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
    )
    .unwrap();
    declarations::Declarations::default()
        .with_type_environment(types)
        .with_root(root)
}
fn project_binding(plan: &mut Plan) -> &mut Binding {
    plan.body
        .bindings
        .iter_mut()
        .find(|b| matches!(b.operation, Operation::TupleProject { .. }))
        .unwrap()
}
#[test]
fn tuple_projection_retains_owned_members_for_later_use() {
    let mut source = root(projection(reference("input"), 0));
    source.actions.push(StateAction::Expression {
        value: projection(reference("input"), 0),
    });
    source.actions.push(StateAction::Expression {
        value: projection(reference("input"), 1),
    });
    let plan = checked(&source);
    let bindings = plan
        .plan()
        .body
        .bindings
        .iter()
        .filter(|b| matches!(b.operation, Operation::TupleProject { .. }))
        .collect::<Vec<_>>();
    assert_eq!(bindings.len(), 3);
    assert_eq!(bindings[0].value.ty, Type::OpaqueBytes);
    assert_eq!(bindings[2].value.ty, Type::Field);
    for binding in bindings {
        let Operation::TupleProject { value, .. } = &binding.operation else {
            unreachable!()
        };
        assert_eq!(value, &plan.plan().parameters[0]);
    }
    validate::check(plan.plan().clone(), authority(&source)).unwrap();
    let rendered = render(&plan).unwrap();
    let steps = rendered.steps;
    let syntax: syn::Block = syn::parse_quote!({ #(#steps)* });
    let text = quote::quote!(#syntax).to_string();
    assert_eq!(
        text.matches("clone").count(),
        2,
        "owned projected leaves must be retained"
    );
}

#[test]
fn tuple_projection_operand_witness_executes_once_before_each_projection() {
    let result = Type::Tuple {
        elements: vec![Type::Field, Type::Boolean],
    };
    let witness = WitnessDeclaration {
        source: None,
        name: "pair".into(),
        parameters: vec![],
        result,
    };
    let source = root(projection(
        Expr::WitnessCall {
            name: "pair".into(),
            arguments: vec![],
        },
        1,
    ));
    let plan = check(
        &source,
        &HashMap::new(),
        &HashMap::from([("pair", &witness)]),
    )
    .unwrap();
    let operations = &plan.plan().body.bindings;
    assert!(matches!(operations[0].operation, Operation::Witness { .. }));
    let Operation::TupleProject { value, index } = &operations[1].operation else {
        panic!("projection after witness")
    };
    assert_eq!(value, &operations[0].value);
    assert_eq!(*index, 1);
    assert_eq!(
        operations
            .iter()
            .filter(|b| matches!(b.operation, Operation::Witness { .. }))
            .count(),
        1
    );
    render(&plan).unwrap();
}

#[test]
fn tuple_projection_source_refuses_wrong_kind_and_out_of_range_index() {
    for (ty, index, expected) in [
        (tuple(), 2, "tuple projection index is outside its arity"),
        (
            tuple(),
            usize::MAX,
            "tuple projection index is outside its arity",
        ),
        (
            Type::Tuple { elements: vec![] },
            0,
            "tuple projection index is outside its arity",
        ),
        (
            Type::Vector {
                element: Box::new(Type::Field),
                length: 2,
            },
            0,
            "tuple projection requires a tuple value",
        ),
        (Type::Field, 0, "tuple projection requires a tuple value"),
    ] {
        let mut source = root(projection(reference("input"), index));
        source.parameters[0].ty = ty;
        let Err(PlanError::Unsupported(gap)) = check(&source, &HashMap::new(), &HashMap::new())
        else {
            panic!("invalid projection")
        };
        assert_eq!(gap.detail, expected);
    }
}

#[test]
fn independent_tuple_projection_rechecks_operand_type_index_result_and_scope() {
    let source = root(projection(reference("input"), 0));
    let original = checked(&source).plan().clone();
    for mutation in 0..5 {
        let mut forged = original.clone();
        let binding = project_binding(&mut forged);
        let Operation::TupleProject { value, index } = &mut binding.operation else {
            unreachable!()
        };
        match mutation {
            0 => *index = usize::MAX,
            1 => *index = 1, // in range, but declared result remains opaque bytes
            2 => {
                value.ty = Type::Vector {
                    element: Box::new(Type::OpaqueBytes),
                    length: 2,
                }
            }
            3 => value.id = usize::MAX,
            4 => binding.value.ty = Type::OpaqueString,
            _ => unreachable!(),
        }
        assert!(
            validate::check(forged, authority(&source)).is_err(),
            "mutation {mutation}"
        );
    }
    // Supply a genuine in-scope vector rather than only forging an annotation.
    let vector_type = Type::Vector {
        element: Box::new(Type::OpaqueBytes),
        length: 2,
    };
    let mut source = source;
    source.parameters[0].ty = vector_type.clone();
    source.actions.clear();
    let mut forged = checked(&source).plan().clone();
    forged.body.bindings.insert(
        0,
        Binding {
            value: Value {
                id: 99,
                ty: Type::OpaqueBytes,
            },
            operation: Operation::TupleProject {
                value: Value {
                    id: 0,
                    ty: vector_type,
                },
                index: 0,
            },
            path: "forged.vector".into(),
        },
    );
    assert!(validate::check(forged, authority(&source)).is_err());
}
