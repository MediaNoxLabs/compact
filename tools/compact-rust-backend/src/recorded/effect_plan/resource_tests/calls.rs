// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Finite scoped-call qualification on the existing requested 2 MiB worker.
//! Admission boundaries are discovered from public preflight, not guessed from
//! a single depth limit. This does not measure actual stack high-water marks.
use super::*;

#[derive(Clone, Copy, Debug)]
enum Shape {
    Linear,
    Conditional,
    Diamond,
}
fn input() -> Expr {
    Expr::Parameter {
        name: "selected".into(),
    }
}
fn read_value() -> Expr {
    Expr::CellRead {
        field: "state".into(),
        index: 0,
    }
}
fn call(name: String) -> Expr {
    Expr::Call {
        name,
        arguments: vec![input()],
    }
}
fn chain(edges: usize, shape: Shape) -> Contract {
    let mut result = contract(read());
    result.stateful_circuits.clear();
    for index in 0..=edges {
        let value = if index == edges {
            read_value()
        } else {
            let next = || call(format!("helper_{:03}", index + 1));
            match shape {
                Shape::Linear => next(),
                Shape::Conditional => Expr::If {
                    condition: Box::new(input()),
                    then: Box::new(next()),
                    otherwise: Box::new(read_value()),
                },
                Shape::Diamond => Expr::Add {
                    left: Box::new(next()),
                    right: Box::new(next()),
                },
            }
        };
        result.stateful_circuits.push(StatefulCircuit {
            source: None,
            name: format!("helper_{index:03}"),
            internal: index != 0,
            parameters: vec![Parameter {
                name: "selected".into(),
                ty: Type::Boolean,
            }],
            actions: vec![],
            result: Type::Field,
            // Explicit returns ensure the default algebra route owns every
            // declaration; this test does not depend on ordinary-root rollout.
            return_value: StateReturn::Effectful {
                body: ReturnPlan::Value { value },
            },
        });
    }
    result
}
fn maximum_edges(shape: Shape) -> usize {
    let mut maximum = 0;
    preflight(&chain(maximum, shape)).unwrap();
    // All three families necessarily hit at least the shared call-depth limit.
    for edges in 1..=crate::resource_limits::recording_call_limits().0 + 1 {
        match preflight(&chain(edges, shape)) {
            Ok(()) => maximum = edges,
            Err(RenderError::ResourceLimit { .. }) => return maximum,
            Err(error) => panic!("unexpected {shape:?} preflight result: {error}"),
        }
    }
    panic!("call chain exceeded its public depth budget without refusal")
}
fn qualify_calls(contract: &Contract, label: &str) {
    preflight(contract).expect("accepted call-family boundary");
    let ledger = contract
        .ledger_fields
        .iter()
        .map(|f| (f.id.as_str(), f))
        .collect();
    let circuits = contract
        .stateful_circuits
        .iter()
        .map(|c| (c.name.as_str(), c))
        .collect();
    let checked = check_with_calls(
        &contract.stateful_circuits[0],
        &ledger,
        &HashMap::new(),
        &HashMap::new(),
        &circuits,
    )
    .unwrap();
    eprintln!("{label}: checked scoped call plan");
    assert!(!render(&checked).unwrap().steps.is_empty());
    drop(checked);
    eprintln!("{label}: rendered scoped call plan; running public pipeline");
    let output = crate::render_with_capabilities(contract).unwrap();
    let roots: Vec<_> = output
        .capabilities
        .circuits
        .iter()
        .filter(|c| c.name == "helper_000")
        .collect();
    assert_eq!(roots.len(), 1);
    assert!(roots[0].recorded);
    eprintln!("{label}: public pipeline passed");
}
fn refuse_before_expansion(contract: &Contract) -> &'static str {
    let preflight_error = preflight(contract).unwrap_err();
    let RenderError::ResourceLimit {
        resource,
        observed,
        limit,
    } = preflight_error
    else {
        panic!("expected resource refusal: {preflight_error}")
    };
    assert!(observed > limit);
    for candidate in [contract.clone(), {
        let mut malformed = contract.clone();
        // Source-body/result mismatch would fail later semantic checking.
        // Resource refusal must win before any recursive compiler owner runs.
        malformed.stateful_circuits[0].result = Type::Boolean;
        malformed
    }] {
        let error = crate::render_with_capabilities(&candidate)
            .err()
            .expect("public refusal");
        assert!(
            matches!(error, RenderError::ResourceLimit { resource: actual, .. } if actual == resource),
            "{error}"
        );
    }
    eprintln!("refused before expansion: resource={resource}, observed={observed}, limit={limit}");
    resource
}

#[test]
fn call_chain_boundary_on_two_mib_worker() {
    isolated("calls::call_chain_boundary_on_two_mib_worker", || {
        let maximum = maximum_edges(Shape::Linear);
        assert!(maximum > 0);
        eprintln!("linear call chain admitted edges={maximum}");
        qualify_calls(&chain(maximum, Shape::Linear), "linear call chain");
        refuse_before_expansion(&chain(maximum + 1, Shape::Linear));
    });
}

#[test]
fn mixed_branch_call_boundary_on_two_mib_worker() {
    isolated(
        "calls::mixed_branch_call_boundary_on_two_mib_worker",
        || {
            let maximum = maximum_edges(Shape::Conditional);
            assert!(maximum > 0);
            eprintln!("mixed If/call chain admitted edges={maximum}");
            qualify_calls(&chain(maximum, Shape::Conditional), "mixed If/call chain");
            refuse_before_expansion(&chain(maximum + 1, Shape::Conditional));
        },
    );
}

#[test]
fn diamond_call_work_is_refused_before_expansion() {
    isolated(
        "calls::diamond_call_work_is_refused_before_expansion",
        || {
            // A finite small positive control; the rejected DAG remains compact
            // and is never recursively expanded by lower/check/render.
            qualify_calls(&chain(2, Shape::Diamond), "small diamond calls");
            let maximum = maximum_edges(Shape::Diamond);
            let next = chain(maximum + 1, Shape::Diamond);
            assert_eq!(refuse_before_expansion(&next), "expanded_work");
            eprintln!(
                "diamond preflight admitted edges={maximum}; next rejected (no claim of rendering maximum diamond)"
            );
        },
    );
}
