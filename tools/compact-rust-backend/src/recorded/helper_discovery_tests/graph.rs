// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;
use crate::recorded::helpers::call_graph::CallGraph;

#[test]
fn raw_pure_calls_keep_root_preview_but_only_stateful_declarations_are_edges() {
    let mut c = fixture(Type::Unit, StateReturn::Unit, true);
    c.stateful_circuits[1].actions = vec![StateAction::Expression {
        value: Expr::Call {
            name: "pure".into(),
            arguments: vec![],
        },
    }];
    let graph = CallGraph::from_declarations(&c.stateful_circuits).unwrap();
    assert!(graph.has_calls(1));
    assert_eq!(graph.reachable_from([1]), vec![false, false]);
}

#[test]
fn complete_return_arguments_and_cycles_form_finite_deterministic_closures() {
    let mut c = fixture(Type::Unit, StateReturn::Unit, true);
    let mut child = c.stateful_circuits[0].clone();
    child.name = "child".into();
    c.stateful_circuits.push(child);
    c.stateful_circuits[1].actions.clear();
    c.stateful_circuits[1].return_value = StateReturn::Expression {
        value: Expr::Call {
            name: "callee".into(),
            arguments: vec![Expr::Call {
                name: "child".into(),
                arguments: vec![],
            }],
        },
    };
    c.stateful_circuits[0]
        .actions
        .push(StateAction::CircuitCall {
            name: "child".into(),
            arguments: vec![],
        });
    c.stateful_circuits[2]
        .actions
        .push(StateAction::CircuitCall {
            name: "callee".into(),
            arguments: vec![],
        });
    // Syntactic discovery facts only; invalid arity/cycles need independent checking.
    let graph = CallGraph::from_declarations(&c.stateful_circuits).unwrap();
    assert_eq!(graph.reachable_from([1, 1]), vec![true, false, true]);
    assert_eq!(graph.reachable_from([2]), vec![true, false, true]);
}

#[test]
fn declaration_graph_uses_existing_policy_before_allocating_edges() {
    let mut c = fixture(Type::Unit, StateReturn::Unit, true);
    let (limit, _) = crate::resource_limits::recording_call_fact_limits();
    c.stateful_circuits = vec![c.stateful_circuits[0].clone(); limit + 1];
    assert!(
        matches!(CallGraph::from_declarations(&c.stateful_circuits), Err(RenderError::ResourceLimit { resource: "declarations", observed, limit: actual }) if observed == limit + 1 && actual == limit)
    );
}

#[test]
fn cached_raw_target_budget_is_global_even_for_unknown_names() {
    let c = fixture(Type::Unit, StateReturn::Unit, true);
    let (declarations, nodes) = crate::resource_limits::recording_call_fact_limits();
    let mut circuits = vec![c.stateful_circuits[0].clone(); declarations];
    for (index, circuit) in circuits.iter_mut().enumerate() {
        circuit.name = format!("decl_{index}");
        circuit.actions = (0..(nodes / declarations + 1))
            .map(|target| StateAction::CircuitCall {
                name: format!("unresolved_{target}"),
                arguments: vec![],
            })
            .collect();
    }
    // Defensive direct graph API law, deliberately not native-valid source.
    assert!(
        matches!(CallGraph::from_declarations(&circuits), Err(RenderError::ResourceLimit { resource: "nodes", observed, limit }) if observed > limit && limit == nodes)
    );
}
