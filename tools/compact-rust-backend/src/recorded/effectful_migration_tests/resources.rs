// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Qualify legacy-admitted inputs at the existing preflight edges. These tests
//! request a 2 MiB worker stack; they do not measure the OS stack or establish a
//! universal memory/time bound. Child isolation preserves stack-abort evidence.
use super::*;
use std::process::Command;

const STACK_BYTES: usize = 2 * 1024 * 1024;
const CHILD_MODE: &str = "COMPACT_EFFECTFUL_MIGRATION_RESOURCE_CHILD";

fn read() -> ReturnPlan {
    ReturnPlan::Value {
        value: Expr::CellRead {
            field: "state".into(),
            index: 0,
        },
    }
}

fn admitted_body() -> ReturnPlan {
    ReturnPlan::Conditional {
        condition: Expr::Boolean { value: true },
        then: Box::new(ReturnPlan::Sequence {
            actions: vec![StateAction::CellWrite {
                field: "state".into(),
                index: 0,
                value: Expr::FieldLiteral { value: "1".into() },
            }],
            result: Box::new(read()),
        }),
        otherwise: Box::new(read()),
    }
}

fn candidate(body: ReturnPlan) -> Contract {
    let mut contract = effectful_fixture();
    contract.stateful_circuits[0].source = None;
    contract.stateful_circuits[0].return_value = StateReturn::Effectful { body };
    contract
}

fn deep(branches: usize) -> Contract {
    let mut body = admitted_body();
    for _ in 0..branches {
        body = ReturnPlan::Conditional {
            condition: Expr::Boolean { value: true },
            then: Box::new(body),
            otherwise: Box::new(read()),
        };
    }
    candidate(body)
}

fn wide(bindings: usize) -> Contract {
    candidate(ReturnPlan::Let {
        bindings: (0..bindings)
            .map(|i| LocalBinding {
                name: format!("local_{i}"),
                ty: Type::Field,
                value: Expr::FieldLiteral { value: "1".into() },
            })
            .collect(),
        result: Box::new(admitted_body()),
    })
}

fn preflight(contract: &Contract) -> Result<(), RenderError> {
    crate::resource_limits::validate(contract)?.finish(contract)
}

/// The boundary is shape-specific under the conjunctive depth/node/work policy.
fn maximum(make: fn(usize) -> Contract, upper: usize) -> usize {
    let (mut accepted, mut refused) = (0, upper);
    preflight(&make(accepted)).unwrap();
    assert!(matches!(
        preflight(&make(refused)),
        Err(RenderError::ResourceLimit { .. })
    ));
    while refused - accepted > 1 {
        let midpoint = accepted + (refused - accepted) / 2;
        match preflight(&make(midpoint)) {
            Ok(()) => accepted = midpoint,
            Err(RenderError::ResourceLimit { .. }) => refused = midpoint,
            Err(error) => panic!("unexpected preflight error: {error}"),
        }
    }
    accepted
}

fn qualify(contract: &Contract) {
    preflight(contract).expect("accepted preflight edge");
    let context = Context::new(contract);
    let circuit = &contract.stateful_circuits[0];
    // Establish membership in both renderers without retaining either syntax
    // tree while the public pipeline exercises its production selection/drop.
    drop(
        context
            .legacy(circuit)
            .expect("boundary must remain legacy-admitted"),
    );
    drop(
        context
            .algebra(circuit)
            .expect("boundary must be in the migrated intersection"),
    );
    let rendered = crate::render_with_capabilities(contract).unwrap();
    assert_eq!(rendered.capabilities.circuits.len(), 1);
    assert!(rendered.capabilities.circuits[0].recorded);
    drop(rendered);
}

fn reject(contract: &Contract, resource: &str, expected_limit: usize) {
    for error in [
        preflight(contract).unwrap_err(),
        crate::render_with_capabilities(contract)
            .err()
            .expect("public preflight refusal"),
    ] {
        let RenderError::ResourceLimit {
            resource: actual,
            limit,
            observed,
        } = error
        else {
            panic!("expected preflight resource refusal: {error}")
        };
        assert_eq!(actual, resource);
        assert_eq!(limit, expected_limit);
        assert_eq!(observed, limit + 1);
    }
}

fn isolated(case: &'static str, work: fn()) {
    if std::env::var(CHILD_MODE).as_deref() == Ok(case) {
        eprintln!(
            "migration resource case={case}, requested_stack_bytes={STACK_BYTES}, target={}-{}",
            std::env::consts::ARCH,
            std::env::consts::OS
        );
        std::thread::Builder::new()
            .name(case.into())
            .stack_size(STACK_BYTES)
            .spawn(work)
            .expect("start bounded worker")
            .join()
            .expect("bounded worker");
        return;
    }
    let output = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            &format!("{}::{case}", module_path!().split_once("::").unwrap().1),
            "--nocapture",
        ])
        .env(CHILD_MODE, case)
        .env_remove("RUST_MIN_STACK")
        .output()
        .expect("isolated resource test");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    eprintln!("{stdout}{stderr}");
    assert!(
        output.status.success(),
        "isolated {case} failed with {}",
        output.status
    );
    assert!(stdout.contains("1 passed"), "exact child test must execute");
}

#[test]
fn legacy_admitted_depth_edge_on_two_mib_worker() {
    isolated("legacy_admitted_depth_edge_on_two_mib_worker", || {
        let depth = maximum(deep, 64);
        eprintln!("maximum admitted additional ReturnPlan branch depth={depth}");
        qualify(&deep(depth));
        reject(&deep(depth + 1), "syntax_depth", 48);
    });
}

#[test]
fn legacy_admitted_binding_edge_on_two_mib_worker() {
    isolated("legacy_admitted_binding_edge_on_two_mib_worker", || {
        let width = maximum(wide, 4097);
        eprintln!("maximum admitted migration local-binding width={width}");
        qualify(&wide(width));
        reject(&wide(width + 1), "pending_nodes", 4096);
    });
}
