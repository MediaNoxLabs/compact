// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Explicit local measurement, never a production support policy or ordinary gate.
//! One process/fixture; public DEFAULT gates precede certification and timing.
use super::super::Certifier;
use crate::{
    RenderError,
    ir::{Contract, Expr, Parameter, PureCircuit, StructField, Type},
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::HashMap, time::Instant};

const PREFIX: &str = "COMPACT_TYPING_RESOURCE_";
const STACK_BYTES: usize = 2 * 1024 * 1024;

struct Dimensions {
    fields: usize,
    repetitions: usize,
    name_bytes: usize,
}
fn dimension(suffix: &str, fallback: usize, maximum: usize) -> usize {
    let key = format!("{PREFIX}{suffix}");
    let value = std::env::var(&key).map_or(fallback, |text| {
        text.parse().expect("numeric probe dimension")
    });
    // Experiment containment only, allowing source inputs over production caps.
    assert!(
        value <= maximum,
        "{key} exceeds the local probe construction envelope"
    );
    value
}
fn nanos(start: Instant) -> u64 {
    u64::try_from(start.elapsed().as_nanos()).unwrap_or(u64::MAX)
}
fn refused(error: &RenderError) -> Value {
    let mut result =
        json!({"status":"refused", "error":error.to_string(), "debug":format!("{error:?}")});
    if let RenderError::ResourceLimit {
        resource,
        observed,
        limit,
    } = error
    {
        result["resource"] = json!(resource);
        result["observed"] = json!(observed);
        result["limit"] = json!(limit);
    }
    result
}
fn parameter() -> Expr {
    Expr::Parameter { name: "p".into() }
}
fn fixture(dim: &Dimensions) -> (Contract, usize, usize, usize) {
    let mut field_name_bytes = 0usize;
    let mut field_name_min = usize::MAX;
    let mut field_name_max = 0usize;
    let fields = (0..dim.fields)
        .map(|index| {
            let suffix = index.to_string();
            let padding = dim.name_bytes.saturating_sub(suffix.len() + 1);
            let name = format!("f{}{suffix}", "x".repeat(padding));
            field_name_bytes += name.len();
            field_name_min = field_name_min.min(name.len());
            field_name_max = field_name_max.max(name.len());
            StructField {
                name,
                ty: Type::Field,
            }
        })
        .collect();
    let steps = (0..dim.repetitions)
        .map(|_| Expr::Assert {
            condition: Box::new(Expr::Equal {
                left: Box::new(parameter()),
                right: Box::new(parameter()),
            }),
            message: String::new(),
        })
        .collect();
    let circuit = PureCircuit {
        source: None,
        internal: true,
        name: "probe".into(),
        parameters: vec![Parameter {
            name: "p".into(),
            ty: Type::Struct {
                name: "S".into(),
                fields,
            },
        }],
        result: Type::Unit,
        body: Expr::Sequence {
            steps,
            value: Box::new(Expr::Unit),
        },
    };
    (
        Contract {
            schema_version: crate::ir::SCHEMA_VERSION,
            type_aliases: vec![],
            ledger_fields: vec![],
            constructor: None,
            witnesses: vec![],
            circuits: vec![circuit],
            stateful_circuits: vec![],
        },
        field_name_bytes,
        if dim.fields == 0 { 0 } else { field_name_min },
        field_name_max,
    )
}

fn finish(mut report: Value, contract: Contract) -> Value {
    let start = Instant::now();
    drop(contract);
    report["fixture_teardown_ns"] = json!(nanos(start));
    report
}

fn measure() -> Value {
    let dim = Dimensions {
        fields: dimension("FIELDS", 32, 8192),
        repetitions: dimension("REPETITIONS", 32, 8192),
        name_bytes: dimension("NAME_BYTES", 0, 1024),
    };
    let start = Instant::now();
    let (contract, field_name_bytes, field_name_min, field_name_max) = fixture(&dim);
    let construction_ns = nanos(start);
    let start = Instant::now();
    let bytes = serde_json::to_vec(&contract).expect("serialize actual fixture");
    let byte_count = bytes.len();
    let hash = format!("{:x}", Sha256::digest(&bytes));
    if let Ok(path) = std::env::var(format!("{PREFIX}FIXTURE_OUTPUT")) {
        std::fs::write(path, &bytes).expect("write requested fixture artifact");
    }
    drop(bytes);
    let fixture_export_ns = nanos(start);
    let not_run = json!({"status":"not_run"});
    let mut report = json!({
        "measurement_schema":1, "kind":"direct_ir_repeated_struct_equality",
        "worker_requested_stack_bytes":STACK_BYTES,
        "target":format!("{}-{}",std::env::consts::ARCH,std::env::consts::OS),
        "debug_assertions":cfg!(debug_assertions),
        "fixture":{"fields":dim.fields,"repetitions":dim.repetitions,
            "requested_field_name_min_bytes":dim.name_bytes,"actual_field_name_bytes":field_name_bytes,
            "actual_field_name_min_bytes":field_name_min,"actual_field_name_max_bytes":field_name_max,
            "source_string_bytes_for_exact_shape":field_name_bytes + 7 + 2 * dim.repetitions,
            "json_bytes":byte_count,"json_sha256":hash},
        "construction_ns":construction_ns,"fixture_export_ns":fixture_export_ns,
        "public_preflight":not_run,"pure_source_budget":not_run,
        "certification":not_run,"repeated_checker":not_run,
        "qualification":"process RSS includes setup and earlier phases; Certifier already checks the body once"
    });
    let start = Instant::now();
    let preflight =
        crate::resource_limits::validate(&contract).and_then(|gate| gate.finish(&contract));
    report["public_preflight"] = match preflight {
        Ok(()) => json!({"status":"accepted"}),
        Err(ref error) => refused(error),
    };
    report["public_preflight"]["elapsed_ns"] = json!(nanos(start));
    if preflight.is_err() {
        report["outcome"] = json!("public_refused");
        return finish(report, contract);
    }
    let source = &contract.circuits[0];
    let start = Instant::now();
    let source_budget = crate::resource_limits::PureBodyBudget::default().check(source);
    report["pure_source_budget"] = match source_budget {
        Ok(nodes) => {
            // Independent assertion from the real exhaustive visitor, not a
            // copied traversal. This exact fixture has one extra Contract node.
            assert_eq!(nodes, 3 * dim.fields + 9 + 7 * dim.repetitions);
            json!({"status":"accepted","actual_pure_source_nodes":nodes,
                "contract_nodes_from_actual_pure":nodes + 1})
        }
        Err(ref error) => refused(error),
    };
    report["pure_source_budget"]["elapsed_ns"] = json!(nanos(start));
    assert!(
        source_budget.is_ok(),
        "public acceptance must satisfy this single pure source budget"
    );
    let pure = HashMap::from([("probe", source)]);
    let stateful = HashMap::new();
    let start = Instant::now();
    let certified = (|| {
        let mut certifier = Certifier::new(&pure, &stateful);
        certifier.certify("probe", "measurement.root")?;
        let certificates = certifier.into_certificates();
        certificates.verify_bindings(&pure)?;
        let entry = certificates
            .get("probe")
            .expect("completed root certificate");
        Ok::<_, crate::recorded::RecordingGap>((entry.call_depth(), entry.expanded_work()))
    })();
    report["certification"] = match certified {
        Ok((depth, work)) => json!({"status":"accepted","call_depth":depth,"expanded_work":work,
            "includes":"borrowed scans, obligations, checker, source snapshot and exact binding verification"}),
        Err(ref error) => json!({"status":"refused","path":error.path,"detail":error.detail,
            "debug":format!("{error:?}")}),
    };
    report["certification"]["elapsed_ns"] = json!(nanos(start));
    if certified.is_err() {
        report["outcome"] = json!("certification_refused");
        drop(pure);
        drop(stateful);
        return finish(report, contract);
    }
    let start = Instant::now();
    let checked = super::check(source, &pure);
    report["repeated_checker"] = match checked {
        Ok(()) => json!({"status":"accepted"}),
        Err(ref error) => refused(error),
    };
    report["repeated_checker"]["elapsed_ns"] = json!(nanos(start));
    report["outcome"] = json!(if checked.is_ok() {
        "accepted"
    } else {
        "checker_refused"
    });
    drop(pure);
    drop(stateful);
    finish(report, contract)
}

#[test]
#[ignore = "explicit single-fixture local time/RSS experiment; mandatory output path"]
fn measure_cumulative_typing_resources() {
    let output =
        std::env::var(format!("{PREFIX}OUTPUT")).expect("mandatory measurement JSON output path");
    let report = std::thread::Builder::new()
        .name("cumulative-typing-measurement".into())
        .stack_size(STACK_BYTES)
        .spawn(measure)
        .unwrap()
        .join()
        .unwrap();
    std::fs::write(output, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    println!(
        "COMPACT_TYPING_RESOURCE_MEASUREMENT {}",
        serde_json::to_string(&report).unwrap()
    );
    if let Ok(expected) = std::env::var(format!("{PREFIX}EXPECT")) {
        match expected.as_str() {
            "accept" => assert_eq!(report["outcome"], "accepted"),
            "refuse" => {
                assert_eq!(report["outcome"], "public_refused");
                assert_eq!(report["certification"]["status"], "not_run");
                assert_eq!(report["repeated_checker"]["status"], "not_run");
            }
            _ => panic!("expected accept or refuse for explicit measurement"),
        }
    }
}
