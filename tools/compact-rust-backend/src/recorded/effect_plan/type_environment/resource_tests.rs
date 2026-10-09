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
