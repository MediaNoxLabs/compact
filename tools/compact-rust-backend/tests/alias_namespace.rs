// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::io::Write;
use std::process::{Command, Stdio};

use compact_rust_backend::ir::{
    Contract, LedgerField, LedgerFieldKind, SCHEMA_VERSION, SourceLocation, StructField, Type,
    TypeAlias,
};
use compact_rust_backend::{RenderError, render};

fn contract(alias: &str, ledger: bool) -> Contract {
    Contract {
        schema_version: SCHEMA_VERSION,
        type_aliases: vec![TypeAlias {
            source: Some(SourceLocation {
                file: "alias.compact".into(),
                line: 2,
                column: 1,
            }),
            name: alias.into(),
            ty: Type::Field,
        }],
        ledger_fields: if ledger {
            vec![LedgerField {
                source: None,
                id: "count".into(),
                index: 0,
                path: vec![0],
                declaration: LedgerFieldKind::Counter,
            }]
        } else {
            vec![]
        },
        constructor: None,
        witnesses: vec![],
        circuits: vec![],
        stateful_circuits: vec![],
    }
}

#[test]
fn actual_slots_module_rejects_all_equivalent_alias_spellings_at_source() {
    for name in [
        "ledger_slots",
        "ledger$slots",
        "r#ledger_slots",
        "r#ledger$slots",
    ] {
        assert_eq!(
            render(&contract(name, true)),
            Err(RenderError::Located {
                location: SourceLocation {
                    file: "alias.compact".into(),
                    line: 2,
                    column: 1
                },
                error: Box::new(RenderError::ConflictingTypeAlias(name.into())),
            }),
            "{name}"
        );
    }
}

#[test]
fn absent_slots_module_leaves_source_alias_available() {
    for name in [
        "ledger_slots",
        "ledger$slots",
        "r#ledger_slots",
        "r#ledger$slots",
    ] {
        let source = render(&contract(name, false)).unwrap();
        let parsed = syn::parse_file(&source).unwrap();
        assert!(
            !parsed
                .items
                .iter()
                .any(|item| matches!(item, syn::Item::Mod(m) if m.ident == "ledger_slots"))
        );
        assert!(source.contains("pub type "));
        assert!(source.contains("pub use types::"));
    }
}

#[test]
fn distinct_alias_with_slots_remains_available() {
    let source = render(&contract("UserField", true)).unwrap();
    syn::parse_file(&source).unwrap();
    assert!(source.contains("pub mod ledger_slots"));
    assert!(source.contains("pub type UserField"));
}

#[test]
fn cli_refuses_collision_without_emitting_partial_rust() {
    for (name, ledger) in [
        ("ledger_slots", true),
        ("pure$circuits", false),
        ("r#types", false),
    ] {
        let mut child = Command::new(env!("CARGO_BIN_EXE_compact-rust-backend"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(&serde_json::to_vec(&contract(name, ledger)).unwrap())
            .unwrap();
        let output = child.wait_with_output().unwrap();
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert_eq!(
            String::from_utf8(output.stderr).unwrap(),
            format!(
                "compact-rust-backend: alias.compact line 2 char 1: conflicting exported type alias {name:?}\n"
            )
        );
    }
}

fn assert_alias_conflict(contract: &Contract, name: &str) {
    assert_eq!(
        render(contract),
        Err(RenderError::Located {
            location: SourceLocation {
                file: "alias.compact".into(),
                line: 2,
                column: 1,
            },
            error: Box::new(RenderError::ConflictingTypeAlias(name.into())),
        }),
        "{name}"
    );
}

#[test]
fn reserved_modules_use_emitted_alias_names() {
    // `$` spellings are Compact source names; `r#` spellings are typed IR cases.
    for name in [
        "pure$circuits",
        "ledger$contract",
        "r#runtime",
        "r#types",
        "r#pure_circuits",
        "r#ledger$contract",
    ] {
        assert_alias_conflict(&contract(name, false), name);
    }
}

#[test]
fn equivalent_aliases_conflict_in_either_order() {
    for (first, second) in [
        ("a$b", "a_b"),
        ("a_b", "a$b"),
        ("r#a_b", "a$b"),
        ("a$b", "r#a_b"),
    ] {
        let mut input = contract(first, false);
        let mut alias = input.type_aliases[0].clone();
        alias.name = second.into();
        input.type_aliases.push(alias);
        assert_alias_conflict(&input, second);
    }
}

#[test]
fn aliases_cannot_shadow_normalized_struct_or_enum_names() {
    for (alias, named) in [
        ("User_Value", "User$Value"),
        ("User$Value", "User_Value"),
        ("r#User_Value", "User$Value"),
    ] {
        for ty in [
            Type::Struct {
                name: named.into(),
                fields: vec![],
            },
            Type::Enum {
                name: named.into(),
                variants: vec!["First".into()],
            },
        ] {
            let mut input = contract(alias, false);
            input.ledger_fields.push(LedgerField {
                source: None,
                id: "value".into(),
                index: 0,
                path: vec![0],
                declaration: LedgerFieldKind::Cell { ty },
            });
            assert_alias_conflict(&input, alias);
        }
    }
}

#[test]
fn distinct_alias_and_normalized_named_type_remain_available() {
    let mut input = contract("UserAlias", false);
    input.type_aliases[0].ty = Type::Struct {
        name: "User$Value".into(),
        fields: vec![],
    };
    input.ledger_fields.push(LedgerField {
        source: None,
        id: "value".into(),
        index: 0,
        path: vec![0],
        declaration: LedgerFieldKind::Cell {
            ty: input.type_aliases[0].ty.clone(),
        },
    });
    let rendered = render(&input).unwrap();
    assert!(rendered.contains("pub struct User_Value"));
    assert!(rendered.contains("pub type UserAlias = crate::types::User_Value"));
}

fn named_type(name: &str, is_enum: bool) -> Type {
    if is_enum {
        Type::Enum {
            name: name.into(),
            variants: vec!["First".into()],
        }
    } else {
        Type::Struct {
            name: name.into(),
            fields: vec![],
        }
    }
}

fn named_ledger_fields(first: Type, second: Type) -> Contract {
    let mut input = contract("UnusedAlias", false);
    input.type_aliases.clear();
    input.ledger_fields = [first, second]
        .into_iter()
        .enumerate()
        .map(|(index, ty)| LedgerField {
            source: Some(SourceLocation {
                file: "types.compact".into(),
                line: index + 5,
                column: 1,
            }),
            id: format!("value_{index}"),
            index: index as u8,
            path: vec![index as u8],
            declaration: LedgerFieldKind::Cell { ty },
        })
        .collect();
    input
}

#[test]
fn declared_types_share_one_emitted_namespace_with_source_locations() {
    // Exercise both collection orders and all kind pairs. Raw `r#` inputs
    // are typed-IR defense cases, not Compact source spellings.
    for (first, second) in [
        ("a$b", "a_b"),
        ("a_b", "a$b"),
        ("r#a_b", "a$b"),
        ("a$b", "r#a_b"),
    ] {
        for first_is_enum in [false, true] {
            for second_is_enum in [false, true] {
                let input = named_ledger_fields(
                    named_type(first, first_is_enum),
                    named_type(second, second_is_enum),
                );
                let error = if second_is_enum {
                    RenderError::ConflictingEnum(second.into())
                } else {
                    RenderError::ConflictingStruct(second.into())
                };
                assert_eq!(
                    render(&input),
                    Err(RenderError::Located {
                        location: SourceLocation {
                            file: "types.compact".into(),
                            line: 6,
                            column: 1
                        },
                        error: Box::new(error),
                    }),
                    "{first}, {second}, enum: {first_is_enum}/{second_is_enum}"
                );
            }
        }
    }
}

#[test]
fn repeated_raw_type_definitions_and_distinct_types_remain_valid() {
    for name in ["a$b", "a_b", "r#a_b"] {
        for is_enum in [false, true] {
            let ty = named_type(name, is_enum);
            let repeated = render(&named_ledger_fields(ty.clone(), ty)).unwrap();
            let parsed = syn::parse_file(&repeated).unwrap();
            let types = parsed
                .items
                .iter()
                .find_map(|item| match item {
                    syn::Item::Mod(module) if module.ident == "types" => module.content.as_ref(),
                    _ => None,
                })
                .unwrap();
            assert_eq!(types.1.len(), 1, "one declaration for repeated {name}");
        }
    }
    for first_is_enum in [false, true] {
        for second_is_enum in [false, true] {
            render(&named_ledger_fields(
                named_type("a$b", first_is_enum),
                named_type("a$c", second_is_enum),
            ))
            .unwrap();
        }
    }
}

#[test]
fn alias_only_struct_and_enum_definitions_are_emitted() {
    for is_enum in [false, true] {
        let mut input = contract("Alias", false);
        input.type_aliases[0].ty = named_type("User$Value", is_enum);
        let rendered = render(&input).unwrap();
        assert!(rendered.contains(if is_enum {
            "pub enum User_Value"
        } else {
            "pub struct User_Value"
        }));
        assert!(rendered.contains("pub type Alias = crate::types::User_Value;"));
    }
}

#[test]
fn alias_only_named_types_are_collected_through_nested_containers_and_fields() {
    let mut input = contract("Nested", false);
    input.type_aliases[0].ty = Type::Vector {
        length: 2,
        element: Box::new(Type::Tuple {
            elements: vec![
                Type::Struct {
                    name: "Record".into(),
                    fields: vec![StructField {
                        name: "choice".into(),
                        ty: named_type("Choice", true),
                    }],
                },
                Type::Field,
            ],
        }),
    };
    let rendered = render(&input).unwrap();
    assert!(rendered.contains("pub struct Record"));
    assert!(rendered.contains("pub choice: crate::types::Choice"));
    assert!(rendered.contains("pub enum Choice"));
    assert!(rendered.contains("pub type Nested ="));
    assert!(rendered.contains("FixedVector<"));
}

#[test]
fn an_alias_cannot_silently_replace_an_alias_only_named_type() {
    for is_enum in [false, true] {
        // Raw spelling is a manual typed-IR case; `$` is valid source syntax.
        for spelling in ["Foo_Bar", "Foo$Bar", "r#Foo_Bar"] {
            for reverse in [false, true] {
                let mut input = contract(spelling, false);
                let mut named = input.type_aliases[0].clone();
                named.name = "A".into();
                named.ty = named_type("Foo$Bar", is_enum);
                input.type_aliases.push(named);
                if reverse {
                    input.type_aliases.reverse();
                }
                assert_alias_conflict(&input, spelling);
            }
        }
    }
}

#[test]
fn alias_only_named_type_conflicts_retain_the_alias_owner_location() {
    let mut input = contract("First", false);
    input.type_aliases[0].ty = named_type("a$b", false);
    let mut second = input.type_aliases[0].clone();
    second.name = "Second".into();
    second.source.as_mut().unwrap().line = 3;
    second.ty = named_type("a_b", true);
    input.type_aliases.push(second);
    assert_eq!(
        render(&input),
        Err(RenderError::Located {
            location: SourceLocation {
                file: "alias.compact".into(),
                line: 3,
                column: 1
            },
            error: Box::new(RenderError::ConflictingEnum("a_b".into())),
        })
    );
}
