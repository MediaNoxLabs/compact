// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Finite qualification of the experimental owner under the existing preflight.
//! The worker requests exactly 2 MiB; this is not an OS stack-size measurement,
//! a memory/time bound, or qualification for untested platforms/toolchains.
//! Child isolation retains a useful test failure if a stack overflow aborts.
use super::*;
use crate::RenderError;
use crate::ir::*;
use std::process::Command;
use std::time::Instant;

mod jubjub;

const STACK_BYTES: usize = 2 * 1024 * 1024;
const CHILD_MODE: &str = "COMPACT_EFFECT_PLAN_RESOURCE_CHILD";

fn contract(body: ReturnPlan) -> Contract {
    Contract {
        schema_version: crate::ir::SCHEMA_VERSION,
        type_aliases: vec![],
        ledger_fields: vec![LedgerField {
            id: "state".into(),
            index: 0,
            path: vec![],
            source: None,
            declaration: LedgerFieldKind::Cell { ty: Type::Field },
        }],
        constructor: None,
        witnesses: vec![],
        circuits: vec![],
        stateful_circuits: vec![StatefulCircuit {
            source: None,
            name: "bounded".into(),
            internal: false,
            parameters: vec![],
            actions: vec![],
            result: Type::Field,
            return_value: StateReturn::Effectful { body },
        }],
    }
}
fn read() -> ReturnPlan {
    ReturnPlan::Value {
        value: Expr::CellRead {
            field: "state".into(),
            index: 0,
        },
    }
}
fn deep(branches: usize) -> Contract {
    let mut body = read();
    for _ in 0..branches {
        body = ReturnPlan::Conditional {
            condition: Expr::Boolean { value: true },
            then: Box::new(body),
            otherwise: Box::new(read()),
        };
    }
    contract(body)
}
fn deep_arithmetic(additions: usize) -> Contract {
    let mut value = Expr::CellRead {
        field: "state".into(),
        index: 0,
    };
    for _ in 0..additions {
        value = Expr::Add {
            left: Box::new(value),
            right: Box::new(Expr::FieldLiteral { value: "1".into() }),
        };
    }
    contract(ReturnPlan::Value { value })
}
fn deep_bindings(bindings: usize) -> Contract {
    let mut body = read();
    for i in 0..bindings {
        body = ReturnPlan::Let {
            bindings: vec![LocalBinding {
                name: format!("v{i}"),
                ty: Type::Field,
                value: Expr::FieldLiteral { value: "1".into() },
            }],
            result: Box::new(body),
        };
    }
    contract(body)
}
fn wide(bindings: usize) -> Contract {
    contract(ReturnPlan::Let {
        bindings: (0..bindings)
            .map(|i| LocalBinding {
                name: format!("v{i}"),
                ty: Type::Field,
                value: Expr::FieldLiteral { value: "1".into() },
            })
            .collect(),
        result: Box::new(read()),
    })
}
fn sibling_branches(branches: usize) -> Contract {
    contract(ReturnPlan::Sequence {
        actions: (0..branches)
            .map(|_| StateAction::If {
                condition: Expr::Boolean { value: true },
                then: Box::new(StateAction::CellWrite {
                    field: "state".into(),
                    index: 0,
                    value: Expr::FieldLiteral { value: "1".into() },
                }),
                otherwise: Box::new(StateAction::CellWrite {
                    field: "state".into(),
                    index: 0,
                    value: Expr::FieldLiteral { value: "2".into() },
                }),
            })
            .collect(),
        result: Box::new(read()),
    })
}
fn preflight(contract: &Contract) -> Result<(), RenderError> {
    crate::resource_limits::validate(contract)?.finish(contract)
}

/// Find the exact admitted width for this family under the conjunctive policy.
/// This does not claim the pending ceiling admits every shape of that width.
fn maximum_width(make: fn(usize) -> Contract) -> usize {
    let (mut accepted, mut refused) = (0, 4097);
    preflight(&make(accepted)).unwrap();
    assert!(matches!(
        preflight(&make(refused)),
        Err(RenderError::ResourceLimit { .. })
    ));
    while refused - accepted > 1 {
        let middle = accepted + (refused - accepted) / 2;
        match preflight(&make(middle)) {
            Ok(()) => accepted = middle,
            Err(RenderError::ResourceLimit { .. }) => refused = middle,
            Err(error) => panic!("unexpected preflight error: {error}"),
        }
    }
    accepted
}
fn qualify(contract: &Contract, label: &str) {
    preflight(contract).expect("accepted preflight boundary");
    let ledger = contract
        .ledger_fields
        .iter()
        .map(|f| (f.id.as_str(), f))
        .collect();
    let witnesses = HashMap::new();
    let started = Instant::now();
    eprintln!("{label}: checking experimental plan");
    let checked = check(&contract.stateful_circuits[0], &ledger, &witnesses).unwrap();
    eprintln!(
        "{label}: check elapsed {:?}; rendering checked plan",
        started.elapsed()
    );
    let started = Instant::now();
    let rendered = render(&checked).unwrap();
    assert!(!rendered.steps.is_empty());
    drop(rendered);
    drop(checked);
    eprintln!(
        "{label}: plan rendering elapsed {:?}; public pipeline",
        started.elapsed()
    );
    let rendered = crate::render_with_capabilities(contract).unwrap();
    assert_eq!(rendered.capabilities.circuits.len(), 1);
    assert!(rendered.capabilities.circuits[0].recorded);
    eprintln!("{label}: public pipeline passed");
}
fn reject(contract: &Contract, resource: &str, limit: usize, observed: Option<usize>) {
    for error in [
        preflight(contract).unwrap_err(),
        crate::render_with_capabilities(contract)
            .err()
            .expect("public refusal"),
    ] {
        let RenderError::ResourceLimit {
            resource: actual,
            limit: actual_limit,
            observed: actual_observed,
        } = error
        else {
            panic!("expected bounded preflight refusal: {error}")
        };
        assert_eq!(actual, resource);
        assert_eq!(actual_limit, limit);
        assert!(actual_observed > actual_limit);
        if let Some(observed) = observed {
            assert_eq!(actual_observed, observed);
        }
    }
}
fn isolated(case: &'static str, work: fn()) {
    if std::env::var(CHILD_MODE).as_deref() == Ok(case) {
        eprintln!(
            "effect plan resource case={case}, requested_stack_bytes={STACK_BYTES}, target={}-{}",
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
        .expect("start isolated resource test");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    eprintln!("{stdout}{stderr}");
    assert!(
        output.status.success(),
        "isolated {case} failed with {}",
        output.status
    );
    assert!(
        stdout.contains("1 passed"),
        "the exact child test must execute"
    );
}
#[test]
fn maximum_depth_on_two_mib_worker() {
    isolated("maximum_depth_on_two_mib_worker", || {
        // StatefulCircuit=1, StateReturn=2, base ReturnPlan=3, Expr=4,
        // field text=5. Forty-three nested ReturnPlans reach exactly 48.
        for (label, make) in [
            ("ReturnPlan branches", deep as fn(usize) -> Contract),
            ("Expr arithmetic", deep_arithmetic),
            ("ReturnPlan lexical scopes", deep_bindings),
        ] {
            qualify(&make(43), label);
            reject(&make(44), "syntax_depth", 48, Some(49));
        }
    });
}
#[test]
fn maximum_binding_width_on_two_mib_worker() {
    isolated("maximum_binding_width_on_two_mib_worker", || {
        let width = maximum_width(wide);
        eprintln!("maximum admitted local-binding width={width}");
        qualify(&wide(width), "maximum binding width");
        reject(&wide(width + 1), "pending_nodes", 4096, Some(4097));
    });
}
#[test]
fn maximum_sibling_branch_width_on_two_mib_worker() {
    isolated("maximum_sibling_branch_width_on_two_mib_worker", || {
        let width = maximum_width(sibling_branches);
        eprintln!("maximum admitted sibling-branch width={width}");
        qualify(&sibling_branches(width), "maximum sibling-branch width");
        // This family reaches the node ceiling before the pending ceiling.
        reject(&sibling_branches(width + 1), "nodes", 32768, None);
    });
}

fn ordinary_continuation(depth: usize, width: usize) -> Contract {
    let snapshot = || LocalBinding {
        name: "snapshot".into(),
        ty: Type::Field,
        value: Expr::CellRead {
            field: "state".into(),
            index: 0,
        },
    };
    let mut action = StateAction::CellWrite {
        field: "state".into(),
        index: 0,
        value: Expr::FieldLiteral { value: "0".into() },
    };
    for _ in 0..depth {
        let mut bindings: Vec<_> = (0..width)
            .map(|i| LocalBinding {
                name: format!("local{i}"),
                ty: Type::Field,
                value: Expr::FieldLiteral { value: "1".into() },
            })
            .collect();
        bindings.push(snapshot());
        action = StateAction::Sequence {
            actions: vec![StateAction::Let {
                bindings,
                action: Box::new(action),
            }],
        };
    }
    let mut contract = contract(read());
    let circuit = &mut contract.stateful_circuits[0];
    circuit.actions = vec![StateAction::Let {
        bindings: vec![snapshot()],
        action: Box::new(action),
    }];
    circuit.return_value = StateReturn::Expression {
        value: Expr::Parameter {
            name: "snapshot".into(),
        },
    };
    contract
}

/// Bracket the first refusal without constructing an arbitrarily deep value.
fn maximum_ordinary_depth() -> usize {
    let make = |depth| ordinary_continuation(depth, 0);
    let (mut accepted, mut refused) = (0, 1);
    preflight(&make(accepted)).unwrap();
    while preflight(&make(refused)).is_ok() {
        accepted = refused;
        refused *= 2;
    }
    while refused - accepted > 1 {
        let middle = accepted + (refused - accepted) / 2;
        match preflight(&make(middle)) {
            Ok(()) => accepted = middle,
            Err(RenderError::ResourceLimit { .. }) => refused = middle,
            Err(error) => panic!("unexpected preflight error: {error}"),
        }
    }
    accepted
}

fn qualify_ordinary_candidate(contract: &Contract) {
    preflight(contract).expect("accepted ordinary continuation boundary");
    let ledger = contract
        .ledger_fields
        .iter()
        .map(|f| (f.id.as_str(), f))
        .collect();
    let checked = check(&contract.stateful_circuits[0], &ledger, &HashMap::new()).unwrap();
    let body = &checked.plan().body;
    let snapshot = body
        .bindings
        .iter()
        .rev()
        .find(|binding| matches!(binding.operation, model::Operation::CellRead(_)))
        .unwrap();
    assert_eq!(
        body.result, snapshot.value,
        "return the innermost prewrite snapshot"
    );
    assert!(matches!(
        body.bindings.last().unwrap().operation,
        model::Operation::CellWrite { .. }
    ));
    assert!(!render(&checked).unwrap().steps.is_empty());
    // This is the direct candidate API, not public ordinary-root algebra routing.
}

#[test]
fn ordinary_continuation_boundaries_on_two_mib_worker() {
    isolated("ordinary_continuation_boundaries_on_two_mib_worker", || {
        let depth = maximum_ordinary_depth();
        let width = maximum_width(|width| ordinary_continuation(1, width));
        eprintln!("ordinary candidate maximum continuation depth={depth}, local width={width}");
        for (accepted, next) in [
            (
                ordinary_continuation(depth, 0),
                ordinary_continuation(depth + 1, 0),
            ),
            (
                ordinary_continuation(1, width),
                ordinary_continuation(1, width + 1),
            ),
        ] {
            qualify_ordinary_candidate(&accepted);
            // The next member must be rejected before candidate check/render is called.
            assert!(matches!(
                preflight(&next),
                Err(RenderError::ResourceLimit { .. })
            ));
        }
    });
}
