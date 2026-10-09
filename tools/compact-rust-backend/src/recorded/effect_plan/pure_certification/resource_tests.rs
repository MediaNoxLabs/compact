// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Requested-stack qualification for this bridge, not runtime fold-work bounds.
use super::*;
use crate::ir::{Parameter, Type};

fn source(name: &str, body: Expr) -> PureCircuit {
    PureCircuit {
        source: None,
        name: name.into(),
        internal: true,
        parameters: vec![Parameter {
            name: "selected".into(),
            ty: Type::Boolean,
        }],
        result: Type::Field,
        body,
    }
}
fn leaf() -> Expr {
    Expr::FieldLiteral { value: "1".into() }
}
fn selected() -> Expr {
    Expr::Parameter {
        name: "selected".into(),
    }
}
fn nested_body(depth: usize) -> PureCircuit {
    let mut body = leaf();
    for _ in 0..depth {
        body = Expr::If {
            condition: Box::new(selected()),
            then: Box::new(body),
            otherwise: Box::new(Expr::FieldLiteral { value: "2".into() }),
        };
    }
    source("nested", body)
}
fn certify_all(sources: &[PureCircuit], name: &str) -> Result<Certificates, RecordingGap> {
    let pure = sources.iter().map(|c| (c.name.as_str(), c)).collect();
    let stateful = HashMap::new();
    let mut certifier = Certifier::new(&pure, &stateful);
    certifier.certify(name, "resource.root")?;
    let result = certifier.into_certificates();
    result.verify_bindings(&pure)?;
    Ok(result)
}
fn work() {
    let mut maximum = 0;
    for depth in 1..=128 {
        match PureBodyBudget::default().check(&nested_body(depth)) {
            Ok(_) => maximum = depth,
            Err(crate::RenderError::ResourceLimit { .. }) => break,
            Err(error) => panic!("unexpected resource refusal: {error}"),
        }
    }
    assert!(maximum > 0 && maximum < 128);
    certify_all(&[nested_body(maximum)], "nested").expect("admitted native-pure body boundary");
    let refusal = certify_all(&[nested_body(maximum + 1)], "nested").unwrap_err();
    assert!(refusal.detail.contains("compiler resource"), "{refusal:?}");
    eprintln!("pure body admitted nested If depth={maximum}; next refused before native typing");

    let (limit, _) = crate::resource_limits::recording_call_limits();
    let make = |count| {
        (0..count)
            .map(|i| {
                source(
                    &format!("helper{i}"),
                    if i + 1 == count {
                        leaf()
                    } else {
                        Expr::Call {
                            name: format!("helper{}", i + 1),
                            arguments: vec![selected()],
                        }
                    },
                )
            })
            .collect::<Vec<_>>()
    };
    let certificates = certify_all(&make(limit), "helper0").expect("admitted pure graph boundary");
    assert_eq!(certificates.get("helper0").unwrap().call_depth(), limit);
    let refusal = certify_all(&make(limit + 1), "helper0").unwrap_err();
    assert!(refusal.detail.contains("depth"), "{refusal:?}");
    eprintln!(
        "pure graph admitted declarations={limit}; next refused; stateful+pure limits tested separately"
    );
}

#[test]
fn pure_bridge_boundaries_on_two_mib_worker() {
    const CHILD: &str = "COMPACT_PURE_CERTIFICATE_RESOURCE_CHILD";
    const STACK_BYTES: usize = 2 * 1024 * 1024;
    if std::env::var(CHILD).as_deref() == Ok("1") {
        eprintln!(
            "pure bridge requested_stack_bytes={STACK_BYTES}, target={}-{}",
            std::env::consts::ARCH,
            std::env::consts::OS
        );
        std::thread::Builder::new()
            .name("pure-certificate-resources".into())
            .stack_size(STACK_BYTES)
            .spawn(work)
            .unwrap()
            .join()
            .unwrap();
        return;
    }
    let test = format!(
        "{}::pure_bridge_boundaries_on_two_mib_worker",
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
        "isolated pure resource test failed: {}",
        output.status
    );
    assert!(stdout.contains("1 passed"), "exact child test must execute");
}
