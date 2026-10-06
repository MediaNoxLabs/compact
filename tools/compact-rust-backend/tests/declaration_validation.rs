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
    Contract, Expr, Parameter, PureCircuit, SCHEMA_VERSION, SourceLocation, StateAction,
    StateReturn, StatefulCircuit, Type,
};
use compact_rust_backend::{RenderError, render};

fn empty() -> Contract {
    Contract {
        schema_version: SCHEMA_VERSION,
        type_aliases: vec![],
        ledger_fields: vec![],
        constructor: None,
        witnesses: vec![],
        circuits: vec![],
        stateful_circuits: vec![],
    }
}

fn source(line: usize) -> Option<SourceLocation> {
    Some(SourceLocation {
        file: "declarations.compact".into(),
        line,
        column: 3,
    })
}

fn pure(name: &str, line: usize, result: Type, body: Expr) -> PureCircuit {
    PureCircuit {
        source: source(line),
        name: name.into(),
        internal: false,
        parameters: vec![],
        result,
        body,
    }
}

fn stateful(name: &str, line: usize, actions: Vec<StateAction>) -> StatefulCircuit {
    StatefulCircuit {
        source: source(line),
        name: name.into(),
        internal: false,
        parameters: vec![],
        actions,
        result: Type::Unit,
        return_value: StateReturn::Unit,
    }
}

fn located(line: usize, error: RenderError) -> RenderError {
    RenderError::Located {
        location: source(line).unwrap(),
        error: Box::new(error),
    }
}

fn duplicate_pure_before_caller_error() -> Contract {
    let mut contract = empty();
    contract.circuits = vec![
        pure(
            "caller",
            2,
            Type::Field,
            Expr::Call {
                name: "target".into(),
                arguments: vec![],
            },
        ),
        pure(
            "target",
            4,
            Type::Field,
            Expr::FieldLiteral { value: "1".into() },
        ),
        pure("target", 7, Type::Boolean, Expr::Boolean { value: true }),
    ];
    contract
}

#[test]
fn duplicate_pure_owner_wins_over_prior_caller_type_error() {
    let contract = duplicate_pure_before_caller_error();
    assert_eq!(
        render(&contract),
        Err(located(7, RenderError::DuplicateCircuit("target".into())))
    );
}

#[test]
fn cli_reports_the_duplicate_owner_not_the_prior_caller() {
    let contract = duplicate_pure_before_caller_error();
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
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8(output.stderr).unwrap(),
        "compact-rust-backend: declarations.compact line 7 char 3: duplicate circuit \"target\"\n"
    );
}

#[test]
fn duplicate_stateful_owner_wins_over_prior_caller_signature_error() {
    let mut contract = empty();
    contract.stateful_circuits = vec![
        stateful(
            "caller",
            2,
            vec![StateAction::CircuitCall {
                name: "target".into(),
                arguments: vec![],
            }],
        ),
        stateful("target", 4, vec![]),
        StatefulCircuit {
            parameters: vec![Parameter {
                name: "extra".into(),
                ty: Type::Field,
            }],
            ..stateful("target", 7, vec![])
        },
    ];
    assert_eq!(
        render(&contract),
        Err(located(7, RenderError::DuplicateCircuit("target".into())))
    );
}

#[test]
fn cross_kind_duplicate_identifies_the_later_declaration() {
    let mut contract = empty();
    contract
        .circuits
        .push(pure("same", 2, Type::Unit, Expr::Unit));
    contract.stateful_circuits.push(stateful("same", 9, vec![]));
    assert_eq!(
        render(&contract),
        Err(located(9, RenderError::DuplicateCircuit("same".into())))
    );
}

#[test]
fn invalid_declaration_identifier_is_located_before_lookup() {
    let mut contract = empty();
    contract
        .circuits
        .push(pure("bad-name", 4, Type::Unit, Expr::Unit));
    assert_eq!(
        render(&contract),
        Err(located(
            4,
            RenderError::InvalidIdentifier("bad-name".into())
        ))
    );

    let mut contract = empty();
    contract
        .stateful_circuits
        .push(stateful("bad-name", 8, vec![]));
    assert_eq!(
        render(&contract),
        Err(located(
            8,
            RenderError::InvalidIdentifier("bad-name".into())
        ))
    );
}

#[test]
fn distinct_forward_calls_still_render() {
    let mut contract = empty();
    contract.circuits = vec![
        pure(
            "caller",
            2,
            Type::Field,
            Expr::Call {
                name: "target".into(),
                arguments: vec![],
            },
        ),
        pure(
            "target",
            7,
            Type::Field,
            Expr::FieldLiteral { value: "1".into() },
        ),
    ];
    let emitted = render(&contract).unwrap();
    syn::parse_file(&emitted).unwrap();
    assert!(emitted.contains("pub fn caller("));
    assert!(emitted.contains("pub fn target("));

    let mut contract = empty();
    contract.stateful_circuits = vec![
        stateful(
            "caller",
            2,
            vec![StateAction::CircuitCall {
                name: "target".into(),
                arguments: vec![],
            }],
        ),
        stateful("target", 7, vec![]),
    ];
    let emitted = render(&contract).unwrap();
    syn::parse_file(&emitted).unwrap();
    assert!(emitted.contains("pub fn caller<"));
    assert!(emitted.contains("pub fn target<"));
}

#[test]
fn distinct_compact_names_must_not_emit_the_same_rust_function() {
    let mut pure_contract = empty();
    pure_contract.circuits = vec![
        pure("a$b", 2, Type::Unit, Expr::Unit),
        pure("a_b", 7, Type::Unit, Expr::Unit),
    ];
    let error = render(&pure_contract).unwrap_err();
    assert_eq!(
        error,
        located(
            7,
            RenderError::ConflictingCircuitIdentifier {
                first: "a$b".into(),
                second: "a_b".into(),
                rust_identifier: "a_b".into(),
            }
        )
    );
    assert_eq!(
        error.to_string(),
        "declarations.compact line 7 char 3: circuits \"a$b\" and \"a_b\" both emit Rust identifier \"a_b\""
    );

    let mut stateful_contract = empty();
    stateful_contract.stateful_circuits =
        vec![stateful("a$b", 2, vec![]), stateful("a_b", 7, vec![])];
    assert_eq!(
        render(&stateful_contract),
        Err(located(
            7,
            RenderError::ConflictingCircuitIdentifier {
                first: "a$b".into(),
                second: "a_b".into(),
                rust_identifier: "a_b".into(),
            }
        ))
    );
}

#[test]
fn raw_identifiers_do_not_create_a_second_function_name() {
    let mut contract = empty();
    contract.circuits = vec![
        pure("foo", 2, Type::Unit, Expr::Unit),
        pure("r#foo", 7, Type::Unit, Expr::Unit),
    ];
    let error = render(&contract).unwrap_err();
    assert_eq!(
        error,
        located(
            7,
            RenderError::ConflictingCircuitIdentifier {
                first: "foo".into(),
                second: "r#foo".into(),
                rust_identifier: "foo".into(),
            }
        )
    );
    assert_eq!(
        error.to_string(),
        "declarations.compact line 7 char 3: circuits \"foo\" and \"r#foo\" both emit Rust identifier \"foo\""
    );

    let mut contract = empty();
    contract.stateful_circuits = vec![stateful("a$b", 3, vec![]), stateful("r#a_b", 9, vec![])];
    assert_eq!(
        render(&contract),
        Err(located(
            9,
            RenderError::ConflictingCircuitIdentifier {
                first: "a$b".into(),
                second: "r#a_b".into(),
                rust_identifier: "a_b".into(),
            }
        ))
    );
}

#[test]
fn raw_duplicate_has_precedence_over_normalized_collision() {
    let mut contract = empty();
    contract.circuits = vec![
        pure("a$b", 2, Type::Unit, Expr::Unit),
        pure("a_b", 4, Type::Unit, Expr::Unit),
        pure("a$b", 7, Type::Unit, Expr::Unit),
    ];
    assert_eq!(
        render(&contract),
        Err(located(7, RenderError::DuplicateCircuit("a$b".into())))
    );
}

#[test]
fn normalization_can_be_used_once_in_each_function_namespace() {
    let mut contract = empty();
    contract
        .circuits
        .push(pure("a$b", 2, Type::Unit, Expr::Unit));
    contract.stateful_circuits.push(stateful("a_b", 7, vec![]));
    let emitted = render(&contract).unwrap();
    syn::parse_file(&emitted).unwrap();
    assert!(emitted.contains("pub fn a_b()"));
    assert!(emitted.contains("pub fn a_b<"));
}
