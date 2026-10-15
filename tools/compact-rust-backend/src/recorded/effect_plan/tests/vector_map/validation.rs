// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;

#[test]
fn independent_vector_map_validation_rechecks_formals_captures_results_and_escape() {
    let checked = checked(&circuit(2, addition()));
    for mutation in [
        "source",
        "formal_collision",
        "formal_type",
        "capture",
        "annotation",
        "body_result",
        "output_length",
        "escape",
    ] {
        let mut plan = checked.plan().clone();
        let binding = map_binding(&mut plan);
        if mutation == "output_length" {
            binding.value.ty = vector(Type::Field, 3);
        }
        let Operation::VectorMap {
            source,
            parameter,
            body,
        } = &mut binding.operation
        else {
            unreachable!()
        };
        match mutation {
            "source" => source.id = 9999,
            "formal_collision" => parameter.id = 0,
            "formal_type" => parameter.ty = Type::Boolean,
            "capture" | "annotation" => {
                let Operation::Add(_, capture) = &mut body.bindings[0].operation else {
                    unreachable!()
                };
                if mutation == "capture" {
                    capture.id = 9999;
                } else {
                    capture.ty = Type::Boolean;
                }
            }
            "body_result" => body.result.ty = Type::Boolean,
            "escape" => {
                plan.body.result = body.result.clone();
                plan.result_type = Type::Field;
            }
            "output_length" => (),
            _ => unreachable!(),
        }
        assert!(
            validate::validate(&plan, checked.declarations()).is_err(),
            "{mutation}"
        );
    }
}

#[test]
fn semantic_purity_rejects_private_public_and_stateful_operations_even_without_public_trace() {
    let initial = checked(&circuit(1, addition()));
    // Classification is exhaustive and intentionally stricter than public-effect presence.
    let operations = [
        Operation::OwnPublicKey,
        Operation::Witness {
            declaration: WitnessId(0),
            arguments: vec![],
        },
        Operation::CellRead(CellSlot {
            declaration: LedgerId(0),
            index: 0,
        }),
        Operation::CreateZswapInput { coin: field(1) },
        Operation::Call {
            declaration: CallId(0),
            arguments: vec![],
            parameters: vec![],
            body: Block {
                bindings: vec![],
                result: field(1),
            },
        },
    ];
    for operation in operations {
        let mut plan = initial.plan().clone();
        let Operation::VectorMap { body, .. } = &mut map_binding(&mut plan).operation else {
            unreachable!()
        };
        body.bindings[0].operation = operation;
        assert!(!context_free::bindings(&body.bindings));
        assert!(validate::validate(&plan, initial.declarations()).is_err());
    }
    // OwnPublicKey is itself a valid independently typed operation. Its refusal
    // here must come specifically from map purity, not missing declaration/SSA.
    let mut c = circuit(1, parameter("item"));
    c.parameters.push(Parameter {
        name: "unused_key".into(),
        ty: NativeWitnessBuiltin::OwnPublicKey.result_type(),
    });
    let checked = checked(&c);
    let mut plan = checked.plan().clone();
    let Operation::VectorMap { body, .. } = &mut map_binding(&mut plan).operation else {
        unreachable!()
    };
    body.bindings.push(Binding {
        value: Value {
            id: 9999,
            ty: NativeWitnessBuiltin::OwnPublicKey.result_type(),
        },
        operation: Operation::OwnPublicKey,
        path: "return_value.value.body.key".into(),
    });
    let error = validate::validate(&plan, checked.declarations()).unwrap_err();
    assert_eq!(error.detail, "VectorMap body must be context-free");
}
