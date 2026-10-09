// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Finite source transformations, not an evaluator or an execution oracle.
use crate::ir::*;
use std::collections::HashMap;

pub(super) fn parameter(name: &str) -> Expr {
    Expr::Parameter { name: name.into() }
}
pub(super) fn number(value: u64) -> Expr {
    Expr::FieldLiteral {
        value: value.to_string(),
    }
}
pub(super) fn add(left: Expr, right: Expr) -> Expr {
    Expr::Add {
        left: Box::new(left),
        right: Box::new(right),
    }
}
pub(super) fn equal(left: Expr, right: Expr) -> Expr {
    Expr::Equal {
        left: Box::new(left),
        right: Box::new(right),
    }
}
pub(super) fn local(name: &str, value: Expr) -> LocalBinding {
    LocalBinding {
        name: name.into(),
        ty: Type::Field,
        value,
    }
}
pub(super) fn returned(value: Expr) -> ReturnPlan {
    ReturnPlan::Value { value }
}
pub(super) fn write(value: Expr) -> StateAction {
    StateAction::CellWrite {
        field: "state".into(),
        index: 0,
        value,
    }
}
pub(super) fn read() -> Expr {
    Expr::CellRead {
        field: "state".into(),
        index: 0,
    }
}
pub(super) fn contract(body: ReturnPlan) -> Contract {
    Contract {
        schema_version: SCHEMA_VERSION,
        type_aliases: vec![],
        constructor: None,
        circuits: vec![],
        ledger_fields: vec![LedgerField {
            id: "state".into(),
            index: 0,
            path: vec![],
            source: None,
            declaration: LedgerFieldKind::Cell { ty: Type::Field },
        }],
        witnesses: vec![WitnessDeclaration {
            source: None,
            name: "mark".into(),
            parameters: vec![Parameter {
                name: "value".into(),
                ty: Type::Field,
            }],
            result: Type::Field,
        }],
        stateful_circuits: vec![StatefulCircuit {
            source: None,
            name: "sample".into(),
            internal: false,
            parameters: vec![Parameter {
                name: "x".into(),
                ty: Type::Field,
            }],
            actions: vec![],
            result: Type::Field,
            return_value: StateReturn::Effectful { body },
        }],
    }
}
struct Sequence(u64);
impl Sequence {
    fn choose(&mut self, upper: u64) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 32) % upper
    }
    fn field(&mut self, depth: usize) -> Expr {
        if depth == 0 {
            return match self.choose(5) {
                0 => parameter("x"),
                1 => parameter("keep"),
                2 => read(),
                3 => Expr::WitnessCall {
                    name: "mark".into(),
                    arguments: vec![parameter("x")],
                },
                _ => number(self.choose(17)),
            };
        }
        match self.choose(5) {
            0 => add(self.field(depth - 1), self.field(depth - 1)),
            1 => Expr::If {
                condition: Box::new(equal(parameter("x"), number(self.choose(3)))),
                then: Box::new(self.field(depth - 1)),
                otherwise: Box::new(self.field(depth - 1)),
            },
            2 => Expr::Let {
                bindings: vec![local("x", self.field(depth - 1))],
                body: Box::new(add(parameter("x"), self.field(depth - 1))),
            },
            3 => Expr::Let {
                bindings: vec![
                    local("x", add(parameter("x"), number(1))),
                    local("x", add(parameter("x"), number(2))),
                ],
                body: Box::new(self.field(depth - 1)),
            },
            _ => Expr::Sequence {
                steps: vec![Expr::Assert {
                    condition: Box::new(equal(parameter("keep"), parameter("keep"))),
                    message: "retained value".into(),
                }],
                value: Box::new(self.field(depth - 1)),
            },
        }
    }
}

/// 32 fixed seeds, depth at most three; no recursion/call expansion generator.
pub(super) fn generated(seed: u64) -> Contract {
    let mut choices = Sequence(seed);
    let depth = 1 + (seed as usize % 3);
    contract(ReturnPlan::Let {
        bindings: vec![local("keep", parameter("x"))],
        result: Box::new(ReturnPlan::Conditional {
            condition: equal(parameter("x"), number(0)),
            then: Box::new(ReturnPlan::Sequence {
                actions: vec![write(choices.field(depth))],
                result: Box::new(returned(choices.field(depth))),
            }),
            otherwise: Box::new(ReturnPlan::Let {
                bindings: vec![local("x", choices.field(depth))],
                result: Box::new(returned(add(parameter("x"), parameter("keep")))),
            }),
        }),
    })
}

/// Rename declarations and only the references resolved to those declarations.
/// Fresh names are globally distinct; each lexical child receives its own map.
pub(super) fn alpha_rename(contract: &Contract) -> Contract {
    let mut output = contract.clone();
    let mut next = 0;
    let circuit = &mut output.stateful_circuits[0];
    let mut scope = HashMap::new();
    for parameter in &mut circuit.parameters {
        let fresh = format!("renamed_{next}");
        next += 1;
        scope.insert(parameter.name.clone(), fresh.clone());
        parameter.name = fresh;
    }
    let StateReturn::Effectful { body } = &mut circuit.return_value else {
        panic!("corpus return form")
    };
    rename_return(body, &scope, &mut next);
    output
}
fn rename_locals(
    bindings: &mut [LocalBinding],
    incoming: &HashMap<String, String>,
    next: &mut usize,
) -> HashMap<String, String> {
    let mut scope = incoming.clone();
    for binding in bindings {
        rename_expression(&mut binding.value, &scope, next);
        let fresh = format!("renamed_{next}");
        *next += 1;
        scope.insert(binding.name.clone(), fresh.clone());
        binding.name = fresh;
    }
    scope
}
fn rename_expression(value: &mut Expr, scope: &HashMap<String, String>, next: &mut usize) {
    match value {
        Expr::Parameter { name } => {
            *name = scope
                .get(name)
                .expect("generator creates bound references")
                .clone()
        }
        Expr::Add { left, right } | Expr::Equal { left, right } => {
            rename_expression(left, scope, next);
            rename_expression(right, scope, next);
        }
        Expr::If {
            condition,
            then,
            otherwise,
        } => {
            for child in [condition, then, otherwise] {
                rename_expression(child, scope, next);
            }
        }
        Expr::Let { bindings, body } => {
            let scope = rename_locals(bindings, scope, next);
            rename_expression(body, &scope, next);
        }
        Expr::Sequence { steps, value } => {
            for step in steps {
                rename_expression(step, scope, next);
            }
            rename_expression(value, scope, next);
        }
        Expr::Assert { condition, .. } => rename_expression(condition, scope, next),
        Expr::WitnessCall { arguments, .. } => {
            for arg in arguments {
                rename_expression(arg, scope, next);
            }
        }
        Expr::FieldLiteral { .. } | Expr::CellRead { .. } | Expr::Boolean { .. } => {}
        _ => panic!("alpha-renaming corpus must explicitly cover its source variant"),
    }
}
fn rename_action(action: &mut StateAction, scope: &HashMap<String, String>, next: &mut usize) {
    match action {
        StateAction::CellWrite { value, .. } => rename_expression(value, scope, next),
        StateAction::Sequence { actions } => {
            for action in actions {
                rename_action(action, scope, next);
            }
        }
        StateAction::Let { bindings, action } => {
            let scope = rename_locals(bindings, scope, next);
            rename_action(action, &scope, next);
        }
        StateAction::Assert { condition, .. } => rename_expression(condition, scope, next),
        _ => panic!("alpha-renaming corpus action coverage"),
    }
}
fn rename_return(body: &mut ReturnPlan, scope: &HashMap<String, String>, next: &mut usize) {
    match body {
        ReturnPlan::Value { value } => rename_expression(value, scope, next),
        ReturnPlan::Let { bindings, result } => {
            let scope = rename_locals(bindings, scope, next);
            rename_return(result, &scope, next);
        }
        ReturnPlan::Sequence { actions, result } => {
            for action in actions {
                rename_action(action, scope, next);
            }
            rename_return(result, scope, next);
        }
        ReturnPlan::Conditional {
            condition,
            then,
            otherwise,
        } => {
            rename_expression(condition, scope, next);
            rename_return(then, scope, next);
            rename_return(otherwise, scope, next);
        }
    }
}
