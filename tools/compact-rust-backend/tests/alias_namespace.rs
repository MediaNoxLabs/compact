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
    Contract, LedgerField, LedgerFieldKind, SCHEMA_VERSION, SourceLocation, Type, TypeAlias,
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
        .write_all(&serde_json::to_vec(&contract("ledger_slots", true)).unwrap())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8(output.stderr).unwrap(),
        "compact-rust-backend: alias.compact line 2 char 1: conflicting exported type alias \"ledger_slots\"\n"
    );
}
