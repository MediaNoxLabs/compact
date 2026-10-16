// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//  	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Schema and source diagnostics, including the backend CLI.

use super::support::identity;
use compact_rust_backend::ir::{
    Constructor, ConstructorStep, Contract, Expr, LedgerField, LedgerFieldKind, SCHEMA_VERSION,
    SourceLocation, StateAction, StateReturn, StatefulCircuit, Type, TypeAlias, WitnessDeclaration,
};
use compact_rust_backend::{RenderError, render};
use std::io::Write;
use std::process::{Command, Stdio};

#[test]
fn ledger_validation_retains_compact_source_location() {
    let mut contract = identity(Type::Unit, Expr::Unit);
    let location = SourceLocation {
        file: "vault.compact".into(),
        line: 2,
        column: 8,
    };
    contract.ledger_fields.push(LedgerField {
        source: Some(location.clone()),
        id: "balance".into(),
        index: 3,
        path: vec![3, 0],
        declaration: LedgerFieldKind::Counter,
    });
    let error = render(&contract).unwrap_err();
    assert_eq!(
        error,
        RenderError::Located {
            location,
            error: Box::new(RenderError::InvalidLedgerPath(vec![3, 0])),
        }
    );
    assert_eq!(
        error.to_string(),
        "vault.compact line 2 char 8: invalid ledger field path [3, 0]"
    );
    assert!(std::error::Error::source(&error).is_some());
}

#[test]
fn backend_cli_prints_source_diagnostic_instead_of_debug_structure() {
    let mut contract = identity(Type::Unit, Expr::Unit);
    contract.ledger_fields.push(LedgerField {
        source: Some(SourceLocation {
            file: "vault.compact".into(),
            line: 2,
            column: 8,
        }),
        id: "balance".into(),
        index: 3,
        path: vec![3, 0],
        declaration: LedgerFieldKind::Counter,
    });
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
        .write_all(&serde_json::to_vec(&contract).unwrap())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8(output.stderr).unwrap(),
        "compact-rust-backend: vault.compact line 2 char 8: invalid ledger field path [3, 0]\n"
    );
    assert!(output.stdout.is_empty());
}

#[test]
fn declaration_errors_retain_their_compact_owner_location() {
    let location = SourceLocation {
        file: "owners.compact".into(),
        line: 7,
        column: 1,
    };
    let located = |error| RenderError::Located {
        location: location.clone(),
        error: Box::new(error),
    };

    let mut pure = identity(Type::Field, Expr::Boolean { value: true });
    pure.circuits[0].source = Some(location.clone());
    assert_eq!(
        render(&pure),
        Err(located(RenderError::TypeMismatch {
            expected: Type::Field,
            actual: Type::Boolean,
        }))
    );

    let mut stateful = identity(Type::Unit, Expr::Unit);
    stateful.stateful_circuits.push(StatefulCircuit {
        source: Some(location.clone()),
        name: "write".into(),
        internal: false,
        parameters: vec![],
        actions: vec![StateAction::CellWrite {
            field: "missing".into(),
            index: 0,
            value: Expr::FieldLiteral { value: "1".into() },
        }],
        result: Type::Unit,
        return_value: StateReturn::Unit,
    });
    assert_eq!(
        render(&stateful),
        Err(located(RenderError::UnknownLedgerField("missing".into())))
    );

    let mut witnesses = identity(Type::Unit, Expr::Unit);
    for _ in 0..2 {
        witnesses.witnesses.push(WitnessDeclaration {
            source: Some(location.clone()),
            name: "answer".into(),
            parameters: vec![],
            result: Type::Field,
        });
    }
    assert_eq!(
        render(&witnesses),
        Err(located(RenderError::DuplicateWitness("answer".into())))
    );

    let mut constructor = identity(Type::Unit, Expr::Unit);
    constructor.constructor = Some(Constructor {
        source: Some(location.clone()),
        parameters: vec![],
        steps: vec![ConstructorStep::CellWrite {
            field: "missing".into(),
            index: 0,
            value: Expr::FieldLiteral { value: "1".into() },
        }],
    });
    assert_eq!(
        render(&constructor),
        Err(located(RenderError::UnknownLedgerField("missing".into())))
    );

    let mut aliases = identity(Type::Unit, Expr::Unit);
    for _ in 0..2 {
        aliases.type_aliases.push(TypeAlias {
            source: Some(location.clone()),
            name: "Amount".into(),
            ty: Type::Field,
        });
    }
    assert_eq!(
        render(&aliases),
        Err(located(RenderError::ConflictingTypeAlias("Amount".into())))
    );
}

#[test]
fn rejects_bad_schema_and_unknown_references() {
    let mut contract = identity(
        Type::Field,
        Expr::Parameter {
            name: "value".into(),
        },
    );
    contract.schema_version = 2;
    assert_eq!(render(&contract), Err(RenderError::SchemaVersion(2)));
    contract.schema_version = 4;
    assert_eq!(render(&contract), Err(RenderError::SchemaVersion(4)));
    contract.schema_version = 6;
    assert_eq!(render(&contract), Err(RenderError::SchemaVersion(6)));
    contract.schema_version = 7;
    assert_eq!(render(&contract), Err(RenderError::SchemaVersion(7)));
    contract.schema_version = 8;
    assert_eq!(render(&contract), Err(RenderError::SchemaVersion(8)));
    contract.schema_version = 9;
    assert_eq!(render(&contract), Err(RenderError::SchemaVersion(9)));
    contract.schema_version = 10;
    assert_eq!(render(&contract), Err(RenderError::SchemaVersion(10)));
    contract.schema_version = 11;
    assert_eq!(render(&contract), Err(RenderError::SchemaVersion(11)));
    contract.schema_version = 12;
    assert_eq!(render(&contract), Err(RenderError::SchemaVersion(12)));
    contract.schema_version = 13;
    assert_eq!(render(&contract), Err(RenderError::SchemaVersion(13)));
    contract.schema_version = SCHEMA_VERSION;
    contract.circuits[0].body = Expr::Parameter {
        name: "missing".into(),
    };
    assert_eq!(
        render(&contract),
        Err(RenderError::UnknownParameter("missing".into()))
    );
}

#[test]
fn unknown_json_fields_are_rejected() {
    let json = r#"{"schema_version":4,"ledger_fields":[],"circuits":[],"stateful_circuits":[],"rust_source":"panic!()"}"#;
    assert!(serde_json::from_str::<Contract>(json).is_err());

    let json = r#"{"schema_version":4,"ledger_fields":[{"id":"round","index":0,"declaration":{"kind":"counter"},"rust_source":"panic!()"}],"circuits":[],"stateful_circuits":[]}"#;
    assert!(serde_json::from_str::<Contract>(json).is_err());
}
