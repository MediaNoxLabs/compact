// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

fn main() {
    let input = "../../tools/compact-rust-backend/tests/call-arg-declared-type-schema20-ir.json";
    println!("cargo:rerun-if-changed={input}");
    let contract = serde_json::from_slice(&std::fs::read(input).unwrap()).unwrap();
    let rendered = compact_rust_backend::render_with_capabilities(&contract).unwrap();
    let names = rendered
        .capabilities
        .circuits
        .iter()
        .map(|circuit| {
            assert!(
                circuit.recorded && circuit.observed_call,
                "{} capability changed: {:?}",
                circuit.name,
                circuit
            );
            circuit.name.as_str()
        })
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        names,
        std::collections::BTreeSet::from([
            "commitSmall",
            "commitU128",
            "commitFieldOnly",
            "pureBodyVec",
            "pureBodyFieldOnly",
            "bridgeTupleIntoVec",
            "bridgeVecIntoTuple",
            "witnessConst",
            "witnessBare",
            "pureFromImpure",
            "impureConst",
            "impureBare",
            "impureInIfArm",
            "inlinedAssert",
            "hashPersistentVec",
            "hashTransientVec"
        ])
    );
    let output = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    std::fs::write(output.join("contract.rs"), rendered.source).unwrap();
}
