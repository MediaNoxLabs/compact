// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
#[path = "../build_support/legacy_abi.rs"]
mod legacy_abi;

#[test]
fn whole_frozen_reference_projects_only_two_guards_and_reverses_exactly() {
    let original = include_str!("../legacy_reference.rs");
    let projected = legacy_abi::project(original).unwrap();
    assert_eq!(midnight_compact_runtime::RUST_RUNTIME_ABI, 52);
    assert_eq!(
        projected.replace("RUST_RUNTIME_ABI == 52", "RUST_RUNTIME_ABI == 51"),
        original
    );
    for changed in [
        original.to_owned() + "\n// changed body",
        original.replacen("RUST_RUNTIME_ABI == 51", "RUST_RUNTIME_ABI == 52", 1),
        original.replace("RUST_RUNTIME_ABI == 51", "RUST_RUNTIME_ABI == 50"),
    ] {
        assert!(legacy_abi::project(&changed).is_err());
    }
}
