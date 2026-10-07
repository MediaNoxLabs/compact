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

use compact_rust_backend::{
    RenderError,
    ir::{Contract, SCHEMA_VERSION},
    render,
};
use serde_json::{Value, json};

const WIDE: &str = "340282366920938463463374607431768211456";

fn contract(body: Value, result: Value, stateful: bool) -> Contract {
    let mut circuit = json!({
        "name": "probe",
        "source": {"file": "wide.compact", "line": 7, "column": 3},
        "parameters": [
            {"name": "left", "ty": {"kind": "unsigned", "max": WIDE}},
            {"name": "right", "ty": {"kind": "unsigned", "max": WIDE}}
        ],
        "result": result
    });
    if stateful {
        circuit["actions"] = json!([]);
        circuit["return_value"] = json!({"kind": "expression", "value": body});
    } else {
        circuit["body"] = body;
    }
    serde_json::from_value(json!({
        "schema_version": SCHEMA_VERSION,
        "ledger_fields": [],
        "circuits": if stateful {json!([])} else {json!([circuit.clone()])},
        "stateful_circuits": if stateful {json!([circuit])} else {json!([])}
    }))
    .unwrap()
}

fn arithmetic(kind: &str, max: &str) -> Value {
    json!({"kind": kind, "max": max,
        "left": {"kind": "parameter", "name": "left"},
        "right": {"kind": "parameter", "name": "right"}})
}

fn uint() -> Value {
    json!({"kind": "unsigned", "max": WIDE})
}

#[test]
fn valid_wide_subtraction_names_unsupported_operation_not_invalid_maximum() {
    let error = render(&contract(
        arithmetic("unsigned_subtract", WIDE),
        uint(),
        false,
    ))
    .unwrap_err();
    assert_eq!(
        error.to_string(),
        format!(
            "wide.compact line 7 char 3: unsupported wide Compact Uint subtraction for maximum {WIDE:?}"
        )
    );
}

fn unlocated(error: &RenderError) -> &RenderError {
    let RenderError::Located { location, error } = error else {
        panic!("expected source location, got {error:?}");
    };
    assert_eq!(
        (location.file.as_str(), location.line, location.column),
        ("wide.compact", 7, 3)
    );
    error
}

#[test]
fn pure_and_stateful_wide_arithmetic_preserve_support_and_refusal_classes() {
    for stateful in [false, true] {
        let identity = contract(
            json!({"kind": "parameter", "name": "left"}),
            uint(),
            stateful,
        );
        assert!(render(&identity).unwrap().contains("runtime::WideUint"));
        let add = contract(arithmetic("unsigned_add", WIDE), uint(), stateful);
        assert!(render(&add).unwrap().contains("runtime::add_wide_unsigned"));
        for (kind, operation, maximum) in [
            ("unsigned_subtract", "subtraction", WIDE),
            ("unsigned_multiply", "multiplication", WIDE),
            // Addition supports wide results, not wide inputs into the narrow helper.
            ("unsigned_add", "addition", "255"),
        ] {
            let error = render(&contract(arithmetic(kind, maximum), uint(), stateful)).unwrap_err();
            assert_eq!(
                unlocated(&error),
                &RenderError::UnsupportedWideUnsignedOperation {
                    operation,
                    max: WIDE.into()
                }
            );
            assert_eq!(
                error.to_string(),
                format!(
                    "wide.compact line 7 char 3: unsupported wide Compact Uint {operation} for maximum {WIDE:?}"
                )
            );
        }
    }
}

#[test]
fn either_wide_comparison_operand_has_the_operation_diagnostic() {
    for stateful in [false, true] {
        for operator in ["less", "less_equal", "greater", "greater_equal"] {
            for wide_left in [false, true] {
                let operand = |wide| {
                    if wide {
                        json!({"kind": "parameter", "name": "left"})
                    } else {
                        json!({"kind": "unsigned_literal", "max": "255", "value": "1"})
                    }
                };
                let body = json!({"kind": "compare", "operator": operator,
                    "left": operand(wide_left), "right": operand(!wide_left)});
                let error =
                    render(&contract(body, json!({"kind": "boolean"}), stateful)).unwrap_err();
                assert_eq!(
                    unlocated(&error),
                    &RenderError::UnsupportedWideUnsignedOperation {
                        operation: "ordered comparison",
                        max: WIDE.into()
                    }
                );
                assert_eq!(
                    error.to_string(),
                    format!(
                        "wide.compact line 7 char 3: unsupported wide Compact Uint ordered comparison for maximum {WIDE:?}"
                    )
                );
            }
        }
    }
}

#[test]
fn malformed_result_maximum_keeps_its_class_before_wide_operation_refusal() {
    for stateful in [false, true] {
        for max in [
            "01",
            "-1",
            "bad",
            "452312848583266388373324160190187140051835877600158453279131187530910662656",
        ] {
            for kind in ["unsigned_add", "unsigned_subtract", "unsigned_multiply"] {
                let error = render(&contract(arithmetic(kind, max), uint(), stateful)).unwrap_err();
                assert_eq!(
                    unlocated(&error),
                    &RenderError::InvalidUnsignedMaximum(max.into())
                );
                assert_eq!(
                    error.to_string(),
                    format!(
                        "wide.compact line 7 char 3: invalid Compact Uint maximum {max:?}; expected a canonical decimal integer from 0 through 2^248 - 1"
                    )
                );
            }
        }
    }
}

#[test]
fn comparison_keeps_each_emitters_existing_operand_error_precedence() {
    let body = json!({"kind": "compare", "operator": "less",
        "left": {"kind": "parameter", "name": "left"},
        "right": {"kind": "parameter", "name": "missing"}});
    let pure = render(&contract(body.clone(), json!({"kind": "boolean"}), false)).unwrap_err();
    assert_eq!(
        unlocated(&pure),
        &RenderError::UnknownParameter("missing".into())
    );
    let stateful = render(&contract(body, json!({"kind": "boolean"}), true)).unwrap_err();
    assert_eq!(
        unlocated(&stateful),
        &RenderError::UnsupportedWideUnsignedOperation {
            operation: "ordered comparison",
            max: WIDE.into()
        }
    );
}
