// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Bounded qualified Set subdomain examples, never whole membership retirement.
use super::*;
use crate::recorded::admission_probe::trace;
fn included(c: &Contract, name: &str, label: &str) {
    let ledger = c.ledger_fields.iter().map(|f| (f.id.as_str(), f)).collect();
    let witnesses = c.witnesses.iter().map(|w| (w.name.as_str(), w)).collect();
    let pure = c.circuits.iter().map(|r| (r.name.as_str(), r)).collect();
    let circuits: HashMap<_, _> = c
        .stateful_circuits
        .iter()
        .map(|r| (r.name.as_str(), r))
        .collect();
    let root = circuits[name];
    assert!(
        crate::recorded::typed_plan::lower(root, &ledger, &witnesses, &pure, &circuits).is_some(),
        "{label}: whole old membership oracle first"
    );
    let plan = check_with_calls(root, &ledger, &witnesses, &pure, &circuits)
        .unwrap_or_else(|e| panic!("{label}: {e:?}"));
    render(&plan).unwrap();
    let (output, rows) = trace::capture_json(|| crate::render_with_capabilities(c));
    let output = output.unwrap_or_else(|e| panic!("{label}: native render {e:?}"));
    let rows = rows
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["circuit"] == name && r["phase"] == "public_emission" && r["helper"] == false)
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), 1);
    assert_eq!(
        rows[0]["body_owner"], "algebra.ordinary",
        "{label}: whole old control now routes through independently checked composition"
    );
    let capability = output
        .capabilities
        .circuits
        .iter()
        .find(|r| r.name == name)
        .unwrap();
    assert!(capability.recorded && capability.observed_call);
    if let Some(path) = std::env::var_os("COMPACT_QUALIFIED_SET_PROBE_OUTPUT") {
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let root = std::path::PathBuf::from(path);
        std::fs::create_dir_all(&root).unwrap();
        let index = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let directory = root.join(format!("{index:03}"));
        std::fs::create_dir(&directory).expect("qualified Set export requires a fresh destination");
        std::fs::write(
            directory.join("compact-rust-ir.json"),
            serde_json::to_vec_pretty(c).unwrap(),
        )
        .unwrap();
        std::fs::write(directory.join("lib.rs"), output.source).unwrap();
        std::fs::write(directory.join("case.json"), serde_json::to_vec_pretty(&serde_json::json!({
            "label": label, "root": name, "whole_old_accepted": true, "checked_candidate": true,
            "actual_owner": "algebra.ordinary", "claim": "finite qualified Set subdomain only; no retirement or proof claim"
        })).unwrap()).unwrap();
    }
}
#[test]
fn retained_qualified_set_sources_are_whole_old_first_checked_candidates() {
    included(&source(), "insert_coin", "retained qualified Set insertion");
    let c: Contract = serde_json::from_str(include_str!(
        "../../../../../tests/adt-set-qualified-coin-info-schema20-ir.json"
    ))
    .unwrap();
    included(
        &c,
        "test_ShieldedCoinInfo",
        "retained zero-parameter ADT lifecycle",
    );
}
#[test]
fn finite_old_qualified_set_operand_and_action_compositions_remain_included() {
    for mode in 0..5 {
        let mut c = source();
        let r = &mut c.stateful_circuits[0];
        match mode {
            0 => r.name = "renamed_insert".into(),
            1 => {
                r.actions = vec![StateAction::Sequence {
                    actions: std::mem::take(&mut r.actions),
                }]
            }
            2 => {
                r.actions = vec![StateAction::Let {
                    bindings: vec![LocalBinding {
                        name: "coin".into(),
                        ty: shielded_coin_type(),
                        value: parameter("coin"),
                    }],
                    action: Box::new(insert(parameter("coin"), parameter("recipient"))),
                }]
            }
            3 => {
                r.actions = vec![StateAction::If {
                    condition: Expr::Boolean { value: true },
                    then: Box::new(insert(parameter("coin"), parameter("recipient"))),
                    otherwise: Box::new(StateAction::Sequence { actions: vec![] }),
                }]
            }
            4 => r.actions.push(r.actions[0].clone()),
            _ => unreachable!(),
        }
        let name = c.stateful_circuits[0].name.clone();
        included(&c, &name, &format!("finite qualified Set variant {mode}"));
    }
}
#[test]
fn full_frontend_nested_sources_use_first_eligible_algebra() {
    for (text, names) in [
        (
            include_str!("../../../../../tests/qualified-coin-set-chunked-schema20-ir.json"),
            &["insert_coin"][..],
        ),
        (
            include_str!("../../../../../tests/effect-plan-qualified-set-source-schema20-ir.json"),
            &["insert_once", "insert_selected"][..],
        ),
    ] {
        let c: Contract = serde_json::from_str(text).unwrap();
        let coins = c.ledger_fields.iter().find(|f| matches!(&f.declaration, LedgerFieldKind::Set { ty } if *ty == qualified_coin_type())).unwrap();
        assert_eq!(coins.path.len(), 2);
        let (output, rows) = trace::capture_json(|| crate::render_with_capabilities(&c));
        let output = output.unwrap();
        for name in names {
            let capability = output
                .capabilities
                .circuits
                .iter()
                .find(|r| r.name == *name)
                .unwrap();
            assert!(capability.recorded && capability.observed_call, "{name}");
            let rows = rows
                .as_array()
                .unwrap()
                .iter()
                .filter(|r| {
                    r["circuit"] == *name && r["phase"] == "public_emission" && r["helper"] == false
                })
                .collect::<Vec<_>>();
            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0]["body_owner"], "algebra.ordinary", "{name}");
        }
    }
}
