// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Explicit whole-root old admission before candidate checks, using authenticated external corpus.
use super::*;
use crate::recorded::{admission_probe::trace, profile_attempt::ProfileAttempt};
use serde_json::json;

#[test]
#[ignore = "external authenticated corpus and fresh output required"]
fn export_complete_retained_map_family() {
    let input = std::path::PathBuf::from(std::env::var_os("COMPACT_MAP_INPUTS").expect("inputs"));
    let output = std::path::PathBuf::from(std::env::var_os("COMPACT_MAP_OUTPUT").expect("output"));
    let baseline: serde_json::Value = serde_json::from_slice(
        &std::fs::read(std::env::var_os("COMPACT_MAP_ROOTS").expect("root list")).unwrap(),
    )
    .unwrap();
    let mut groups = std::collections::BTreeMap::<String, Vec<String>>::new();
    for row in baseline["roots"].as_array().unwrap() {
        groups
            .entry(row["source"].as_str().unwrap().into())
            .or_default()
            .push(row["circuit"].as_str().unwrap().into());
    }
    assert_eq!(groups.len(), 14);
    assert_eq!(groups.values().map(Vec::len).sum::<usize>(), 31);
    std::fs::create_dir(&output).expect("fresh export directory");
    for (file, roots) in groups {
        let source = std::fs::read_to_string(input.join(&file)).unwrap();
        // Preserve the established explicit envelope for complete historical DID roots.
        let output = output.clone();
        std::thread::Builder::new().stack_size(8 * 1024 * 1024).spawn(move || {
            let contract: Contract = serde_json::from_str(&source).unwrap();
            let ledger = contract.ledger_fields.iter().map(|v| (v.id.as_str(), v)).collect();
            let witnesses = contract.witnesses.iter().map(|v| (v.name.as_str(), v)).collect();
            let pure = contract.circuits.iter().map(|v| (v.name.as_str(), v)).collect();
            let stateful: HashMap<_, _> = contract.stateful_circuits.iter().map(|v| (v.name.as_str(), v)).collect();
            let mut checked = Vec::new();
            for name in &roots {
                let root = stateful[name.as_str()];
                assert!(matches!(crate::recorded::typed_plan::lower_unit_composition_checked(
                    root, &ledger, &witnesses, &pure, &stateful), ProfileAttempt::Admitted(_)),
                    "{file}/{name}: complete original old admission");
                let outcome = match check_with_calls(root, &ledger, &witnesses, &pure, &stateful) {
                    Ok(plan) => { render(&plan).unwrap(); json!({"status":"supported"}) },
                    Err(PlanError::Unsupported(gap)) => json!({"status":"unsupported","gap":{"ir_node":gap.ir_node,"path":gap.path,"detail":gap.detail}}),
                    Err(PlanError::Invalid(gap)) => panic!("{file}/{name}: invalid candidate {gap:?}"),
                };
                checked.push((name, root.internal, outcome));
            }
            let (result, invocations) = trace::capture_json(|| crate::render_with_capabilities(&contract));
            let result = result.unwrap();
            for (name, internal, outcome) in checked {
                let rows: Vec<_> = invocations.as_array().unwrap().iter().filter(|v|
                    v["circuit"]==name.as_str() && v["phase"]=="public_emission" && v["helper"]==false).collect();
                assert_eq!(rows.len(),1);assert_eq!(rows[0]["outcome"]["status"],"supported");
                let label=format!("{}-{name}",file.trim_end_matches(".json"));
                let directory=output.join(&label);std::fs::create_dir(&directory).unwrap();
                let metadata=json!({"label":label,"root":name,"internal":internal,"source":file,
                    "whole_old_required":true,"whole_old_unit_composition":"admitted","checked":outcome,
                    "public_emission":rows[0],"capabilities":result.capabilities,
                    "claim":"Complete old admission precedes checked candidate; raw full consumer compiled separately."});
                std::fs::write(directory.join("compact-rust-ir.json"),&source).unwrap();
                std::fs::write(directory.join("lib.rs"),&result.source).unwrap();
                std::fs::write(directory.join("case.json"),serde_json::to_vec_pretty(&metadata).unwrap()).unwrap();
            }
        }).unwrap().join().unwrap();
    }
}
