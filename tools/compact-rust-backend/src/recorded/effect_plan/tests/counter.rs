// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Counter signatures, lexical operands and evaluation order at the plan boundary.
use super::*;
#[path = "counter/validation.rs"]
mod validation;
const U16_MAX: &str = "65535";
const U64_MAX: &str = "18446744073709551615";

fn contract() -> Contract {
    let mut contract: Contract = serde_json::from_str(include_str!(
        "../../../../tests/effect-plan-prototype-schema20-ir.json"
    ))
    .unwrap();
    let mut peer = contract.ledger_fields[1].clone();
    peer.id = "peer".into();
    peer.index = 2;
    peer.path = vec![2];
    contract.ledger_fields.push(peer);
    contract
}
fn declarations(contract: &Contract) -> declarations::Declarations {
    declarations::Declarations::new(
        &contract
            .ledger_fields
            .iter()
            .map(|f| (f.id.as_str(), f))
            .collect(),
        &contract
            .witnesses
            .iter()
            .map(|w| (w.name.as_str(), w))
            .collect(),
    )
}
fn circuit(actions: Vec<StateAction>, expression: Expr, result: Type) -> StatefulCircuit {
    StatefulCircuit {
        source: None,
        name: "counter_candidate".into(),
        internal: false,
        parameters: vec![Parameter {
            name: "amount".into(),
            ty: validate::uint(U16_MAX),
        }],
        actions,
        result,
        return_value: StateReturn::Expression { value: expression },
    }
}
fn checked(circuit: &StatefulCircuit) -> Result<CheckedPlan, PlanError> {
    let contract = contract();
    check(
        circuit,
        &contract
            .ledger_fields
            .iter()
            .map(|f| (f.id.as_str(), f))
            .collect(),
        &contract
            .witnesses
            .iter()
            .map(|w| (w.name.as_str(), w))
            .collect(),
    )
}
fn decrement(name: &str) -> StateAction {
    StateAction::CounterDecrement {
        field: "round".into(),
        index: 1,
        amount: CounterAmount::Parameter { name: name.into() },
    }
}
fn read(field: &str, index: u8) -> Expr {
    Expr::CounterRead {
        field: field.into(),
        index,
    }
}
fn less_than(threshold: Expr) -> Expr {
    Expr::CounterLessThan {
        field: "round".into(),
        index: 1,
        threshold: Box::new(threshold),
    }
}
fn unsigned(value: &str, max: &str) -> Expr {
    Expr::UnsignedLiteral {
        value: value.into(),
        max: max.into(),
    }
}
fn unsupported(circuit: &StatefulCircuit) -> RecordingGap {
    match checked(circuit).unwrap_err() {
        PlanError::Unsupported(gap) => gap,
        other => panic!("expected lowering refusal: {other:?}"),
    }
}

#[test]
fn mutations_reuse_slot_adapters_and_resolve_lexical_amounts() {
    for literal in [0, u16::MAX] {
        let candidate = circuit(
            vec![
                StateAction::CounterIncrement {
                    field: "round".into(),
                    index: 1,
                    amount: CounterAmount::Literal { value: literal },
                },
                decrement("amount"),
                StateAction::Let {
                    bindings: vec![LocalBinding {
                        name: "amount".into(),
                        ty: validate::uint(U16_MAX),
                        value: unsigned("7", U16_MAX),
                    }],
                    action: Box::new(decrement("amount")),
                },
                decrement("amount"),
                StateAction::CounterReset {
                    field: "round".into(),
                    index: 1,
                },
            ],
            read("round", 1),
            validate::uint(U64_MAX),
        );
        let checked = checked(&candidate).unwrap();
        let amounts: Vec<_> = checked
            .plan()
            .body
            .bindings
            .iter()
            .filter_map(|b| {
                if let Operation::CounterDecrement { amount, .. } = &b.operation {
                    Some(amount.id)
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(amounts[0], checked.plan().parameters[0].id);
        assert_ne!(amounts[0], amounts[1]);
        assert_eq!(amounts[0], amounts[2], "a local must not escape its action");
        let steps = render(&checked).unwrap().steps;
        let syntax = quote::quote!(#(#steps)*).to_string();
        assert!(syntax.contains("record_increment"));
        assert_eq!(syntax.matches("record_decrement").count(), 3);
        assert!(syntax.contains("record_reset"));
    }
}

#[test]
fn comparison_evaluates_its_threshold_once_before_the_counter_query() {
    let threshold = Expr::Let {
        bindings: vec![LocalBinding {
            name: "observed".into(),
            ty: Type::Field,
            value: Expr::WitnessCall {
                name: "mark".into(),
                arguments: vec![],
            },
        }],
        body: Box::new(read("peer", 2)),
    };
    let checked = checked(&circuit(vec![], less_than(threshold), Type::Boolean)).unwrap();
    let bindings = &checked.plan().body.bindings;
    assert_eq!(bindings.len(), 3);
    assert!(matches!(bindings[0].operation, Operation::Witness { .. }));
    assert!(matches!(
        bindings[1].operation,
        Operation::CounterRead { .. }
    ));
    let Operation::CounterLessThan { threshold, .. } = &bindings[2].operation else {
        panic!("expected final comparison")
    };
    assert_eq!(threshold.id, bindings[1].value.id);
    let steps = render(&checked).unwrap().steps;
    let syntax = quote::quote!(#(#steps)*).to_string();
    assert_eq!(syntax.matches("record_read").count(), 1);
    assert_eq!(syntax.matches("record_less_than").count(), 1);
    assert!(syntax.find("record_read").unwrap() < syntax.find("record_less_than").unwrap());
    for value in ["0", "1", U64_MAX] {
        assert!(checked_boundary(value).is_ok());
    }
}
fn checked_boundary(value: &str) -> Result<CheckedPlan, PlanError> {
    checked(&circuit(
        vec![],
        less_than(unsigned(value, U64_MAX)),
        Type::Boolean,
    ))
}

#[test]
fn malformed_operands_keep_exact_source_paths_and_do_not_expand_types() {
    for ty in [Type::Field, validate::uint(U64_MAX)] {
        let mut candidate = circuit(
            vec![decrement("amount")],
            read("round", 1),
            validate::uint(U64_MAX),
        );
        candidate.parameters[0].ty = ty;
        assert_eq!(unsupported(&candidate).path, "actions[0].amount");
    }
    let candidate = circuit(
        vec![decrement("missing")],
        read("round", 1),
        validate::uint(U64_MAX),
    );
    assert_eq!(unsupported(&candidate).path, "actions[0].amount");
    for threshold in [
        unsigned("0", U16_MAX),
        Expr::FieldLiteral { value: "0".into() },
    ] {
        let gap = unsupported(&circuit(vec![], less_than(threshold), Type::Boolean));
        assert_eq!(gap.path, "return_value.value.threshold");
        assert!(gap.detail.contains("Uint<64>"));
    }
    let gap = unsupported(&circuit(
        vec![],
        less_than(Expr::Call {
            name: "unsupported".into(),
            arguments: vec![],
        }),
        Type::Boolean,
    ));
    assert_eq!(gap.path, "return_value.value.threshold");
    assert_eq!(gap.ir_node, "Expr::Call");
    assert!(checked_boundary("18446744073709551616").is_err());
}

#[test]
fn unselected_branches_are_checked_without_hiding_the_first_refusal() {
    let actions = vec![StateAction::If {
        condition: Expr::Boolean { value: false },
        then: Box::new(decrement("missing")),
        otherwise: Box::new(StateAction::CounterReset {
            field: "unknown".into(),
            index: 9,
        }),
    }];
    let candidate = circuit(actions, read("round", 1), validate::uint(U64_MAX));
    assert_eq!(unsupported(&candidate).path, "actions[0].then.amount");
}
