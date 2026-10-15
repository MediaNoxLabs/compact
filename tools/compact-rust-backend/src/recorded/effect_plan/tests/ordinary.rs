// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Candidate-only ordinary return lowering; production admission is unchanged.
use super::*;
#[path = "ordinary/scopes.rs"]
mod scopes;

fn contract() -> Contract {
    serde_json::from_str(include_str!(
        "../../../../tests/terminal-lexical-return-schema20-ir.json"
    ))
    .unwrap()
}
fn checked(contract: &Contract, circuit: &StatefulCircuit) -> Result<CheckedPlan, PlanError> {
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
fn candidate(actions: Vec<StateAction>, result: Expr) -> StatefulCircuit {
    StatefulCircuit {
        source: None,
        name: "ordinary_candidate".into(),
        internal: false,
        parameters: vec![],
        actions,
        result: Type::Field,
        return_value: StateReturn::Expression { value: result },
    }
}
fn parameter(name: &str) -> Expr {
    Expr::Parameter { name: name.into() }
}
fn literal(value: &str) -> Expr {
    Expr::FieldLiteral {
        value: value.into(),
    }
}
fn sequence(actions: Vec<StateAction>) -> StateAction {
    StateAction::Sequence { actions }
}
fn binding(name: &str, value: Expr, action: StateAction) -> StateAction {
    StateAction::Let {
        bindings: vec![LocalBinding {
            name: name.into(),
            ty: Type::Field,
            value,
        }],
        action: Box::new(action),
    }
}
fn assert_true() -> StateAction {
    StateAction::Assert {
        condition: Expr::Boolean { value: true },
        message: "guard".into(),
    }
}
fn read() -> Expr {
    Expr::CellRead {
        field: "stored".into(),
        index: 0,
    }
}
fn write(value: Expr) -> StateAction {
    StateAction::CellWrite {
        field: "stored".into(),
        index: 0,
        value,
    }
}
fn refusal(circuit: &StatefulCircuit) -> RecordingGap {
    match checked(&contract(), circuit).unwrap_err() {
        PlanError::Unsupported(gap) => gap,
        other => panic!("expected lowering refusal: {other:?}"),
    }
}
fn returned_field(checked: &CheckedPlan) -> [u8; 32] {
    let plan = checked.plan();
    let result = plan
        .body
        .bindings
        .iter()
        .find(|b| b.value == plan.body.result)
        .unwrap();
    let Operation::Literal(Literal::Field(value)) = result.operation else {
        panic!("expected Field literal return")
    };
    value
}

#[test]
fn retained_terminal_and_root_let_sources_lower_without_helper_expansion() {
    let contract = contract();
    for name in ["two", "three", "echo", "observed", "inner"] {
        let circuit = contract
            .stateful_circuits
            .iter()
            .find(|c| c.name == name)
            .unwrap();
        let checked = checked(&contract, circuit).unwrap();
        assert!(render(&checked).is_ok(), "{name}");
        let plan = checked.plan();
        if name == "echo" {
            assert_eq!(plan.body.result, plan.parameters[0]);
        } else {
            let last_write = plan
                .body
                .bindings
                .iter()
                .rev()
                .find_map(|b| {
                    if let Operation::CellWrite { value, .. } = &b.operation {
                        Some(value)
                    } else {
                        None
                    }
                })
                .unwrap();
            assert_eq!(
                &plan.body.result, last_write,
                "{name} returns its selected local"
            );
        }
        if name == "observed" {
            let position = |test: fn(&Operation) -> bool| {
                plan.body
                    .bindings
                    .iter()
                    .position(|b| test(&b.operation))
                    .unwrap()
            };
            assert!(
                position(|o| matches!(o, Operation::CellRead(_)))
                    < position(|o| matches!(o, Operation::Witness { .. }))
            );
            assert!(
                position(|o| matches!(o, Operation::Witness { .. }))
                    < position(|o| matches!(o, Operation::Assert { .. }))
            );
            assert!(
                position(|o| matches!(o, Operation::Assert { .. }))
                    < position(|o| matches!(o, Operation::CellWrite { .. }))
            );
        }
    }
    let nested = contract
        .stateful_circuits
        .iter()
        .find(|c| c.name == "nested")
        .unwrap();
    let PlanError::Unsupported(gap) = checked(&contract, nested).unwrap_err() else {
        panic!("helper calls remain unsupported")
    };
    assert_eq!(gap.ir_node, "Expr::Call");
    assert_eq!(gap.path, "return_value.value");
    let root: Contract = serde_json::from_str(include_str!(
        "../../../../tests/root-let-action-return-schema17-ir.json"
    ))
    .unwrap();
    let checked = checked(&root, &root.stateful_circuits[0]).unwrap();
    assert_eq!(checked.plan().body.result, checked.plan().parameters[0]);
}

#[test]
fn extracted_return_keeps_the_prewrite_snapshot_and_source_order() {
    let mut circuit = candidate(
        vec![binding("previous", read(), write(parameter("next")))],
        parameter("previous"),
    );
    circuit.parameters.push(Parameter {
        name: "next".into(),
        ty: Type::Field,
    });
    let checked = checked(&contract(), &circuit).unwrap();
    let bindings = &checked.plan().body.bindings;
    assert_eq!(bindings.len(), 2, "do not reread the cell for the return");
    assert!(matches!(bindings[0].operation, Operation::CellRead(_)));
    assert!(matches!(bindings[1].operation, Operation::CellWrite { .. }));
    assert_eq!(checked.plan().body.result, bindings[0].value);
    assert_eq!(bindings[0].path, "actions[0].bindings[0].value");
    assert_eq!(bindings[1].path, "actions[0].action");
}

#[test]
fn first_refusal_follows_original_action_and_binding_paths() {
    let unsupported = StateAction::Expression {
        value: Expr::Call {
            name: "unknown".into(),
            arguments: vec![],
        },
    };
    let circuit = candidate(
        vec![
            unsupported.clone(),
            binding("value", literal("1"), sequence(vec![])),
        ],
        parameter("missing"),
    );
    assert_eq!(refusal(&circuit).path, "actions[0].value");
    let circuit = candidate(
        vec![binding(
            "value",
            Expr::Call {
                name: "unknown".into(),
                arguments: vec![],
            },
            unsupported,
        )],
        parameter("missing"),
    );
    assert_eq!(refusal(&circuit).path, "actions[0].bindings[0].value");
    let circuit = candidate(
        vec![sequence(vec![
            assert_true(),
            binding(
                "value",
                literal("1"),
                StateAction::Expression {
                    value: parameter("missing"),
                },
            ),
        ])],
        literal("0"),
    );
    assert_eq!(refusal(&circuit).path, "actions[0].actions[1].action.value");
}

#[test]
fn specialized_and_effectful_returns_keep_their_existing_scope_policy() {
    for (returned, result_type) in [
        (StateReturn::Unit, Type::Unit),
        (
            StateReturn::CellRead {
                field: "stored".into(),
                index: 0,
            },
            Type::Field,
        ),
    ] {
        let mut circuit = candidate(
            vec![binding("local", read(), sequence(vec![]))],
            literal("0"),
        );
        circuit.return_value = returned;
        circuit.result = result_type;
        assert!(checked(&contract(), &circuit).is_ok());
    }
    let mut circuit = candidate(vec![], literal("0"));
    circuit.return_value = StateReturn::Effectful {
        body: ReturnPlan::Sequence {
            actions: vec![binding("local", literal("1"), sequence(vec![]))],
            result: Box::new(ReturnPlan::Value {
                value: parameter("local"),
            }),
        },
    };
    assert_eq!(refusal(&circuit).path, "return_value.body.result.value");
}
