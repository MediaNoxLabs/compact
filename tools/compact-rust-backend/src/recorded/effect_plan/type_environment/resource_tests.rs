// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Requested-stack qualification of bounded collection and exact type matching.
use super::*;
use crate::ir::{Parameter, StateReturn};
fn source(depth: usize) -> StatefulCircuit {
    let mut ty = Type::Field;
    for level in 0..depth {
        ty = Type::Struct {
            name: format!("Level{level}"),
            fields: vec![StructField {
                name: "value".into(),
                ty,
            }],
        };
    }
    StatefulCircuit {
        source: None,
        name: "root".into(),
        internal: false,
        parameters: vec![Parameter {
            name: "value".into(),
            ty,
        }],
        result: Type::Unit,
        actions: vec![],
        return_value: StateReturn::Unit,
    }
}
fn collect(root: &StatefulCircuit) -> Result<TypeEnvironment, RecordingGap> {
    TypeEnvironment::from_sources(
        root,
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
    )
}
fn work() {
    owned_pipeline_boundary();
    let mut maximum = 0;
    for depth in 1..=64 {
        let root = source(depth);
        match collect(&root) {
            Ok(environment) => {
                environment.validate(&root.parameters[0].ty).unwrap();
                maximum = depth;
            }
            Err(gap) => {
                assert!(gap.detail.contains("compiler resource"), "{gap:?}");
                break;
            }
        }
    }
    assert!(maximum > 0 && maximum < 64);
    let accepted = source(maximum);
    collect(&accepted)
        .unwrap()
        .validate(&accepted.parameters[0].ty)
        .unwrap();
    assert!(
        collect(&source(maximum + 1))
            .unwrap_err()
            .detail
            .contains("compiler resource")
    );
    eprintln!(
        "canonical nested Struct source admitted depth={maximum}; next refused before collectors"
    );
    let limit = crate::resource_limits::recording_fixed_layout_limit();
    for (length, accepted) in [(limit - 1, true), (limit, false), (usize::MAX, false)] {
        let ty = Type::Vector {
            element: Box::new(Type::OpaqueString),
            length,
        };
        let mut root = source(0);
        root.parameters[0].ty = ty;
        let env = collect(&root).unwrap();
        assert_eq!(env.validate(&root.parameters[0].ty).is_ok(), accepted);
    }
    eprintln!(
        "fixed owned layout admitted slots={limit}; next refused on bounded type validation (not generated runtime allocation)"
    );
}
#[test]
fn canonical_type_boundary_on_two_mib_worker() {
    const CHILD: &str = "COMPACT_TYPE_ENVIRONMENT_RESOURCE_CHILD";
    const STACK: usize = 2 * 1024 * 1024;
    if std::env::var(CHILD).as_deref() == Ok("1") {
        eprintln!(
            "type environment requested_stack_bytes={STACK}, target={}-{}",
            std::env::consts::ARCH,
            std::env::consts::OS
        );
        std::thread::Builder::new()
            .name("canonical-type-resources".into())
            .stack_size(STACK)
            .spawn(work)
            .unwrap()
            .join()
            .unwrap();
        return;
    }
    let test = format!(
        "{}::canonical_type_boundary_on_two_mib_worker",
        module_path!().split_once("::").unwrap().1
    );
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", &test, "--nocapture"])
        .env(CHILD, "1")
        .env_remove("RUST_MIN_STACK")
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    eprintln!("{stdout}{stderr}");
    assert!(
        output.status.success(),
        "isolated type test failed: {}",
        output.status
    );
    assert!(stdout.contains("1 passed"), "exact child test must execute");
}

/// Small IR; the large dimension exists only in its fixed value layout.
fn owned_contract(length: usize) -> crate::ir::Contract {
    use crate::ir::*;
    let owned = Type::Struct {
        name: "OwnedBoundary".into(),
        fields: vec![StructField {
            name: "items".into(),
            ty: Type::Vector {
                element: Box::new(Type::Unit),
                length,
            },
        }],
    };
    Contract {
        schema_version: SCHEMA_VERSION,
        type_aliases: vec![],
        ledger_fields: vec![LedgerField {
            source: None,
            id: "state".into(),
            index: 0,
            path: vec![0],
            declaration: LedgerFieldKind::Cell { ty: Type::Field },
        }],
        constructor: None,
        witnesses: vec![],
        circuits: vec![PureCircuit {
            source: None,
            name: "owned".into(),
            internal: true,
            parameters: vec![],
            result: owned.clone(),
            body: Expr::Default { ty: owned },
        }],
        stateful_circuits: vec![StatefulCircuit {
            source: None,
            name: "bounded_owned".into(),
            internal: false,
            parameters: vec![Parameter {
                name: "value".into(),
                ty: Type::Field,
            }],
            result: Type::Unit,
            actions: vec![
                StateAction::Expression {
                    value: Expr::Call {
                        name: "owned".into(),
                        arguments: vec![],
                    },
                },
                StateAction::CellWrite {
                    field: "state".into(),
                    index: 0,
                    value: Expr::Parameter {
                        name: "value".into(),
                    },
                },
            ],
            return_value: StateReturn::Expression { value: Expr::Unit },
        }],
    }
}

fn owned_pipeline_boundary() {
    // Unit leaf=1, vector container=1, named Struct container=1.
    let limit = crate::resource_limits::recording_fixed_layout_limit();
    let maximum = limit
        .checked_sub(2)
        .expect("policy must fit two containers");
    for (length, accepted) in [(maximum, true), (maximum + 1, false)] {
        let contract = owned_contract(length);
        let root = &contract.stateful_circuits[0];
        let ledger = contract
            .ledger_fields
            .iter()
            .map(|f| (f.id.as_str(), f))
            .collect();
        let pure = contract
            .circuits
            .iter()
            .map(|c| (c.name.as_str(), c))
            .collect();
        let checked =
            super::super::check_with_calls(root, &ledger, &HashMap::new(), &pure, &HashMap::new());
        if accepted {
            checked.unwrap();
        } else {
            match checked {
                Err(super::super::PlanError::Unsupported(gap)) => {
                    assert!(gap.detail.contains("fixed aggregate layout"), "{gap:?}")
                }
                other => panic!("over-limit source must refuse before validation: {other:?}"),
            }
        }
        let (rendered, trace) = crate::recorded::admission_probe::trace::capture_json(|| {
            crate::render_with_capabilities(&contract)
        });
        let rendered = rendered.unwrap();
        let roots = trace
            .as_array()
            .unwrap()
            .iter()
            .filter(|row| {
                row["phase"] == "public_emission"
                    && row["helper"] == false
                    && row["circuit"] == "bounded_owned"
            })
            .collect::<Vec<_>>();
        assert_eq!(roots.len(), 1);
        if accepted {
            assert_eq!(roots[0]["body_owner"], "algebra.unit-actions");
            assert_eq!(roots[0]["outcome"]["status"], "supported");
        } else {
            assert_ne!(roots[0]["body_owner"], "algebra.unit-actions");
            eprintln!("over-limit public admission: {}", roots[0]);
        }
        let capabilities = rendered
            .capabilities
            .circuits
            .iter()
            .filter(|c| c.name == "bounded_owned")
            .collect::<Vec<_>>();
        assert_eq!(capabilities.len(), 1);
        assert_eq!(capabilities[0].recorded, accepted);
        syn::parse_file(&rendered.source).unwrap();
    }
    eprintln!(
        "whole public rendering accepted owned fixed slots={limit}, vector elements={maximum}; next refused by algebra; no generated runtime allocation measured"
    );
}
