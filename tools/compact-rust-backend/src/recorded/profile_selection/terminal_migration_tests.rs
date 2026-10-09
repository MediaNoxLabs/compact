// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Finite old-first inclusion and actual routing; not exhaustive language proof.
use super::*;
use crate::ir::{
    Contract, Expr, LedgerFieldKind, LocalBinding, Parameter, StateAction, StateReturn, Type,
};
use quote::quote;

#[path = "terminal_migration_tests/inclusion.rs"]
mod inclusion;
#[path = "terminal_migration_tests/routing.rs"]
mod routing;

fn fixture() -> Contract {
    serde_json::from_str(include_str!(
        "../../../tests/terminal-lexical-return-schema20-ir.json"
    ))
    .unwrap()
}
fn parameter(name: &str) -> Expr {
    Expr::Parameter { name: name.into() }
}
fn literal() -> Expr {
    Expr::FieldLiteral { value: "7".into() }
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
fn sequence(actions: Vec<StateAction>) -> StateAction {
    StateAction::Sequence { actions }
}
fn bind(name: &str, ty: Type, value: Expr, action: StateAction) -> StateAction {
    StateAction::Let {
        bindings: vec![LocalBinding {
            name: name.into(),
            ty,
            value,
        }],
        action: Box::new(action),
    }
}
fn call(name: &str, arguments: Vec<Expr>) -> Expr {
    Expr::Call {
        name: name.into(),
        arguments,
    }
}
fn subject() -> Contract {
    let mut contract = fixture();
    contract.stateful_circuits = vec![StatefulCircuit {
        source: None,
        name: "root".into(),
        internal: false,
        parameters: vec![
            Parameter {
                name: "seed".into(),
                ty: Type::Field,
            },
            Parameter {
                name: "selected".into(),
                ty: Type::Boolean,
            },
        ],
        result: Type::Field,
        actions: vec![write(read())],
        return_value: StateReturn::Expression {
            value: parameter("seed"),
        },
    }];
    contract
}
struct Context<'a> {
    ledger: HashMap<&'a str, &'a LedgerField>,
    witnesses: HashMap<&'a str, &'a WitnessDeclaration>,
    pure: HashMap<&'a str, &'a PureCircuit>,
    circuits: HashMap<&'a str, &'a StatefulCircuit>,
}
impl<'a> Context<'a> {
    fn new(contract: &'a Contract) -> Self {
        Self {
            ledger: contract
                .ledger_fields
                .iter()
                .map(|v| (v.id.as_str(), v))
                .collect(),
            witnesses: contract
                .witnesses
                .iter()
                .map(|v| (v.name.as_str(), v))
                .collect(),
            pure: contract
                .circuits
                .iter()
                .map(|v| (v.name.as_str(), v))
                .collect(),
            circuits: contract
                .stateful_circuits
                .iter()
                .map(|v| (v.name.as_str(), v))
                .collect(),
        }
    }
    fn old(&self, circuit: &StatefulCircuit) -> Option<TypedPlan> {
        typed_plan::lower_terminal_returns(
            circuit,
            &self.ledger,
            &self.witnesses,
            &self.pure,
            &self.circuits,
        )
    }
    fn algebra(&self, circuit: &StatefulCircuit) -> Result<TypedPlan, effect_plan::PlanError> {
        effect_plan::lower_with_calls(
            circuit,
            &self.ledger,
            &self.witnesses,
            &self.pure,
            &self.circuits,
        )
    }
    fn selected(&self, circuit: &StatefulCircuit) -> Result<ProfileSelection, RenderError> {
        select(
            circuit,
            &self.ledger,
            &self.witnesses,
            &self.pure,
            &self.circuits,
            None,
        )
    }
    fn terminal(&self, circuit: &StatefulCircuit) -> Result<Option<TypedPlan>, RenderError> {
        checked_terminal_domain(
            circuit,
            &self.ledger,
            &self.witnesses,
            &self.pure,
            &self.circuits,
        )
    }
}
fn tokens(plan: &TypedPlan) -> String {
    let (steps, result) = (&plan.steps, &plan.result);
    quote!(#(#steps)* #result).to_string()
}
fn selected_tokens(selected: ProfileSelection) -> Option<String> {
    let (steps, result) = (selected.organizer_steps?, selected.typed_result?);
    Some(quote!(#(#steps)* #result).to_string())
}
fn included(contract: &Contract, name: &str) {
    let circuit = contract
        .stateful_circuits
        .iter()
        .find(|c| c.name == name)
        .unwrap();
    let context = Context::new(contract);
    context
        .old(circuit)
        .expect("case must FIRST pass complete legacy terminal admission");
    let algebra = context
        .algebra(circuit)
        .expect("old case must lower/check/render");
    assert_eq!(
        tokens(&context.terminal(circuit).unwrap().unwrap()),
        tokens(&algebra)
    );
    let rendered = crate::render_with_capabilities(contract).unwrap();
    let entries: Vec<_> = rendered
        .capabilities
        .circuits
        .iter()
        .filter(|c| c.name == name)
        .collect();
    assert_eq!(entries.len(), 1, "expected exact public export {name}");
    assert!(entries[0].recorded && entries[0].observed_call, "{name}");
}

#[test]
fn retained_public_family_includes_lexical_witness_and_nested_helper_returns() {
    let contract = fixture();
    for name in ["two", "three", "echo", "observed", "nested"] {
        included(&contract, name);
    }
}

#[test]
fn removed_single_slot_and_effect_counts_are_separate_from_old_inclusion() {
    for mode in 0..5 {
        let mut contract = subject();
        match mode {
            0 => contract.stateful_circuits[0].actions.clear(), // no public effects
            1 => contract.stateful_circuits[0].actions = vec![write(literal())], // no reads
            2 => {
                contract.ledger_fields.push(LedgerField {
                    source: None,
                    id: "second".into(),
                    index: 1,
                    path: vec![1],
                    declaration: LedgerFieldKind::Cell { ty: Type::Field },
                });
                contract.stateful_circuits[0]
                    .actions
                    .push(StateAction::CellWrite {
                        field: "second".into(),
                        index: 1,
                        value: read(),
                    });
            }
            3 => {
                let paths = crate::expected_ledger_paths(16);
                contract.ledger_fields[0].path = paths[0].clone();
                for (position, path) in paths.into_iter().enumerate().skip(1) {
                    contract.ledger_fields.push(LedgerField {
                        source: None,
                        id: format!("padding_{position}"),
                        index: path[0],
                        path,
                        declaration: LedgerFieldKind::Cell { ty: Type::Field },
                    });
                }
            }
            4 => {
                // Audit allowed this node, but the legacy shared evaluator only
                // admitted Boolean-valued If in the TerminalReturns domain.
                contract.stateful_circuits[0].return_value = StateReturn::Expression {
                    value: Expr::If {
                        condition: Box::new(parameter("selected")),
                        then: Box::new(parameter("seed")),
                        otherwise: Box::new(literal()),
                    },
                };
            }
            _ => unreachable!(),
        }
        let circuit = &contract.stateful_circuits[0];
        let context = Context::new(&contract);
        assert!(
            context.old(circuit).is_none(),
            "expansion must not masquerade as inclusion"
        );
        let algebra = context.algebra(circuit).unwrap();
        assert_eq!(
            selected_tokens(context.selected(circuit).unwrap()),
            Some(tokens(&algebra))
        );
        let rendered = crate::render_with_capabilities(&contract).unwrap();
        assert_eq!(rendered.capabilities.circuits.len(), 1);
        assert!(rendered.capabilities.circuits[0].recorded);
    }
}

#[test]
fn invalid_scope_slot_and_witness_are_semantic_refusals() {
    let mut cases = vec![];
    let mut slot = subject();
    slot.ledger_fields[0].index = 1;
    cases.push(slot);
    let mut kind = subject();
    kind.ledger_fields[0].declaration = LedgerFieldKind::Counter;
    cases.push(kind);
    let mut scope = subject();
    scope.stateful_circuits[0].actions = vec![
        bind("hidden", Type::Field, read(), write(parameter("hidden"))),
        write(literal()),
    ];
    scope.stateful_circuits[0].return_value = StateReturn::Expression {
        value: parameter("hidden"),
    };
    cases.push(scope);
    let mut witness = subject();
    witness.stateful_circuits[0].return_value = StateReturn::Expression {
        value: Expr::WitnessCall {
            name: "next_value".into(),
            arguments: vec![parameter("selected")],
        },
    };
    cases.push(witness);
    for contract in cases {
        let circuit = &contract.stateful_circuits[0];
        let context = Context::new(&contract);
        assert!(context.old(circuit).is_none());
        assert!(matches!(
            context.algebra(circuit),
            Err(effect_plan::PlanError::Unsupported(_))
        ));
        assert!(context.terminal(circuit).unwrap().is_none());
    }
}
