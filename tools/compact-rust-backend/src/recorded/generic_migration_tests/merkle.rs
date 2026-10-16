// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;

#[test]
fn complete_plain_and_historic_merkle_action_and_return_arms() {
    let digest = Type::Struct {
        name: "MerkleTreeDigest".into(),
        fields: vec![StructField {
            name: "field".into(),
            ty: Type::Field,
        }],
    };
    for historic in [false, true] {
        let prefix = if historic {
            "historic_merkle"
        } else {
            "merkle"
        };
        let declaration = if historic {
            LedgerFieldKind::HistoricMerkleTree {
                depth: 3,
                ty: Type::Boolean,
            }
        } else {
            LedgerFieldKind::MerkleTree {
                depth: 3,
                ty: Type::Boolean,
            }
        };
        for suffix in [
            "insert",
            "insert_index",
            "insert_index_default",
            "insert_hash",
            "insert_hash_index",
            "reset_to_default",
        ] {
            let mut a = json!({"kind":format!("{prefix}_{suffix}"),"field":"tree","index":0});
            if matches!(suffix, "insert" | "insert_index") {
                a["value"] = json!({"kind":"boolean","value":true});
            }
            if suffix.contains("hash") {
                a["hash"] = json!({"kind":"bytes_literal","bytes":vec![0u8;32]});
            }
            if suffix.contains("index") {
                a["position"] =
                    json!({"kind":"unsigned_literal","value":"0","max":u64::MAX.to_string()});
            }
            let mut c = unit(vec![serde_json::from_value(a).unwrap()]);
            slot(&mut c, "tree", declaration.clone());
            included(&c, "root", &format!("{prefix}_{suffix}"));
        }
        if historic {
            let mut c = unit(vec![StateAction::HistoricMerkleResetHistory {
                field: "tree".into(),
                index: 0,
            }]);
            slot(&mut c, "tree", declaration.clone());
            included(&c, "root", "historic reset history");
        }
        for suffix in ["is_full", "check_root"] {
            let mut r = json!({"kind":format!("{prefix}_{suffix}"),"field":"tree","index":0});
            if suffix == "check_root" {
                r["root"] = json!({"kind":"parameter","name":"digest"});
            }
            let mut c = root(Type::Boolean, serde_json::from_value(r).unwrap(), vec![]);
            if suffix == "check_root" {
                c.stateful_circuits[0].parameters = vec![parameter("digest", digest.clone())];
            }
            slot(&mut c, "tree", declaration.clone());
            included(&c, "root", &format!("{prefix}_{suffix}"));
        }
    }
    // Preserve the actual native path implementation/types from complete schema20,
    // but label the new zero-argument witness/root as synthetic derived IR.
    let source: Contract = serde_json::from_str(include_str!(
        "../../../tests/election-oracle-schema20-ir.json"
    ))
    .unwrap();
    let pure = source
        .circuits
        .iter()
        .find(|p| p.name == "merkleTreePathRoot")
        .unwrap();
    let mut c = root(
        Type::Boolean,
        StateReturn::Expression {
            value: Expr::Let {
                bindings: vec![binding(
                    "digest",
                    pure.result.clone(),
                    Expr::Call {
                        name: pure.name.clone(),
                        arguments: vec![Expr::WitnessCall {
                            name: "path".into(),
                            arguments: vec![],
                        }],
                    },
                )],
                body: Box::new(Expr::MerkleCheckRoot {
                    field: "tree".into(),
                    index: 0,
                    root: Box::new(var("digest")),
                }),
            },
        },
        vec![],
    );
    c.circuits = source.circuits.clone();
    c.witnesses.push(WitnessDeclaration {
        source: None,
        name: "path".into(),
        parameters: vec![],
        result: pure.parameters[0].ty.clone(),
    });
    let tree = source
        .ledger_fields
        .iter()
        .find(|f| f.id == "eligible_voters")
        .unwrap();
    slot(&mut c, "tree", tree.declaration.clone());
    let plan = included(&c, "root", "witnessed native path root return");
    let order = methods(&plan);
    assert!(
        order
            .iter()
            .position(|m| m == "try_witness_metered")
            .unwrap()
            < order.iter().position(|m| m == "record_check_root").unwrap()
    );
}
