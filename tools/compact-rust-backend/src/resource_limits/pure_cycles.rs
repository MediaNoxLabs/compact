// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//  	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use super::{GraphStatus, Limits, measure};
use crate::ir::{Contract, Expr, Parameter, PureCircuit, SourceLocation, StateAction, Type};
use crate::{RenderError, render};

fn field() -> Expr {
    Expr::FieldLiteral { value: "1".into() }
}

fn call(name: &str) -> Expr {
    Expr::Call {
        name: name.into(),
        arguments: vec![],
    }
}

fn circuit(name: &str, body: Expr) -> PureCircuit {
    PureCircuit {
        source: None,
        name: name.into(),
        internal: false,
        parameters: vec![],
        result: Type::Field,
        body,
    }
}

fn contract(circuits: Vec<PureCircuit>) -> Contract {
    Contract {
        schema_version: 20,
        type_aliases: vec![],
        ledger_fields: vec![],
        constructor: None,
        witnesses: vec![],
        circuits,
        stateful_circuits: vec![],
    }
}

#[test]
fn pure_self_cycle_refuses_with_source_location() {
    let mut c = contract(vec![circuit("again", call("again"))]);
    let source = SourceLocation {
        file: "cycle.compact".into(),
        line: 3,
        column: 8,
    };
    c.circuits[0].source = Some(source.clone());
    assert_eq!(
        measure(&c, Limits::DEFAULT).unwrap().graph_status,
        GraphStatus::PureCycle(0)
    );
    assert_eq!(
        render(&c),
        Err(RenderError::Located {
            location: source,
            error: Box::new(RenderError::RecursivePureCall("again".into())),
        })
    );
    assert_eq!(
        RenderError::RecursivePureCall("again".into()).to_string(),
        "recursive pure circuit call involving \"again\""
    );
}

#[test]
fn mutual_cycle_refuses_in_either_declaration_order() {
    let mut c = contract(vec![
        circuit("first", call("second")),
        circuit("second", call("first")),
    ]);
    for _ in 0..2 {
        assert_eq!(
            render(&c),
            Err(RenderError::RecursivePureCall(c.circuits[0].name.clone()))
        );
        c.circuits.reverse();
    }
}

#[test]
fn unused_private_cycle_is_not_admitted() {
    let mut hidden = circuit("hidden", call("hidden"));
    hidden.internal = true;
    let c = contract(vec![circuit("entry", field()), hidden]);
    assert_eq!(
        render(&c),
        Err(RenderError::RecursivePureCall("hidden".into()))
    );
}

#[test]
fn shared_helpers_are_a_dag_not_a_cycle() {
    let c = contract(vec![
        circuit(
            "entry",
            Expr::Add {
                left: Box::new(call("left")),
                right: Box::new(call("right")),
            },
        ),
        circuit("left", call("leaf")),
        circuit(
            "right",
            Expr::Add {
                left: Box::new(call("leaf")),
                right: Box::new(call("leaf")),
            },
        ),
        circuit("leaf", field()),
    ]);
    assert_eq!(
        measure(&c, Limits::DEFAULT).unwrap().graph_status,
        GraphStatus::Acyclic
    );
    let source = render(&c).expect("shared and repeated calls must remain renderable");
    syn::parse_file(&source).expect("accepted DAG renders valid Rust syntax");
}

#[test]
fn fold_callback_edges_participate_in_cycle_refusal() {
    let mut fold = circuit(
        "fold",
        Expr::VectorFoldCall {
            name: "fold".into(),
            initial: Box::new(field()),
            source: Box::new(Expr::Vector {
                element: Type::Field,
                elements: vec![field()],
            }),
            accumulator: Type::Field,
            element: Type::Field,
            length: 1,
        },
    );
    fold.parameters = ["acc", "item"]
        .map(|name| Parameter {
            name: name.into(),
            ty: Type::Field,
        })
        .to_vec();
    assert_eq!(
        render(&contract(vec![fold])),
        Err(RenderError::RecursivePureCall("fold".into()))
    );
}

#[test]
fn known_semantic_refusals_precede_deferred_cycle() {
    let mut c = contract(vec![
        circuit("again", call("again")),
        circuit("other", call("missing")),
    ]);
    assert_eq!(
        render(&c),
        Err(RenderError::UnknownCircuit("missing".into()))
    );
    c.circuits[1] = circuit("again", field());
    assert_eq!(
        render(&c),
        Err(RenderError::DuplicateCircuit("again".into()))
    );
    c.circuits.pop();
    c.circuits[0].parameters.push(Parameter {
        name: "x".into(),
        ty: Type::Field,
    });
    assert_eq!(
        render(&c),
        Err(RenderError::ArgumentCount {
            circuit: "again".into(),
            expected: 1,
            actual: 0
        })
    );
}

#[test]
fn schema_and_local_resource_checks_still_win() {
    let mut c = contract(vec![circuit("again", call("again"))]);
    c.schema_version = 0;
    assert_eq!(render(&c), Err(RenderError::SchemaVersion(0)));
    c.schema_version = 20;
    c.circuits[0].name = "x".repeat(Limits::DEFAULT.strings + 1);
    assert!(matches!(
        render(&c),
        Err(RenderError::ResourceLimit {
            resource: "string_bytes",
            ..
        })
    ));
}

#[test]
fn stateful_and_mixed_cycle_diagnostics_remain_owned_by_renderer() {
    let mut c = contract(vec![circuit("again", call("again"))]);
    c.stateful_circuits = (0..2)
        .map(|i| crate::ir::StatefulCircuit {
            source: None,
            name: format!("c{i}"),
            internal: false,
            parameters: vec![],
            result: Type::Unit,
            actions: vec![StateAction::CircuitCall {
                name: "c1".into(),
                arguments: vec![],
            }],
            return_value: crate::ir::StateReturn::Unit,
        })
        .collect();
    c.stateful_circuits[1].actions = vec![StateAction::CircuitCall {
        name: "c0".into(),
        arguments: vec![],
    }];
    assert_eq!(
        render(&c),
        Err(RenderError::UnsupportedStatefulCall("c0".into()))
    );
    c.circuits[0].body = call("c0");
    c.stateful_circuits.truncate(1);
    c.stateful_circuits[0].actions = vec![StateAction::PureCall {
        name: "again".into(),
        arguments: vec![],
    }];
    assert_eq!(
        measure(&c, Limits::DEFAULT).unwrap().graph_status,
        GraphStatus::Cycle
    );
    assert_eq!(render(&c), Err(RenderError::UnknownCircuit("c0".into())));
}

#[test]
fn declaration_limit_cycle_is_bounded_on_an_ordinary_worker() {
    // Only render/refuse IR: never execute any generated recursive function.
    let count = Limits::DEFAULT.declarations;
    let c = contract(
        (0..count)
            .map(|i| circuit(&format!("c{i}"), call(&format!("c{}", (i + 1) % count))))
            .collect(),
    );
    assert_eq!(render(&c), Err(RenderError::RecursivePureCall("c0".into())));
}
