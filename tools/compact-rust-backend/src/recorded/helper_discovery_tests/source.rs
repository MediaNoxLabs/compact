// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;

#[test]
fn retained_source_struct_and_token_callees_use_live_allocated_helpers() {
    for (raw, target) in [
        (
            include_str!("../../../tests/stateful-struct-schema20-ir.json"),
            "snapshot",
        ),
        (
            include_str!("../../../tests/micro-dao-source-schema20-ir.json"),
            "dao_voting_token",
        ),
    ] {
        let c: Contract = serde_json::from_str(raw).unwrap();
        let output = crate::render_with_capabilities(&c).unwrap();
        assert!(
            output
                .capabilities
                .circuits
                .iter()
                .find(|v| v.name == target)
                .unwrap()
                .recorded
        );
        let cx = Context::new(&c);
        assert!(!cx.plan(&c, true).unwrap().0.contains(target));
        let (names, _) = cx.plan(&c, false).unwrap();
        assert!(names.contains(target));
        let body = helpers::helper_ident(target, &cx.circuits).unwrap();
        let functions = recorded_functions(&c);
        assert!(functions.iter().any(|f| f.sig.ident == body));
        let public = functions.iter().find(|f| f.sig.ident == target).unwrap();
        assert!(
            public
                .to_token_stream()
                .to_string()
                .contains(&body.to_string())
        );
    }
}

/// Export whole-source old/current planner premises over explicitly supplied
/// retained IR. The old planner cannot consult the new discovery graph.
#[test]
#[ignore = "explicit helper differential evidence export"]
fn export_helper_planner_differential() {
    let root = std::path::PathBuf::from(
        std::env::var_os("COMPACT_HELPER_DIFFERENTIAL_ROOT").expect("set input root"),
    );
    let output = std::path::PathBuf::from(
        std::env::var_os("COMPACT_HELPER_DIFFERENTIAL_OUTPUT").expect("set evidence output"),
    );
    assert!(!output.exists(), "preserve previous evidence");
    let mut paths: Vec<_> = std::fs::read_dir(root)
        .unwrap()
        .map(|v| v.unwrap().path())
        .filter(|p| p.extension().is_some_and(|v| v == "json"))
        .collect();
    paths.sort();
    let mut rows = Vec::new();
    for path in paths {
        let raw = std::fs::read(&path).unwrap();
        let c: Contract = serde_json::from_slice(&raw).unwrap();
        let native = crate::render_with_capabilities(&c).unwrap();
        let cx = Context::new(&c);
        let (old_names, old_items) = cx.plan(&c, true).unwrap();
        let (names, items) = cx.plan(&c, false).unwrap();
        assert!(
            old_names.is_subset(&names),
            "old helper support lost: {}",
            path.display()
        );
        for item in &old_items {
            let old = normalized(item);
            assert!(
                items.iter().any(|v| normalized(v) == old),
                "old helper item changed: {}",
                path.display()
            );
        }
        let mut old_names: Vec<_> = old_names.into_iter().collect();
        old_names.sort();
        let mut names: Vec<_> = names.into_iter().collect();
        names.sort();
        rows.push(serde_json::json!({"input":path.file_name().unwrap().to_string_lossy(), "old_names":old_names, "current_names":names, "old_items":old_items.len(), "current_items":items.len(), "old_items_identical":true, "native_full_render":"accepted", "capabilities":native.capabilities}));
    }
    std::fs::write(output, serde_json::to_vec_pretty(&serde_json::json!({"scope":"finite full-native old/helper-selected current differential; not language or runtime/proof coverage", "rows":rows})).unwrap()).unwrap();
}
