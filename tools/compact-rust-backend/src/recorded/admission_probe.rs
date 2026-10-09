// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Test-only admission diagnostics. Candidate lowering never uses a fallback.
//! This probes compilation, never executes witnesses, and does not promise
//! replay/preparation/proof support. Effectful route observations come from
//! actual dispatch; ordinary roots are explicitly outside route instrumentation.

use super::*;
use crate::ir::Contract;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{cell::Cell, path::Path};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Route {
    AlgebraExperimental,
    AlgebraIntersection,
    LegacyEffectful,
    LegacyStartFunding,
    Unsupported,
}

thread_local! {
    static LAST_ROUTE: Cell<Option<Route>> = const { Cell::new(None) };
}

pub(super) fn observe(route: Route) {
    LAST_ROUTE.set(Some(route));
}

#[derive(Debug, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
enum Candidate {
    Supported,
    Unsupported { gap: RecordingGap },
    Invalid { gap: RecordingGap },
}

#[derive(Debug, Serialize)]
struct Row {
    circuit: String,
    internal: bool,
    candidate: Candidate,
    effectful_route: Option<Route>,
    effectful_route_error: Option<String>,
}

fn inspect(contract: &Contract) -> Vec<Row> {
    let ledger = contract
        .ledger_fields
        .iter()
        .map(|f| (f.id.as_str(), f))
        .collect();
    let witnesses = contract
        .witnesses
        .iter()
        .map(|w| (w.name.as_str(), w))
        .collect();
    let pure = contract
        .circuits
        .iter()
        .map(|c| (c.name.as_str(), c))
        .collect();
    let circuits = contract
        .stateful_circuits
        .iter()
        .map(|c| (c.name.as_str(), c))
        .collect();
    contract
        .stateful_circuits
        .iter()
        .map(|circuit| {
            let candidate = match effect_plan::lower(circuit, &ledger, &witnesses) {
                Ok(_) => Candidate::Supported,
                Err(effect_plan::PlanError::Unsupported(gap)) => Candidate::Unsupported { gap },
                Err(effect_plan::PlanError::Invalid(gap)) => Candidate::Invalid { gap },
            };
            LAST_ROUTE.set(None);
            let effectful_route_error =
                if matches!(circuit.return_value, StateReturn::Effectful { .. }) {
                    lower_effectful_return(circuit, &ledger, &witnesses, &pure, &circuits)
                        .err()
                        .map(|error| error.to_string())
                } else {
                    None
                };
            Row {
                circuit: circuit.name.clone(),
                internal: circuit.internal,
                candidate,
                effectful_route: LAST_ROUTE.take(),
                effectful_route_error,
            }
        })
        .collect()
}

fn fixture() -> Contract {
    serde_json::from_str(include_str!(
        "../../tests/effectful-return-schema20-ir.json"
    ))
    .unwrap()
}

#[test]
fn actual_route_reports_small_and_wide_checked_intersections() {
    let mut contract = fixture();
    let rows = inspect(&contract);
    assert!(matches!(rows[0].candidate, Candidate::Supported));
    assert_eq!(
        rows[0].effectful_route,
        Some(if cfg!(feature = "experimental-recording-plan") {
            Route::AlgebraExperimental
        } else {
            Route::AlgebraIntersection
        })
    );

    let StateReturn::Effectful { body } = &mut contract.stateful_circuits[0].return_value else {
        panic!("expected effectful fixture");
    };
    let max = "340282366920938463463374607431768211456";
    *body = crate::ir::ReturnPlan::Let {
        bindings: vec![LocalBinding {
            name: "wide_probe".into(),
            ty: Type::Unsigned { max: max.into() },
            value: Expr::UnsignedLiteral {
                value: "1".into(),
                max: max.into(),
            },
        }],
        result: Box::new(body.clone()),
    };
    let rows = inspect(&contract);
    assert!(matches!(rows[0].candidate, Candidate::Supported));
    assert_eq!(
        rows[0].effectful_route,
        Some(if cfg!(feature = "experimental-recording-plan") {
            Route::AlgebraExperimental
        } else {
            Route::AlgebraIntersection
        })
    );
}

#[test]
fn independently_supported_candidate_does_not_imply_default_admission() {
    let contract: Contract = serde_json::from_str(include_str!(
        "../../tests/effect-plan-prototype-schema20-ir.json"
    ))
    .unwrap();
    for row in inspect(&contract) {
        assert!(matches!(row.candidate, Candidate::Supported));
        assert_eq!(
            row.effectful_route,
            Some(if cfg!(feature = "experimental-recording-plan") {
                Route::AlgebraExperimental
            } else {
                Route::Unsupported
            })
        );
    }
}

#[test]
fn separate_legacy_family_is_reported_without_disguising_candidate_refusal() {
    let contract: Contract =
        serde_json::from_str(include_str!("../../tests/coracle-start-schema20-ir.json")).unwrap();
    let rows = inspect(&contract);
    let row = rows.iter().find(|row| row.circuit == "start").unwrap();
    assert!(matches!(row.candidate, Candidate::Unsupported { .. }));
    assert_eq!(row.effectful_route, Some(Route::LegacyStartFunding));
}

fn recording_invariant(error: &RenderError) -> bool {
    match error {
        RenderError::RecordingPlanInvariant { .. } => true,
        RenderError::Located { error, .. } => recording_invariant(error),
        _ => false,
    }
}

#[test]
fn public_compilation_cannot_hide_located_recording_invariants() {
    let direct = RenderError::RecordingPlanInvariant {
        path: "return_value".into(),
        detail: "probe".into(),
    };
    assert!(recording_invariant(&direct));
    let located = RenderError::Located {
        location: crate::ir::SourceLocation {
            file: "probe.compact".into(),
            line: 1,
            column: 1,
        },
        error: Box::new(direct),
    };
    assert!(recording_invariant(&located));
    assert!(!recording_invariant(&RenderError::SchemaVersion(0)));
}

fn json_files(root: &Path, files: &mut Vec<std::path::PathBuf>) {
    for entry in std::fs::read_dir(root).unwrap() {
        let entry = entry.unwrap();
        // Do not traverse symlinks; reports are restricted to the supplied tree.
        let kind = entry.file_type().unwrap();
        if kind.is_dir() {
            json_files(&entry.path(), files);
        } else if kind.is_file() && entry.path().extension().is_some_and(|ext| ext == "json") {
            files.push(entry.path());
        }
    }
}

/// Run explicitly with COMPACT_RECORDING_PROBE_OUTPUT and optional
/// COMPACT_RECORDING_PROBE_ROOT. Defaults to this crate's retained IR fixtures.
/// Reports rejected native inputs instead of bypassing their validation.
#[test]
#[ignore = "explicit evidence export; requires COMPACT_RECORDING_PROBE_OUTPUT"]
fn export_candidate_admission_report() {
    let output = std::env::var_os("COMPACT_RECORDING_PROBE_OUTPUT")
        .expect("set COMPACT_RECORDING_PROBE_OUTPUT");
    let root = std::env::var_os("COMPACT_RECORDING_PROBE_ROOT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("tests"));
    let mut files = Vec::new();
    json_files(&root, &mut files);
    files.sort();
    let mut records = Vec::new();
    let mut skipped = Vec::new();
    let mut invariant_failures = 0;
    let mut accepted_contracts = 0;
    let mut rejected_contracts = 0;
    let mut inspected_stateful_rows = 0;
    for path in files {
        let bytes = std::fs::read(&path).unwrap();
        let source = path
            .strip_prefix(&root)
            .unwrap()
            .to_string_lossy()
            .into_owned();
        let contract: Contract = match serde_json::from_slice(&bytes) {
            Ok(contract) => contract,
            Err(error) => {
                skipped.push(
                    serde_json::json!({ "source": source, "parse_error": error.to_string() }),
                );
                continue;
            }
        };
        let fingerprint = format!("{:x}", Sha256::digest(&bytes));
        match crate::render_with_capabilities(&contract) {
            Ok(rendered) => {
                accepted_contracts += 1;
                let rows = inspect(&contract);
                inspected_stateful_rows += rows.len();
                invariant_failures += rows
                    .iter()
                    .filter(|row| {
                        matches!(row.candidate, Candidate::Invalid { .. })
                            || row.effectful_route_error.is_some()
                    })
                    .count();
                records.push(serde_json::json!({
                    "source": source, "ir_sha256": fingerprint,
                    "full_compilation": "accepted", "capabilities": rendered.capabilities,
                    "candidate_rows": rows,
                }));
            }
            Err(error) => {
                rejected_contracts += 1;
                let invariant = recording_invariant(&error);
                invariant_failures += usize::from(invariant);
                records.push(serde_json::json!({
                    "source": source, "ir_sha256": fingerprint,
                    "compilation": "full_compilation_rejected",
                    "recording_invariant": invariant, "error": error.to_string(),
                }));
            }
        }
    }
    assert!(!records.is_empty(), "no retained contract IR was inspected");
    let report = serde_json::json!({
        "schema_version": 1,
        "experimental_feature": cfg!(feature = "experimental-recording-plan"),
        "scope": "finite saved-IR compilation census; not language coverage or execution/proof evidence",
        "routing_scope": "effectful lowering only; public capabilities also apply outer facade rules",
        "records": records, "skipped_inputs": skipped,
        "invariant_failures": invariant_failures,
        "accepted_contracts": accepted_contracts,
        "rejected_contracts": rejected_contracts,
        "inspected_stateful_rows": inspected_stateful_rows,
    });
    std::fs::write(output, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    assert!(
        inspected_stateful_rows > 0,
        "no stateful candidate was inspected"
    );
    assert_eq!(
        invariant_failures, 0,
        "candidate-only validation exposed compiler invariant failures; inspect report"
    );
}
