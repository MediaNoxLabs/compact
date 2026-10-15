// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;
use crate::ir::{LedgerField, SCHEMA_VERSION, TypeAlias, WitnessDeclaration};
use quote::ToTokens;

fn contract(historic: bool) -> Contract {
    Contract {
        schema_version: SCHEMA_VERSION,
        type_aliases: vec![],
        constructor: None,
        ledger_fields: vec![LedgerField {
            id: "tree".into(),
            index: 0,
            path: vec![0],
            source: None,
            declaration: if historic {
                LedgerFieldKind::HistoricMerkleTree {
                    depth: 3,
                    ty: Type::Field,
                }
            } else {
                LedgerFieldKind::MerkleTree {
                    depth: 3,
                    ty: Type::Field,
                }
            },
        }],
        witnesses: vec![WitnessDeclaration {
            source: None,
            name: "flag".into(),
            parameters: vec![],
            result: Type::Boolean,
        }],
        circuits: vec![],
        stateful_circuits: vec![],
    }
}
fn structs(source: &str) -> Vec<syn::ItemStruct> {
    let file = syn::parse_file(source).unwrap();
    file.items
        .into_iter()
        .filter_map(|item| match item {
            syn::Item::Mod(module) if module.ident == "types" => Some(module.content.unwrap().1),
            _ => None,
        })
        .flatten()
        .filter_map(|item| match item {
            syn::Item::Struct(item) => Some(item),
            _ => None,
        })
        .collect()
}
fn has_digest_derive(item: &syn::ItemStruct) -> bool {
    item.attrs.iter().any(|a| {
        a.to_token_stream()
            .to_string()
            .contains("CompactMerkleTreeDigest")
    })
}
fn alias(c: &mut Contract, ty: Type) {
    c.type_aliases.push(TypeAlias {
        source: None,
        name: "SourceValue".into(),
        ty,
    });
}

#[test]
fn plain_historic_and_nested_views_close_the_exact_digest_dependency() {
    for historic in [false, true] {
        for nested in [false, true] {
            let mut c = contract(historic);
            if nested {
                // A physical [1, 14] slot needs the complete 16-field layout;
                // an isolated field at that path is correctly rejected.
                let tree = c.ledger_fields.remove(0);
                c.ledger_fields = crate::expected_ledger_paths(16)
                    .into_iter()
                    .enumerate()
                    .map(|(position, path)| LedgerField {
                        id: if position == 15 {
                            tree.id.clone()
                        } else {
                            format!("padding_{position}")
                        },
                        index: path[0],
                        path,
                        source: None,
                        declaration: if position == 15 {
                            tree.declaration.clone()
                        } else {
                            LedgerFieldKind::Counter
                        },
                    })
                    .collect();
                assert_eq!(c.ledger_fields.last().unwrap().physical_path(), [1, 14]);
            }
            let output = crate::render(&c).unwrap();
            assert!(output.contains("crate::types::MerkleTreeDigest"));
            assert!(!output.contains("pub struct MerkleTreePath"));
            let declarations = structs(&output);
            assert_eq!(declarations.len(), 1);
            let digest = &declarations[0];
            assert_eq!(digest.ident, "MerkleTreeDigest");
            assert_eq!(digest.fields.len(), 1);
            assert_eq!(
                digest.fields.iter().next().unwrap().ident.as_ref().unwrap(),
                "field"
            );
            assert!(
                has_digest_derive(digest),
                "conversion is needed without any path struct"
            );
        }
    }
}

#[test]
fn multiple_views_and_explicit_canonical_alias_reuse_one_declaration() {
    let mut c = contract(false);
    let mut historic = contract(true).ledger_fields.remove(0);
    historic.id = "history".into();
    historic.index = 1;
    historic.path = vec![1];
    c.ledger_fields.push(historic);
    alias(&mut c, merkle_digest());
    let output = crate::render(&c).unwrap();
    let declarations = structs(&output);
    assert_eq!(declarations.len(), 1);
    assert!(has_digest_derive(&declarations[0]));
}

#[test]
fn implicit_dependency_does_not_replace_explicit_conflicting_types() {
    for ty in [
        Type::Struct {
            name: "MerkleTreeDigest".into(),
            fields: vec![],
        },
        Type::Struct {
            name: "MerkleTreeDigest".into(),
            fields: vec![StructField {
                name: "field".into(),
                ty: Type::Boolean,
            }],
        },
        Type::Struct {
            name: "r#MerkleTreeDigest".into(),
            fields: vec![StructField {
                name: "field".into(),
                ty: Type::Field,
            }],
        },
        Type::Enum {
            name: "MerkleTreeDigest".into(),
            variants: vec!["Value".into()],
        },
    ] {
        let mut c = contract(false);
        alias(&mut c, ty);
        assert!(matches!(
            crate::render(&c),
            Err(RenderError::ConflictingStruct(_)) | Err(RenderError::ConflictingEnum(_))
        ));
    }
}

#[test]
fn conflicting_type_already_in_witness_keeps_ledger_source_diagnostic() {
    let mut c = contract(false);
    c.witnesses[0].result = Type::Struct {
        name: "MerkleTreeDigest".into(),
        fields: vec![],
    };
    let source = crate::ir::SourceLocation {
        file: "implicit.compact".into(),
        line: 2,
        column: 1,
    };
    c.ledger_fields[0].source = Some(source.clone());
    assert!(
        matches!(crate::render(&c), Err(RenderError::Located { location, error })
        if location == source && *error == RenderError::ConflictingStruct("MerkleTreeDigest".into()))
    );
}

#[test]
fn witness_free_or_non_merkle_contracts_gain_no_digest() {
    for historic in [false, true] {
        let mut c = contract(historic);
        c.witnesses.clear();
        assert!(!crate::render(&c).unwrap().contains("MerkleTreeDigest"));
    }
    let mut c = contract(false);
    c.ledger_fields[0].declaration = LedgerFieldKind::Counter;
    assert!(!crate::render(&c).unwrap().contains("MerkleTreeDigest"));
}

#[test]
fn new_conversion_is_exact_and_does_not_match_lookalikes() {
    for name in ["OtherDigest", "MerkleTreeDigest_0", "MerkleTreeDigest$0"] {
        let mut c = contract(false);
        alias(
            &mut c,
            Type::Struct {
                name: name.into(),
                fields: vec![StructField {
                    name: "field".into(),
                    ty: Type::Field,
                }],
            },
        );
        let output = crate::render(&c).unwrap();
        let declarations = structs(&output);
        assert_eq!(declarations.len(), 2);
        for item in declarations {
            assert_eq!(has_digest_derive(&item), item.ident == "MerkleTreeDigest");
        }
    }
}

#[test]
fn standalone_explicit_digest_does_not_gain_an_unneeded_conversion() {
    let mut c = contract(false);
    c.witnesses.clear();
    alias(&mut c, merkle_digest());
    let declarations = structs(&crate::render(&c).unwrap());
    assert_eq!(declarations.len(), 1);
    assert!(!has_digest_derive(&declarations[0]));
}
