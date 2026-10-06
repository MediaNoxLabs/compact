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

use super::*;
use crate::ir::{Contract, LocalBinding, SCHEMA_VERSION};
fn fixture(counter: bool) -> Contract {
    let mut c: Contract =
        serde_json::from_str(include_str!("../../tests/unit-composition/flat.json")).unwrap();
    c.schema_version = SCHEMA_VERSION;
    c.witnesses.clear();
    c.circuits.clear();
    c.constructor = None;
    c.type_aliases.clear();
    c.ledger_fields = vec![LedgerField {
        source: None,
        id: "slot".into(),
        index: 0,
        path: vec![0],
        declaration: if counter {
            LedgerFieldKind::Counter
        } else {
            LedgerFieldKind::Cell { ty: Type::Boolean }
        },
    }];
    let action = if counter {
        StateAction::Let {
            bindings: vec![LocalBinding {
                name: "amount".into(),
                ty: Type::Unsigned {
                    max: "65535".into(),
                },
                value: Expr::UnsignedLiteral {
                    value: "1".into(),
                    max: "65535".into(),
                },
            }],
            action: Box::new(StateAction::CounterIncrement {
                field: "slot".into(),
                index: 0,
                amount: CounterAmount::Parameter {
                    name: "amount".into(),
                },
            }),
        }
    } else {
        StateAction::CellWrite {
            field: "slot".into(),
            index: 0,
            value: Expr::Boolean { value: true },
        }
    };
    c.stateful_circuits = vec![StatefulCircuit {
        source: None,
        name: "run".into(),
        internal: false,
        parameters: vec![],
        result: Type::Unit,
        return_value: StateReturn::Unit,
        actions: vec![action],
    }];
    c
}
fn leaf(c: &Contract) -> Option<syn::Item> {
    let fields = c.ledger_fields.iter().map(|f| (f.id.as_str(), f)).collect();
    render_simple_leaf(&c.stateful_circuits[0], &fields)
}
#[test]
fn boolean_values_and_checked_counter_bounds_use_existing_frame() {
    for value in [false, true] {
        let mut c = fixture(false);
        if let StateAction::CellWrite { value: v, .. } = &mut c.stateful_circuits[0].actions[0] {
            *v = Expr::Boolean { value };
        }
        let item = leaf(&c).unwrap();
        let source = quote::quote!(#item).to_string();
        assert!(source.contains("CircuitFrame"));
        assert!(source.contains("apply"));
        assert!(!source.contains("total_cost"));
    }
    for value in ["0", "1", "65535"] {
        let mut c = fixture(true);
        if let StateAction::Let { bindings, .. } = &mut c.stateful_circuits[0].actions[0] {
            bindings[0].value = Expr::UnsignedLiteral {
                value: value.into(),
                max: "65535".into(),
            };
        }
        let item = leaf(&c).unwrap();
        let source = quote::quote!(#item).to_string();
        assert!(source.contains("Compact Uint literal fits its maximum"));
        assert!(source.contains("increment"));
        assert!(source.contains("as u16"));
    }
}
#[test]
fn unsupported_shapes_keep_general_fallback() {
    for counter in [false, true] {
        let base = fixture(counter);
        let mut c = base.clone();
        c.stateful_circuits[0]
            .actions
            .push(StateAction::Sequence { actions: vec![] });
        assert!(leaf(&c).is_none());
        let mut c = base.clone();
        c.stateful_circuits[0].result = Type::Field;
        assert!(leaf(&c).is_none());
        let mut c = base.clone();
        c.ledger_fields[0].index = 1;
        assert!(leaf(&c).is_none());
        let mut c = base.clone();
        c.ledger_fields[0].declaration = LedgerFieldKind::Cell { ty: Type::Field };
        assert!(leaf(&c).is_none());
        let mut c = base.clone();
        c.stateful_circuits[0].actions = vec![StateAction::If {
            condition: Expr::Boolean { value: true },
            then: Box::new(base.stateful_circuits[0].actions[0].clone()),
            otherwise: Box::new(StateAction::Sequence { actions: vec![] }),
        }];
        assert!(leaf(&c).is_none());
    }
    let mut c = fixture(true);
    if let StateAction::Let { bindings, .. } = &mut c.stateful_circuits[0].actions[0] {
        bindings.push(bindings[0].clone());
    }
    assert!(leaf(&c).is_none());
    let mut c = fixture(true);
    if let StateAction::Let { action, .. } = &mut c.stateful_circuits[0].actions[0]
        && let StateAction::CounterIncrement { amount, .. } = action.as_mut()
    {
        *amount = CounterAmount::Parameter {
            name: "other".into(),
        };
    }
    assert!(leaf(&c).is_none());
}
#[test]
fn malformed_literals_and_names_leave_baseline_diagnostics() {
    for value in ["65536", "01", "-1", "x"] {
        let mut c = fixture(true);
        if let StateAction::Let { bindings, .. } = &mut c.stateful_circuits[0].actions[0] {
            bindings[0].value = Expr::UnsignedLiteral {
                value: value.into(),
                max: "65535".into(),
            };
        }
        assert!(leaf(&c).is_none(), "{value}");
        assert!(
            matches!(crate::render(&c), Err(RenderError::InvalidUnsignedLiteral { value: actual, max }) if actual == value && max == "65535"),
            "{value}"
        );
    }
    let mut c = fixture(true);
    if let StateAction::Let { bindings, action } = &mut c.stateful_circuits[0].actions[0] {
        bindings[0].name = "bad-name".into();
        if let StateAction::CounterIncrement { amount, .. } = action.as_mut() {
            *amount = CounterAmount::Parameter {
                name: "bad-name".into(),
            };
        }
    }
    assert!(leaf(&c).is_none());
    assert!(matches!(
        crate::render(&c),
        Err(RenderError::InvalidIdentifier(_))
    ));
}

#[test]
fn parameters_witnesses_calls_sequences_and_other_counter_operations_are_not_leaves() {
    let mut c = fixture(false);
    c.stateful_circuits[0]
        .parameters
        .push(crate::ir::Parameter {
            name: "flag".into(),
            ty: Type::Boolean,
        });
    assert!(leaf(&c).is_none());
    for action in [
        StateAction::Sequence {
            actions: fixture(false).stateful_circuits[0].actions.clone(),
        },
        StateAction::CircuitCall {
            name: "helper".into(),
            arguments: vec![],
        },
        StateAction::CellWrite {
            field: "slot".into(),
            index: 0,
            value: Expr::WitnessCall {
                name: "flag".into(),
                arguments: vec![],
            },
        },
        StateAction::CounterReset {
            field: "slot".into(),
            index: 0,
        },
        StateAction::CounterDecrement {
            field: "slot".into(),
            index: 0,
            amount: CounterAmount::Literal { value: 1 },
        },
        StateAction::CounterIncrement {
            field: "slot".into(),
            index: 0,
            amount: CounterAmount::Literal { value: 1 },
        },
    ] {
        let mut c = fixture(true);
        c.stateful_circuits[0].actions = vec![action];
        assert!(leaf(&c).is_none());
    }
    let mut c = fixture(true);
    if let StateAction::Let { bindings, .. } = &mut c.stateful_circuits[0].actions[0] {
        bindings[0].ty = Type::Unsigned { max: "255".into() };
    }
    assert!(leaf(&c).is_none());
}
