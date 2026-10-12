// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Owned local and Cell policies are independent of recorded external interfaces.
use super::*;
#[path = "owned_values/coercion.rs"]
mod coercion;
#[path = "owned_values/empty.rs"]
mod empty;
#[path = "owned_values/validation.rs"]
mod validation;
fn param(name: &str) -> Expr {
    Expr::Parameter { name: name.into() }
}
fn boolean() -> Expr {
    Expr::Boolean { value: true }
}
fn field(n: u64) -> Expr {
    Expr::FieldLiteral {
        value: n.to_string(),
    }
}
fn vector(element: Type, length: usize) -> Type {
    Type::Vector {
        element: Box::new(element),
        length,
    }
}
fn tuple(elements: Vec<Type>) -> Type {
    Type::Tuple { elements }
}
fn construct_vector(values: Vec<Expr>, element: Type) -> Expr {
    Expr::Coerce {
        ty: vector(element, values.len()),
        value: Box::new(Expr::Tuple { elements: values }),
    }
}
fn local(name: &str, ty: Type, value: Expr, body: Expr) -> Expr {
    Expr::Let {
        bindings: vec![LocalBinding {
            name: name.into(),
            ty,
            value,
        }],
        body: Box::new(body),
    }
}
fn eq(left: Expr, right: Expr) -> Expr {
    Expr::Equal {
        left: Box::new(left),
        right: Box::new(right),
    }
}
fn root(value: Expr) -> StatefulCircuit {
    StatefulCircuit {
        source: None,
        name: "root".into(),
        internal: false,
        parameters: vec![],
        actions: vec![],
        result: Type::Boolean,
        return_value: StateReturn::Expression { value },
    }
}
fn check_root(circuit: &StatefulCircuit, pure: &[PureCircuit]) -> Result<CheckedPlan, PlanError> {
    check_with_calls(
        circuit,
        &HashMap::new(),
        &HashMap::new(),
        &pure.iter().map(|c| (c.name.as_str(), c)).collect(),
        &HashMap::new(),
    )
}
fn source_contract(circuit: StatefulCircuit, pure: Vec<PureCircuit>) -> Contract {
    Contract {
        schema_version: SCHEMA_VERSION,
        type_aliases: vec![],
        ledger_fields: vec![],
        constructor: None,
        witnesses: vec![],
        circuits: pure,
        stateful_circuits: vec![circuit],
    }
}
fn accept(value: Expr, pure: Vec<PureCircuit>) {
    let circuit = root(value);
    let checked = check_root(&circuit, &pure).unwrap();
    render(&checked).unwrap();
    crate::render_with_capabilities(&source_contract(circuit, pure)).unwrap();
}
fn identity(name: &str, ty: Type) -> PureCircuit {
    PureCircuit {
        source: None,
        name: name.into(),
        internal: true,
        parameters: vec![Parameter {
            name: "input".into(),
            ty: ty.clone(),
        }],
        result: ty,
        body: param("input"),
    }
}
fn call(name: &str, arguments: Vec<Expr>) -> Expr {
    Expr::Call {
        name: name.into(),
        arguments,
    }
}
fn carrier() -> Type {
    Type::Struct {
        name: "OwnedCarrier".into(),
        fields: vec![
            StructField {
                name: "text".into(),
                ty: Type::OpaqueString,
            },
            StructField {
                name: "bytes".into(),
                ty: Type::OpaqueBytes,
            },
            StructField {
                name: "values".into(),
                ty: vector(Type::Field, 2),
            },
        ],
    }
}
fn project(value: Expr, name: &str, index: usize) -> Expr {
    Expr::StructField {
        value: Box::new(value),
        field: name.into(),
        index,
    }
}
#[test]
fn owned_projection_pure_arguments_and_repeated_aggregate_uses_render() {
    let ty = carrier();
    let text = project(param("held"), "text", 0);
    let expression = local(
        "held",
        ty.clone(),
        Expr::Default { ty: ty.clone() },
        local(
            "text",
            Type::OpaqueString,
            call("text_identity", vec![text.clone()]),
            eq(param("text"), call("text_identity", vec![text])),
        ),
    );
    accept(
        expression,
        vec![identity("text_identity", Type::OpaqueString)],
    );
    for (index, name, ty) in [
        (1, "bytes", Type::OpaqueBytes),
        (2, "values", vector(Type::Field, 2)),
    ] {
        let carrier = carrier();
        let projected = project(param("held"), name, index);
        accept(
            local(
                "held",
                carrier.clone(),
                Expr::Default { ty: carrier },
                eq(
                    call("identity", vec![projected.clone()]),
                    call("identity", vec![projected]),
                ),
            ),
            vec![identity("identity", ty)],
        );
    }
}
#[test]
fn owned_vector_tuple_and_wide_joins_and_pure_interfaces_are_explicitly_supported() {
    let wide = Type::Unsigned {
        max: "680564733841876926926749214863536422911".into(),
    };
    let wide_value = Expr::UnsignedLiteral {
        value: "340282366920938463463374607431768211456".into(),
        max: "680564733841876926926749214863536422911".into(),
    };
    let owned = project(Expr::Default { ty: carrier() }, "text", 0);
    let cases = vec![
        (Type::OpaqueString, owned),
        (
            vector(Type::Field, 2),
            construct_vector(vec![field(1), field(2)], Type::Field),
        ),
        (
            tuple(vec![Type::Field, Type::Boolean]),
            Expr::Tuple {
                elements: vec![field(3), boolean()],
            },
        ),
        (wide, wide_value),
    ];
    for (ty, value) in cases {
        let selected = Expr::If {
            condition: Box::new(boolean()),
            then: Box::new(value.clone()),
            otherwise: Box::new(value),
        };
        accept(
            local(
                "selected",
                ty.clone(),
                selected,
                eq(call("identity", vec![param("selected")]), param("selected")),
            ),
            vec![identity("identity", ty)],
        );
    }
}
#[test]
fn standalone_large_tuple_value_and_pure_interface_do_not_claim_equality() {
    let ty = tuple(vec![Type::Field; 13]);
    let value = Expr::Tuple {
        elements: vec![field(1); 13],
    };
    accept(
        local(
            "large",
            ty.clone(),
            call("identity", vec![value]),
            boolean(),
        ),
        vec![identity("identity", ty.clone())],
    );
    assert!(!validate::equality_type(&ty));
}
#[test]
fn opaque_default_stays_in_certified_pure_body_or_named_struct() {
    let pure = PureCircuit {
        source: None,
        name: "empty".into(),
        internal: true,
        parameters: vec![],
        result: Type::OpaqueString,
        body: Expr::Default {
            ty: Type::OpaqueString,
        },
    };
    accept(
        local(
            "text",
            Type::OpaqueString,
            call("empty", vec![]),
            eq(param("text"), param("text")),
        ),
        vec![pure],
    );
    assert!(matches!(
        check_root(
            &root(local(
                "text",
                Type::OpaqueString,
                Expr::Default {
                    ty: Type::OpaqueString
                },
                boolean()
            )),
            &[]
        ),
        Err(PlanError::Unsupported(_))
    ));
}
#[test]
fn owned_public_roots_keep_storage_and_witness_result_boundaries() {
    for ty in [
        Type::OpaqueString,
        Type::OpaqueBytes,
        vector(Type::Field, 2),
        carrier(),
        empty::empty_type(),
    ] {
        assert!(validate::value_type(&ty));
        assert!(!validate::surface_type(&ty));
        let mut parameter_root = root(boolean());
        parameter_root.parameters.push(Parameter {
            name: "owned".into(),
            ty: ty.clone(),
        });
        render(&check_root(&parameter_root, &[]).unwrap()).unwrap();
        let mut valid_result = parameter_root.clone();
        valid_result.result = ty.clone();
        valid_result.return_value = StateReturn::Expression {
            value: param("owned"),
        };
        render(&check_root(&valid_result, &[]).unwrap()).unwrap();
        // A Boolean body still cannot satisfy an opaque or product result.
        // Preserve this malformed-result control independently of root eligibility.
        let mut result_root = root(boolean());
        result_root.result = ty.clone();
        assert!(matches!(
            check_root(&result_root, &[]),
            Err(PlanError::Unsupported(_))
        ));
        let cell = LedgerField {
            source: None,
            id: "owned".into(),
            index: 0,
            path: vec![0],
            declaration: LedgerFieldKind::Cell { ty: ty.clone() },
        };
        let expr = local(
            "held",
            ty.clone(),
            Expr::CellRead {
                field: "owned".into(),
                index: 0,
            },
            boolean(),
        );
        let storage = check(
            &root(expr),
            &HashMap::from([("owned", &cell)]),
            &HashMap::new(),
        );
        if matches!(ty, Type::OpaqueString | Type::OpaqueBytes) {
            // Owned Cell reads now have a separately qualified storage domain.
            render(&storage.unwrap()).unwrap();
        } else {
            // The carrier contains a Vector; vectors and empty products remain excluded.
            assert!(matches!(storage, Err(PlanError::Unsupported(_))));
        }
        let witness = WitnessDeclaration {
            source: None,
            name: "owned".into(),
            parameters: vec![],
            result: ty.clone(),
        };
        let expr = local(
            "held",
            ty.clone(),
            Expr::WitnessCall {
                name: "owned".into(),
                arguments: vec![],
            },
            boolean(),
        );
        let observed = check(
            &root(expr),
            &HashMap::new(),
            &HashMap::from([("owned", &witness)]),
        );
        // Aligned witness outputs include these owned products, independently
        // of the narrower storage domain checked above.
        render(&observed.unwrap()).unwrap();
        // Default expressions are only supported for named structures. Use
        // already qualified local expressions for opaque and vector values.
        let value = match &ty {
            Type::OpaqueString => project(Expr::Default { ty: carrier() }, "text", 0),
            Type::OpaqueBytes => project(Expr::Default { ty: carrier() }, "bytes", 1),
            Type::Vector { .. } => construct_vector(vec![field(1), field(2)], Type::Field),
            _ => Expr::Default { ty: ty.clone() },
        };
        let helper = StatefulCircuit {
            source: None,
            name: "helper".into(),
            internal: true,
            parameters: vec![],
            actions: vec![],
            result: ty.clone(),
            return_value: StateReturn::Expression { value },
        };
        let expr = local("held", ty, call("helper", vec![]), boolean());
        render(
            &check_with_calls(
                &root(expr),
                &HashMap::new(),
                &HashMap::new(),
                &HashMap::new(),
                &HashMap::from([("helper", &helper)]),
            )
            .unwrap(),
        )
        .unwrap();
    }
}
#[test]
fn vector_literals_are_exact_typed_ordered_and_include_empty_values() {
    for length in [0, 1, 2] {
        let ty = vector(Type::Field, length);
        accept(
            local(
                "v",
                ty,
                Expr::Vector {
                    element: Type::Field,
                    elements: (0..length).map(|n| field(n as u64)).collect(),
                },
                eq(param("v"), param("v")),
            ),
            vec![],
        );
    }
    let wrong = Expr::Vector {
        element: Type::Unsigned {
            max: "65535".into(),
        },
        elements: vec![Expr::UnsignedLiteral {
            value: "1".into(),
            max: "255".into(),
        }],
    };
    assert!(matches!(
        check_root(
            &root(local(
                "v",
                vector(
                    Type::Unsigned {
                        max: "65535".into()
                    },
                    1
                ),
                wrong,
                boolean()
            )),
            &[]
        ),
        Err(PlanError::Unsupported(_))
    ));
}
#[test]
fn complete_retained_owned_value_source_uses_checked_operations() {
    let contract: Contract = serde_json::from_str(include_str!(
        "../../../../tests/effect-plan-owned-values-schema20-ir.json"
    ))
    .unwrap();
    let pure = contract
        .circuits
        .iter()
        .map(|p| (p.name.as_str(), p))
        .collect();
    let ledger = contract
        .ledger_fields
        .iter()
        .map(|f| (f.id.as_str(), f))
        .collect();
    let witnesses = contract
        .witnesses
        .iter()
        .map(|w| (w.name.as_str(), w))
        .collect();
    let calls = contract
        .stateful_circuits
        .iter()
        .map(|c| (c.name.as_str(), c))
        .collect();
    let public: Vec<_> = contract
        .stateful_circuits
        .iter()
        .filter(|c| !c.internal)
        .collect();
    assert_eq!(
        public.iter().map(|c| c.name.as_str()).collect::<Vec<_>>(),
        ["run", "conversions"]
    );
    for circuit in public {
        let checked = check_with_calls(circuit, &ledger, &witnesses, &pure, &calls)
            .unwrap_or_else(|error| panic!("{}: {error:?}", circuit.name));
        render(&checked).unwrap();
    }
}
#[test]
fn local_and_witness_owned_inputs_preserve_vector_storage_boundaries() {
    let cases = vec![
        (
            Type::OpaqueString,
            project(Expr::Default { ty: carrier() }, "text", 0),
        ),
        (
            Type::OpaqueBytes,
            project(Expr::Default { ty: carrier() }, "bytes", 1),
        ),
        (
            vector(Type::Field, 2),
            construct_vector(vec![field(1), field(2)], Type::Field),
        ),
        (carrier(), Expr::Default { ty: carrier() }),
        (
            empty::empty_type(),
            Expr::Default {
                ty: empty::empty_type(),
            },
        ),
    ];
    for (ty, value) in cases {
        // Local values and calls are accepted independently of witness/storage policy.
        check_root(
            &root(local("held", ty.clone(), value.clone(), boolean())),
            &[],
        )
        .unwrap();
        let witness = WitnessDeclaration {
            source: None,
            name: "consume".into(),
            parameters: vec![Parameter {
                name: "owned".into(),
                ty: ty.clone(),
            }],
            result: Type::Boolean,
        };
        let expression = Expr::WitnessCall {
            name: "consume".into(),
            arguments: vec![value.clone()],
        };
        render(
            &check(
                &root(expression),
                &HashMap::new(),
                &HashMap::from([("consume", &witness)]),
            )
            .unwrap(),
        )
        .unwrap();
        let helper = StatefulCircuit {
            source: None,
            name: "consume".into(),
            internal: true,
            parameters: vec![Parameter {
                name: "owned".into(),
                ty: ty.clone(),
            }],
            actions: vec![],
            result: Type::Unit,
            return_value: StateReturn::Unit,
        };
        let mut caller = root(boolean());
        caller.actions = vec![StateAction::CircuitCall {
            name: "consume".into(),
            arguments: vec![value.clone()],
        }];
        render(
            &check_with_calls(
                &caller,
                &HashMap::new(),
                &HashMap::new(),
                &HashMap::new(),
                &HashMap::from([("consume", &helper)]),
            )
            .unwrap(),
        )
        .unwrap();
        let cell = LedgerField {
            source: None,
            id: "owned".into(),
            index: 0,
            path: vec![0],
            declaration: LedgerFieldKind::Cell { ty: ty.clone() },
        };
        caller.actions = vec![StateAction::CellWrite {
            field: "owned".into(),
            index: 0,
            value,
        }];
        let storage = check(&caller, &HashMap::from([("owned", &cell)]), &HashMap::new());
        if matches!(ty, Type::OpaqueString | Type::OpaqueBytes) {
            render(&storage.unwrap()).unwrap();
        } else {
            assert!(matches!(storage, Err(PlanError::Unsupported(_))));
        }
    }
}
