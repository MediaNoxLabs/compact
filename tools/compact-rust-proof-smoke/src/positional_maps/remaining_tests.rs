// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;

#[test]
fn changed_complete_source_is_refused() {
    counter_scenario::validate_ir(RETAINED_IR.as_bytes(), RETAINED_IR).unwrap();
    let mut changed: serde_json::Value = serde_json::from_str(RETAINED_IR).unwrap();
    changed["independent_map_control"] = true.into();
    assert!(
        counter_scenario::validate_ir(&serde_json::to_vec(&changed).unwrap(), RETAINED_IR).is_err()
    );
}

#[test]
fn missing_source_is_refused_before_seed_or_keys() {
    let root = std::env::temp_dir().join(format!(
        "compact-map-remaining-absent-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
    ));
    assert!(!root.exists());
    assert_eq!(
        run(&root)
            .unwrap_err()
            .downcast_ref::<io::Error>()
            .unwrap()
            .kind(),
        io::ErrorKind::NotFound
    );
    assert!(!root.exists());
}
