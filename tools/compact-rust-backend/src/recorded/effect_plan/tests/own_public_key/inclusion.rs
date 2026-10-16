// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Retained complete old roots are prerequisites, never a whole-family proof.
use super::*;

#[test]
fn whole_old_payout_roots_compose_after_the_key_effect() {
    for (label, json, names) in [
        (
            "micro-dao",
            include_str!("../../../../../tests/micro-dao-source-schema20-ir.json"),
            &["cash_out"][..],
        ),
        (
            "coracle",
            include_str!("../../../../../tests/coracle-source-schema20-ir.json"),
            &["concede", "withdraw"][..],
        ),
    ] {
        let source: Contract = serde_json::from_str(json).unwrap();
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
        for &name in names {
            let root = circuits[name];
            let old = if name == "cash_out" {
                crate::recorded::typed_plan::lower_reset_payout(
                    root, &ledger, &witnesses, &pure, &circuits,
                )
            } else {
                crate::recorded::typed_plan::lower_shielded_payout(
                    root, &ledger, &witnesses, &pure, &circuits,
                )
            };
            assert!(
                old.is_some(),
                "{label}/{name}: complete old admission first"
            );
            let checked = check_with_calls(root, &ledger, &witnesses, &pure, &circuits)
                .unwrap_or_else(|e| panic!("{label}/{name}: next candidate gap {e:?}"));
            render(&checked).unwrap();
        }
        let output = owner(&source, names);
        if let Some(base) = std::env::var_os("COMPACT_OWN_PUBLIC_KEY_PROBE_OUTPUT") {
            let base = std::path::PathBuf::from(base);
            std::fs::create_dir_all(&base).unwrap();
            let dir = base.join(label);
            std::fs::create_dir(&dir).expect("use fresh evidence directory");
            std::fs::write(
                dir.join("compact-rust-ir.json"),
                serde_json::to_vec_pretty(&source).unwrap(),
            )
            .unwrap();
            std::fs::write(dir.join("lib.rs"), output).unwrap();
        }
    }
}
fn owner(source: &Contract, names: &[&str]) -> String {
    let (output, rows) = crate::recorded::admission_probe::trace::capture_json(|| {
        crate::render_with_capabilities(source)
    });
    let output = output.unwrap();
    for &name in names {
        let cap = output
            .capabilities
            .circuits
            .iter()
            .find(|c| c.name == name)
            .unwrap();
        assert!(cap.recorded && cap.observed_call, "{name}");
        let actual: Vec<_> = rows
            .as_array()
            .unwrap()
            .iter()
            .filter(|row| {
                row["circuit"] == name
                    && row["phase"] == "public_emission"
                    && row["helper"] == false
            })
            .collect();
        assert_eq!(actual.len(), 1);
        assert_eq!(actual[0]["body_owner"], "algebra.ordinary", "{name}");
    }
    output.source
}
#[test]
fn genuine_repeated_lazy_key_source_uses_existing_composite_domain() {
    let source: Contract = serde_json::from_str(include_str!(
        "../../../../../tests/effect-plan-own-public-key-schema20-ir.json"
    ))
    .unwrap();
    owner(&source, &["run"]);
}
