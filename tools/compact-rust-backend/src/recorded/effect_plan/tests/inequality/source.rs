// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Genuine frontend IR must use checked composition in actual public emission.
use super::*;
use crate::recorded::admission_probe::trace;
#[test]
fn genuine_source_inequalities_have_checked_public_composite_owners() {
    let source: Contract = serde_json::from_str(include_str!(
        "../../../../../tests/effect-plan-inequality-schema20-ir.json"
    ))
    .unwrap();
    let ledger = source
        .ledger_fields
        .iter()
        .map(|f| (f.id.as_str(), f))
        .collect();
    let witnesses = source
        .witnesses
        .iter()
        .map(|w| (w.name.as_str(), w))
        .collect();
    let pure = source
        .circuits
        .iter()
        .map(|c| (c.name.as_str(), c))
        .collect();
    let circuits: HashMap<_, _> = source
        .stateful_circuits
        .iter()
        .map(|c| (c.name.as_str(), c))
        .collect();
    for name in ["compare", "owned", "witnessed"] {
        let plan = check_with_calls(circuits[name], &ledger, &witnesses, &pure, &circuits).unwrap();
        render(&plan).unwrap();
    }
    let (output, rows) = trace::capture_json(|| crate::render_with_capabilities(&source));
    let output = output.unwrap();
    assert_eq!(output.capabilities.circuits.len(), 3);
    for name in ["compare", "owned", "witnessed"] {
        let cap = output
            .capabilities
            .circuits
            .iter()
            .find(|c| c.name == name)
            .unwrap();
        assert!(cap.recorded && cap.observed_call, "{cap:?}");
        let actual = rows
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| {
                r["circuit"] == name && r["phase"] == "public_emission" && r["helper"] == false
            })
            .collect::<Vec<_>>();
        assert_eq!(actual.len(), 1);
        assert_eq!(actual[0]["body_owner"], "algebra.ordinary");
        assert_eq!(actual[0]["outcome"]["status"], "supported");
    }
}
