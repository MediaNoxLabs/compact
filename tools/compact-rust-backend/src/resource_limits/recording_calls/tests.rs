// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Placement laws over typed IR, not claims that every synthetic combination
//! is frontend-valid or recordable. Declaration resolution belongs to helpers.
use super::*;
use crate::ir::{SourceLocation, StateReturn, Type};
use crate::resource_limits::Kind;
use serde_json::{Value, json};

fn call(name: &str) -> Value {
    json!({"kind":"call", "name":name, "arguments":[]})
}

fn root(actions: Vec<Value>, return_value: Value) -> StatefulCircuit {
    serde_json::from_value(json!({
        "name":"root", "actions":actions,
        "result":{"kind":"unit"}, "return_value":return_value
    }))
    .unwrap()
}

fn expression(value: Value) -> StatefulCircuit {
    root(vec![], json!({"kind":"expression", "value":value}))
}

fn assert_targets(circuit: &StatefulCircuit, names: &[&str]) {
    assert_eq!(
        recording_call_targets(circuit).unwrap(),
        names.iter().copied().collect::<HashSet<_>>(),
        "{}",
        serde_json::to_string(circuit).unwrap()
    );
}

mod expression_carriers;
mod state_carriers;

#[test]
fn graph_fact_limits_reuse_the_existing_policy() {
    assert_eq!(
        recording_call_fact_limits(),
        (Limits::DEFAULT.declarations, Limits::DEFAULT.nodes)
    );
}

#[test]
fn call_kind_is_source_fact_not_registry_resolution() {
    let circuit = root(
        vec![
            json!({"kind":"circuit_call", "name":"unknown", "arguments":[call("arg")]}),
            json!({"kind":"pure_call", "name":"pure_statement", "arguments":[call("pure_arg")]}),
            json!({"kind":"native_witness_call", "builtin":"own_public_key"}),
        ],
        json!({"kind":"expression", "value":{"kind":"tuple", "elements":[
            call("pure_or_stateful"), call("unknown"),
            {"kind":"witness_call", "name":"witness", "arguments":[call("witness_arg")]},
            {"kind":"parameter", "name":"not_a_call"},
            {"kind":"native_witness_call", "builtin":"own_public_key"},
            {"kind":"vector_fold_call", "name":"pure_fold", "initial":call("initial"),
             "source":call("source"), "accumulator":{"kind":"field"},
             "element":{"kind":"field"}, "length":1}
        ]}}),
    );
    assert_targets(
        &circuit,
        &[
            "unknown",
            "arg",
            "pure_arg",
            "pure_or_stateful",
            "witness_arg",
            "initial",
            "source",
        ],
    );
}

#[test]
fn empty_source_and_repeated_targets_are_finite_borrowed_facts() {
    assert_targets(&root(vec![], json!({"kind":"unit"})), &[]);
    let circuit = expression(json!({"kind":"call", "name":"same", "arguments":[call("same")]}));
    let facts = recording_call_targets(&circuit).unwrap();
    assert_eq!(facts.len(), 1);
    let StateReturn::Expression {
        value: Expr::Call { name, arguments },
    } = &circuit.return_value
    else {
        panic!("fixture")
    };
    // Either equal raw source occurrence is valid; HashSet retains the first
    // observed borrowed name. No source string is allocated by discovery.
    let Expr::Call { name: inner, .. } = &arguments[0] else {
        panic!("fixture")
    };
    let fact = facts.iter().next().unwrap();
    assert!(fact.as_ptr() == name.as_ptr() || fact.as_ptr() == inner.as_ptr());
}

#[test]
fn wide_source_is_refused_before_unbounded_fact_allocation() {
    let circuit = root(
        (0..=Limits::DEFAULT.pending)
            .map(|i| json!({"kind":"circuit_call", "name":format!("c{i}"), "arguments":[]}))
            .collect(),
        json!({"kind":"unit"}),
    );
    let error = measure_targets(&circuit, Limits::DEFAULT).unwrap_err();
    assert_eq!(error.resource, Kind::PendingNodes);
}

#[test]
fn deeply_nested_source_is_refused_on_ordinary_worker() {
    let mut value = Expr::Call {
        name: "deep".into(),
        arguments: vec![],
    };
    for _ in 0..Limits::DEFAULT.syntax_depth {
        value = Expr::FieldCast {
            value: Box::new(value),
        };
    }
    let circuit = StatefulCircuit {
        source: None,
        name: "root".into(),
        internal: false,
        parameters: vec![],
        actions: vec![],
        result: Type::Field,
        return_value: StateReturn::Expression { value },
    };
    let error = measure_targets(&circuit, Limits::DEFAULT).unwrap_err();
    assert_eq!(error.resource, Kind::SyntaxDepth);
}

#[test]
fn bounded_nested_arguments_succeed_on_ordinary_worker() {
    let mut value = Expr::Call {
        name: "leaf".into(),
        arguments: vec![],
    };
    for i in 0..8 {
        value = Expr::Call {
            name: format!("outer{i}"),
            arguments: vec![value],
        };
    }
    let circuit = StatefulCircuit {
        source: None,
        name: "root".into(),
        internal: false,
        parameters: vec![],
        actions: vec![],
        result: Type::Field,
        return_value: StateReturn::Expression { value },
    };
    let facts = recording_call_targets(&circuit).unwrap();
    assert_eq!(facts.len(), 9);
    assert!(facts.contains("leaf"));
    for i in 0..8 {
        assert!(facts.contains(format!("outer{i}").as_str()));
    }
}

#[test]
fn independent_source_budgets_and_location_are_preserved() {
    let mut circuit = expression(call("helper"));
    let mut limits = Limits::CENSUS;
    limits.nodes = 1;
    assert_eq!(
        measure_targets(&circuit, limits).unwrap_err().resource,
        Kind::Nodes
    );
    limits = Limits::CENSUS;
    limits.strings = 0;
    assert_eq!(
        measure_targets(&circuit, limits).unwrap_err().resource,
        Kind::StringBytes
    );
    limits = Limits::CENSUS;
    limits.path_units = 0;
    assert_eq!(
        measure_targets(&circuit, limits).unwrap_err().resource,
        Kind::NativePath
    );
    circuit.return_value = StateReturn::Expression {
        value: Expr::BytesLiteral {
            bytes: vec![0; Limits::DEFAULT.literal_bytes + 1],
        },
    };
    circuit.source = Some(SourceLocation {
        file: "calls.compact".into(),
        line: 7,
        column: 3,
    });
    let crate::RenderError::Located { location, error } =
        recording_call_targets(&circuit).unwrap_err()
    else {
        panic!("source location lost")
    };
    assert_eq!(location, circuit.source.unwrap());
    assert!(matches!(
        *error,
        crate::RenderError::ResourceLimit {
            resource: "literal_bytes",
            ..
        }
    ));
}
