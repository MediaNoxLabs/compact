// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;
#[test]
fn recursive_values_cover_every_old_constructor_and_unsigned_bound_class() {
    for maximum in [
        1,
        19,
        255,
        u64::MAX as u128,
        u64::MAX as u128 + 1,
        u128::MAX,
    ] {
        let ty = unsigned(maximum);
        let result = structure("Result", vec![ty.clone(), Type::Boolean]);
        let plain = number(1, maximum);
        let variants = vec![
            plain.clone(),
            Expr::Coerce {
                value: Box::new(plain.clone()),
                ty: ty.clone(),
            },
            Expr::UnsignedCast {
                value: Box::new(plain.clone()),
                max: maximum.to_string(),
            },
            Expr::If {
                condition: Box::new(parameter("selected")),
                then: Box::new(plain.clone()),
                otherwise: Box::new(number(0, maximum)),
            },
            local("saved", ty.clone(), plain.clone(), parameter("saved")),
            local(
                "saved",
                ty.clone(),
                number(0, maximum),
                local("saved", ty.clone(), plain, parameter("saved")),
            ),
        ];
        assert_eq!(variants.len(), 6);
        for (index, value) in variants.into_iter().enumerate() {
            let mut contract = subject(
                result.clone(),
                record(result.clone(), vec![value, parameter("selected")]),
            );
            contract.stateful_circuits[0].parameters.push(Parameter {
                name: "selected".into(),
                ty: Type::Boolean,
            });
            included(
                &contract,
                "root",
                &format!("Values max={maximum} variant={index}"),
            );
        }
    }
    let leaf = structure(
        "Leaf",
        vec![Type::Boolean, Type::Bytes { length: 32 }, unsigned(19)],
    );
    let result = structure(
        "Nested",
        vec![leaf.clone(), crate::ir::contract_address_type()],
    );
    included(
        &subject(
            result.clone(),
            record(
                result,
                vec![
                    Expr::Default { ty: leaf },
                    Expr::KernelSelf {
                        ty: crate::ir::contract_address_type(),
                    },
                ],
            ),
        ),
        "root",
        "nested/default/bytes/address",
    );
}
#[test]
fn restricted_witness_and_isolated_nested_repeated_helpers_are_included() {
    let result = structure("Result", vec![unsigned(u64::MAX as u128)]);
    let witness = Expr::WitnessCall {
        name: "pick".into(),
        arguments: vec![number(7, 255), number(8, 255)],
    };
    let mut contract = subject(result.clone(), record(result.clone(), vec![witness]));
    contract.witnesses.push(WitnessDeclaration {
        source: None,
        name: "pick".into(),
        parameters: vec![
            Parameter {
                name: "a".into(),
                ty: unsigned(255),
            },
            Parameter {
                name: "b".into(),
                ty: unsigned(255),
            },
        ],
        result: unsigned(u64::MAX as u128),
    });
    included(&contract, "root", "Uint8 arguments/Uint64 witness");
    let mut helper = contract.stateful_circuits[0].clone();
    helper.name = "leaf".into();
    helper.internal = true;
    helper.parameters = vec![Parameter {
        name: "flag".into(),
        ty: Type::Boolean,
    }];
    let mut middle = helper.clone();
    middle.name = "middle".into();
    middle.return_value = StateReturn::Expression {
        value: call("leaf", vec![parameter("flag")]),
    };
    contract.stateful_circuits.extend([helper, middle]);
    contract.stateful_circuits[0].return_value = StateReturn::Expression {
        value: local(
            "unused",
            result.clone(),
            call("middle", vec![Expr::Boolean { value: false }]),
            call("middle", vec![Expr::Boolean { value: true }]),
        ),
    };
    included(&contract, "root", "nested/repeated isolated stateful calls");
    included(&contract, "middle", "internal Values root");
}
#[test]
fn retained_plain_struct_exports_are_explicit_old_controls() {
    let contract: Contract = serde_json::from_str(include_str!(
        "../../../../tests/stateful-struct-schema20-ir.json"
    ))
    .unwrap();
    let names: Vec<_> = contract
        .stateful_circuits
        .iter()
        .map(|c| c.name.as_str())
        .collect();
    assert_eq!(names, ["snapshot", "reverse", "nested", "planned"]);
    for name in ["snapshot", "reverse", "nested", "planned"] {
        included(&contract, name, "retained stateful struct");
    }
}
#[test]
fn malformed_value_scopes_members_calls_and_bounds_remain_refused() {
    let result = structure("Result", vec![unsigned(19)]);
    for value in [
        parameter("missing"),
        number(20, 19),
        Expr::Boolean { value: true },
        call("unknown", vec![]),
    ] {
        let contract = subject(result.clone(), record(result.clone(), vec![value]));
        let context = Context::new(&contract);
        let root = &contract.stateful_circuits[0];
        assert!(context.old(root).is_none());
        assert!(matches!(
            context.algebra(root),
            Err(effect_plan::PlanError::Unsupported(_))
        ));
    }
    let contract = subject(result.clone(), record(result, vec![]));
    let context = Context::new(&contract);
    assert!(context.old(&contract.stateful_circuits[0]).is_none());
    assert!(context.algebra(&contract.stateful_circuits[0]).is_err());
}
