// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Intent validation bypasses source lowering and uses independently registered types.
use super::super::type_environment::TypeEnvironment;
use super::*;
#[path = "composite/action_calls.rs"]
mod action_calls;
use crate::coin_shapes::{qualified_coin_type, shielded_coin_type, shielded_recipient_type};
fn value(id: usize, ty: Type) -> Value {
    Value { id, ty }
}
fn binding(id: usize, ty: Type, operation: Operation) -> Binding {
    Binding {
        value: value(id, ty),
        operation,
        path: format!("intent[{id}]"),
    }
}
fn candidate(input: bool) -> Plan {
    let parameters = vec![
        value(0, qualified_coin_type()),
        value(1, shielded_coin_type()),
        value(2, shielded_recipient_type()),
    ];
    let operation = if input {
        Operation::CreateZswapInput {
            coin: parameters[0].clone(),
        }
    } else {
        Operation::CreateZswapOutput {
            coin: parameters[1].clone(),
            recipient: parameters[2].clone(),
        }
    };
    Plan {
        parameters,
        result_type: Type::Unit,
        body: Block {
            bindings: vec![binding(3, Type::Unit, operation)],
            result: value(3, Type::Unit),
        },
    }
}
fn validate(plan: Plan) -> Result<CheckedPlan, RecordingGap> {
    // Register exactly the supplied parameter definitions, even when malformed.
    // Operation-specific canonical checks must therefore supply the rejection.
    let root = StatefulCircuit {
        source: None,
        name: "source".into(),
        internal: false,
        parameters: plan
            .parameters
            .iter()
            .map(|v| Parameter {
                name: format!("p{}", v.id),
                ty: v.ty.clone(),
            })
            .collect(),
        actions: vec![],
        result: plan.result_type.clone(),
        return_value: StateReturn::Unit,
    };
    let types = TypeEnvironment::from_sources(
        &root,
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
    )
    .unwrap();
    validate::check(
        plan,
        declarations::Declarations::default().with_type_environment(types),
    )
}
#[test]
fn exact_canonical_shapes_live_operands_and_unit_result_are_independent_invariants() {
    for input in [true, false] {
        validate(candidate(input)).unwrap();
        for mode in 0..5 {
            let mut plan = candidate(input);
            match mode {
                0 => match &mut plan.body.bindings[0].operation {
                    Operation::CreateZswapInput { coin }
                    | Operation::CreateZswapOutput { coin, .. } => coin.id = 99,
                    _ => unreachable!(),
                },
                1 => match &mut plan.body.bindings[0].operation {
                    Operation::CreateZswapInput { coin }
                    | Operation::CreateZswapOutput { coin, .. } => coin.ty = Type::Boolean,
                    _ => unreachable!(),
                },
                2 => {
                    plan.body.bindings[0].value.ty = Type::Boolean;
                    plan.body.result.ty = Type::Boolean;
                    plan.result_type = Type::Boolean;
                }
                3 => {
                    match &mut plan.body.bindings[0].operation {
                        Operation::CreateZswapInput { coin } => *coin = plan.parameters[1].clone(),
                        Operation::CreateZswapOutput { coin, recipient } => {
                            std::mem::swap(coin, recipient)
                        }
                        _ => unreachable!(),
                    };
                }
                4 => {
                    let index = if input { 0 } else { 2 };
                    plan.parameters.remove(index);
                }
                _ => unreachable!(),
            }
            assert!(validate(plan).is_err(), "input={input} mutation={mode}");
        }
    }
}
#[test]
fn independently_registered_wrong_coin_name_field_order_width_and_recipient_are_refused() {
    for (input, index) in [(true, 0), (false, 1), (false, 2)] {
        for mode in 0..3 {
            let mut plan = candidate(input);
            let Type::Struct { name, fields } = &mut plan.parameters[index].ty else {
                unreachable!()
            };
            match mode {
                0 => *name = "Impostor".into(),
                1 => fields.swap(0, 1),
                2 => fields.last_mut().unwrap().ty = Type::Unsigned { max: "255".into() },
                _ => unreachable!(),
            }
            match &mut plan.body.bindings[0].operation {
                Operation::CreateZswapInput { coin } => coin.ty = plan.parameters[index].ty.clone(),
                Operation::CreateZswapOutput { coin, recipient } => {
                    if index == 1 {
                        coin.ty = plan.parameters[index].ty.clone()
                    } else {
                        recipient.ty = plan.parameters[index].ty.clone()
                    }
                }
                _ => unreachable!(),
            }
            assert!(
                validate(plan).is_err(),
                "operand={index} canonical mutation={mode}"
            );
        }
    }
}
#[test]
fn repeated_aggregate_operands_remain_live_and_intents_do_not_add_private_output_twice() {
    let mut plan = candidate(false);
    plan.body.bindings.push(binding(
        4,
        Type::Unit,
        Operation::CreateZswapOutput {
            coin: plan.parameters[1].clone(),
            recipient: plan.parameters[2].clone(),
        },
    ));
    plan.body.result = value(4, Type::Unit);
    let checked = validate(plan).unwrap();
    let rendered = render(&checked).unwrap();
    let steps = rendered.steps;
    let source = quote::quote!(#(#steps)*).to_string();
    assert_eq!(source.matches("create_zswap_output").count(), 2);
    assert_eq!(source.matches("coin_info_from_compact").count(), 2);
    assert_eq!(source.matches("coin_recipient_from_compact").count(), 2);
    assert!(!source.contains("push_private"));
}
#[test]
fn output_witness_operands_evaluate_coin_before_recipient_once_and_refuse_first_error() {
    let witnesses = [
        WitnessDeclaration {
            source: None,
            name: "coin".into(),
            parameters: vec![],
            result: shielded_coin_type(),
        },
        WitnessDeclaration {
            source: None,
            name: "recipient".into(),
            parameters: vec![],
            result: shielded_recipient_type(),
        },
    ];
    let maps = witnesses.iter().map(|w| (w.name.as_str(), w)).collect();
    let witness = |name: &str| Expr::WitnessCall {
        name: name.into(),
        arguments: vec![],
    };
    let root = |coin, recipient| StatefulCircuit {
        source: None,
        name: "root".into(),
        internal: false,
        parameters: vec![],
        actions: vec![],
        result: Type::Unit,
        return_value: StateReturn::Expression {
            value: Expr::CreateZswapOutput {
                coin: Box::new(coin),
                recipient: Box::new(recipient),
            },
        },
    };
    let checked = check(
        &root(witness("coin"), witness("recipient")),
        &HashMap::new(),
        &maps,
    )
    .unwrap();
    let bindings = &checked.plan().body.bindings;
    assert_eq!(bindings.len(), 3);
    assert_eq!(bindings[0].path, "return_value.value.coin");
    assert_eq!(bindings[1].path, "return_value.value.recipient");
    assert!(matches!(bindings[0].operation, Operation::Witness { .. }));
    assert!(matches!(bindings[1].operation, Operation::Witness { .. }));
    let Operation::CreateZswapOutput { coin, recipient } = &bindings[2].operation else {
        panic!("output after both witnesses")
    };
    assert_eq!(coin, &bindings[0].value);
    assert_eq!(recipient, &bindings[1].value);
    let error = check(
        &root(witness("missing_coin"), witness("missing_recipient")),
        &HashMap::new(),
        &maps,
    )
    .unwrap_err();
    let PlanError::Unsupported(gap) = error else {
        panic!("source refusal")
    };
    assert_eq!(gap.path, "return_value.value.coin");
}
#[test]
fn intent_cannot_capture_branch_local_or_use_a_future_binding() {
    for branch_local in [false, true] {
        let mut plan = candidate(false);
        plan.parameters.push(value(5, Type::Boolean));
        let local_coin = binding(6, shielded_coin_type(), Operation::Default);
        let unit = |id| binding(id, Type::Unit, Operation::Literal(Literal::Unit));
        let prefix = if branch_local {
            vec![binding(
                7,
                Type::Unit,
                Operation::If {
                    condition: value(5, Type::Boolean),
                    then: Block {
                        bindings: vec![local_coin.clone(), unit(8)],
                        result: value(8, Type::Unit),
                    },
                    otherwise: Block {
                        bindings: vec![unit(9)],
                        result: value(9, Type::Unit),
                    },
                },
            )]
        } else {
            vec![]
        };
        let mut intent = plan.body.bindings.remove(0);
        let Operation::CreateZswapOutput { coin, .. } = &mut intent.operation else {
            unreachable!()
        };
        *coin = value(6, shielded_coin_type());
        plan.body.bindings = prefix;
        plan.body.bindings.push(intent);
        if !branch_local {
            plan.body.bindings.push(local_coin);
        }
        assert_eq!(validate(plan).unwrap_err().path, "intent[3]");
    }
}
