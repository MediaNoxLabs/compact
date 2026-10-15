// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Complete old-funded-mint inclusion after typed inequality; rendering is not compilation.
use super::*;
use crate::recorded::admission_probe::trace;
use serde_json::json;

fn source() -> Contract {
    serde_json::from_str(include_str!(
        "../../../../tests/micro-dao-source-schema20-ir.json"
    ))
    .unwrap()
}
fn root_mut(c: &mut Contract) -> &mut StatefulCircuit {
    c.stateful_circuits
        .iter_mut()
        .find(|c| c.name == "buy_in")
        .unwrap()
}
fn normalized_inequality(c: &mut Contract) {
    let StateAction::Assert { condition, .. } = &mut root_mut(c).actions[0] else {
        panic!("original first assertion")
    };
    let Expr::NotEqual { left, right } = condition.clone() else {
        panic!("original inequality")
    };
    *condition = Expr::Equal {
        left: Box::new(Expr::Equal { left, right }),
        right: Box::new(Expr::Boolean { value: false }),
    };
}
fn probe(c: Contract, label: &str) {
    let ledger = c.ledger_fields.iter().map(|v| (v.id.as_str(), v)).collect();
    let witnesses = c.witnesses.iter().map(|v| (v.name.as_str(), v)).collect();
    let pure = c.circuits.iter().map(|v| (v.name.as_str(), v)).collect();
    let circuits: HashMap<_, _> = c
        .stateful_circuits
        .iter()
        .map(|v| (v.name.as_str(), v))
        .collect();
    let root = circuits["buy_in"];
    assert!(
        crate::recorded::typed_plan::lower_funded_mint(root, &ledger, &witnesses, &pure, &circuits)
            .is_some(),
        "{label}: complete old funded-mint acceptance first"
    );
    let checked = match check_with_calls(root, &ledger, &witnesses, &pure, &circuits) {
        Ok(plan) => {
            render(&plan).unwrap();
            json!({"status":"supported"})
        }
        Err(PlanError::Unsupported(gap)) => {
            json!({"status":"unsupported", "path":gap.path,"detail":gap.detail,"ir_node":gap.ir_node})
        }
        Err(PlanError::Invalid(gap)) => panic!("{label}: invariant failure {gap:?}"),
    };
    let (output, rows) = trace::capture_json(|| crate::render_with_capabilities(&c));
    let output = output.unwrap_or_else(|e| panic!("{label}: full native render: {e:?}"));
    let metadata = json!({"label":label,"root":"buy_in","internal":root.internal,"whole_old_funded_mint_admitted":true,"checked":checked,"full_render":true,"capabilities":output.capabilities,"invocations":rows,"claim":"Exploratory full IR/render evidence only; native compilation, runtime and proof qualification remain separate."});
    eprintln!(
        "FUNDED_MINT_PROBE {}",
        serde_json::to_string(&metadata).unwrap()
    );
    if let Some(base) = std::env::var_os("COMPACT_FUNDED_MINT_PROBE_OUTPUT") {
        let dir = std::path::PathBuf::from(base).join(label);
        std::fs::create_dir_all(dir.parent().unwrap()).unwrap();
        std::fs::create_dir(&dir).expect("fresh probe output required");
        std::fs::write(
            dir.join("compact-rust-ir.json"),
            serde_json::to_vec_pretty(&c).unwrap(),
        )
        .unwrap();
        std::fs::write(dir.join("lib.rs"), output.source).unwrap();
        std::fs::write(
            dir.join("case.json"),
            serde_json::to_vec_pretty(&metadata).unwrap(),
        )
        .unwrap();
    }
    assert_eq!(
        checked["status"], "supported",
        "{label}: complete old root must compose"
    );
    if !root.internal {
        let emitted = rows
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| {
                r["circuit"] == "buy_in" && r["phase"] == "public_emission" && r["helper"] == false
            })
            .collect::<Vec<_>>();
        assert_eq!(emitted.len(), 1);
        assert_eq!(emitted[0]["body_owner"], "algebra.ordinary");
        assert_eq!(emitted[0]["outcome"]["status"], "supported");
    }
}
fn run(label: &'static str, normalize: bool, internal: bool) {
    std::thread::Builder::new()
        .stack_size(8 * 1024 * 1024)
        .spawn(move || {
            let mut c = source();
            if normalize {
                normalized_inequality(&mut c);
            }
            root_mut(&mut c).internal = internal;
            probe(c, label);
        })
        .unwrap()
        .join()
        .unwrap();
}
#[test]
fn original_full_buy_in_inequality_now_composes_without_legacy_fallback() {
    run("original-public", false, false);
}
#[test]
fn equivalent_boolean_negation_remains_a_full_root_control() {
    run("normalized-public", true, false);
}
#[test]
fn old_internal_root_uses_same_typed_primitives() {
    run("normalized-internal", true, true);
}
