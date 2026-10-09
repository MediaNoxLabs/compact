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

//! Typed set mutation, membership queries, and recorded key bindings.

use super::support::identity;
use compact_rust_backend::ir::{
    Contract, Expr, LedgerField, LedgerFieldKind, LocalBinding, Parameter, PureCircuit,
    SCHEMA_VERSION, StateAction, StateReturn, StatefulCircuit, Type, WitnessDeclaration,
};
use compact_rust_backend::{RenderError, render, render_with_capabilities};

#[test]
fn opaque_string_set_recording_requires_closed_typed_operations() {
    let mut contract: Contract =
        serde_json::from_str(include_str!("../opaque-string-set-schema13-ir.json")).unwrap();
    contract.schema_version = SCHEMA_VERSION;
    let statuses = |contract: &Contract| {
        render_with_capabilities(contract)
            .unwrap()
            .capabilities
            .circuits
            .into_iter()
            .map(|capability| (capability.name, capability.recorded))
            .collect::<Vec<_>>()
    };
    assert_eq!(
        statuses(&contract),
        vec![("addName".into(), true), ("hasName".into(), true)]
    );

    let extra_action = contract.stateful_circuits[0].actions[0].clone();
    contract.stateful_circuits[0].actions.push(extra_action);
    assert_eq!(
        statuses(&contract),
        vec![("addName".into(), false), ("hasName".into(), true)]
    );

    contract.stateful_circuits[0].actions.truncate(1);
    let StateReturn::SetMember { value, .. } = &mut contract.stateful_circuits[1].return_value
    else {
        unreachable!()
    };
    *value = Expr::Default {
        ty: Type::OpaqueString,
    };
    assert_eq!(
        statuses(&contract),
        vec![("addName".into(), true), ("hasName".into(), false)]
    );
}

#[test]
fn opaque_set_check_in_records_only_typed_parameter_and_unit_witness() {
    let opaque = Type::OpaqueString;
    let parameter = Parameter {
        name: "participant".into(),
        ty: opaque.clone(),
    };
    let key = Expr::Parameter {
        name: "participant".into(),
    };
    let mut contract = Contract {
        schema_version: SCHEMA_VERSION,
        type_aliases: vec![],
        constructor: None,
        witnesses: vec![WitnessDeclaration {
            source: None,
            name: "set_local_id".into(),
            parameters: vec![parameter.clone()],
            result: Type::Unit,
        }],
        ledger_fields: vec![
            LedgerField {
                source: None,
                id: "eligible".into(),
                index: 0,
                path: vec![],
                declaration: LedgerFieldKind::Set { ty: opaque.clone() },
            },
            LedgerField {
                source: None,
                id: "checked".into(),
                index: 1,
                path: vec![],
                declaration: LedgerFieldKind::Set { ty: opaque.clone() },
            },
        ],
        circuits: vec![],
        stateful_circuits: vec![StatefulCircuit {
            source: None,
            internal: false,
            name: "check_in".into(),
            parameters: vec![parameter],
            result: Type::Unit,
            return_value: StateReturn::Unit,
            actions: vec![
                StateAction::Assert {
                    condition: Expr::SetMember {
                        field: "eligible".into(),
                        index: 0,
                        value: Box::new(key.clone()),
                    },
                    message: "not eligible".into(),
                },
                StateAction::SetInsert {
                    field: "checked".into(),
                    index: 1,
                    value: key.clone(),
                },
                StateAction::Expression {
                    value: Expr::WitnessCall {
                        name: "set_local_id".into(),
                        arguments: vec![Expr::Coerce {
                            value: Box::new(key),
                            ty: opaque.clone(),
                        }],
                    },
                },
            ],
        }],
    };
    let rendered = render_with_capabilities(&contract).unwrap();
    assert!(rendered.capabilities.circuits[0].recorded);
    assert!(rendered.capabilities.circuits[0].observed_call);
    assert!(rendered.source.contains("record_member"));
    assert!(rendered.source.contains("record_insert"));
    assert!(rendered.source.contains("try_witness_metered"));

    let mut incomplete = contract.clone();
    incomplete.stateful_circuits[0].actions.truncate(2);
    let rejected = render_with_capabilities(&incomplete).unwrap();
    assert!(!rejected.capabilities.circuits[0].recorded);
    assert_eq!(
        rejected.capabilities.circuits[0]
            .recording_unavailable
            .as_ref()
            .unwrap()
            .path,
        "actions[0]"
    );

    let StateAction::Expression {
        value: Expr::WitnessCall { arguments, .. },
    } = &mut contract.stateful_circuits[0].actions[2]
    else {
        unreachable!()
    };
    arguments[0] = Expr::Default { ty: opaque.clone() };
    let rejected = render_with_capabilities(&contract).unwrap();
    assert!(!rejected.capabilities.circuits[0].recorded);
    assert_eq!(
        rejected.capabilities.circuits[0]
            .recording_unavailable
            .as_ref()
            .unwrap()
            .path,
        "actions[2].value.arguments[0]"
    );

    contract.stateful_circuits[0].actions[2] = StateAction::SetRemove {
        field: "checked".into(),
        index: 1,
        value: Expr::Parameter {
            name: "participant".into(),
        },
    };
    let rejected = render_with_capabilities(&contract).unwrap();
    assert!(!rejected.capabilities.circuits[0].recorded);
    assert_eq!(
        rejected.capabilities.circuits[0]
            .recording_unavailable
            .as_ref()
            .unwrap()
            .path,
        "actions[2]"
    );

    contract.stateful_circuits[0].parameters[0].ty = Type::OpaqueBytes;
    contract.ledger_fields[0].declaration = LedgerFieldKind::Set {
        ty: Type::OpaqueBytes,
    };
    contract.ledger_fields[1].declaration = LedgerFieldKind::Set {
        ty: Type::OpaqueBytes,
    };
    let rejected = render_with_capabilities(&contract).unwrap();
    assert!(!rejected.capabilities.circuits[0].recorded);
    assert_eq!(
        rejected.capabilities.circuits[0]
            .recording_unavailable
            .as_ref()
            .unwrap()
            .path,
        "actions[0]"
    );
}

#[test]
fn nested_set_query_in_cell_write_checks_field_and_item_types() {
    let mut contract = identity(
        Type::Field,
        Expr::Parameter {
            name: "value".into(),
        },
    );
    contract.ledger_fields = vec![
        LedgerField {
            source: None,
            id: "flag".into(),
            index: 0,
            path: vec![],
            declaration: LedgerFieldKind::Cell { ty: Type::Boolean },
        },
        LedgerField {
            source: None,
            id: "seen".into(),
            index: 1,
            path: vec![],
            declaration: LedgerFieldKind::Set { ty: Type::Field },
        },
    ];
    contract.stateful_circuits = vec![StatefulCircuit {
        source: None,
        internal: false,
        name: "check".into(),
        parameters: vec![],
        actions: vec![StateAction::CellWrite {
            field: "flag".into(),
            index: 0,
            value: Expr::SetMember {
                field: "seen".into(),
                index: 1,
                value: Box::new(Expr::FieldLiteral { value: "1".into() }),
            },
        }],
        result: Type::Unit,
        return_value: StateReturn::Unit,
    }];
    let source = render(&contract).unwrap();
    assert!(source.contains("crate::ledger_slots::seen"));
    assert!(source.contains(".member(context,"));
    assert!(source.contains(".record_member(frame,"), "{source}");
    assert!(source.contains("total_cost += __compact_query_0.gas_cost"));
    let effectful_value = {
        let StateAction::CellWrite { value, .. } = &mut contract.stateful_circuits[0].actions[0]
        else {
            unreachable!()
        };
        let Expr::SetMember { value: item, .. } = value else {
            unreachable!()
        };
        **item = Expr::Boolean { value: true };
        value.clone()
    };
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Field,
            actual: Type::Boolean,
        })
    );
    contract.circuits[0].body = effectful_value;
    assert_eq!(render(&contract), Err(RenderError::EffectfulExpression));
}

#[test]
fn native_collection_emptiness_uses_typed_scalar_and_nested_slots() {
    let mut contract = identity(Type::Unit, Expr::Unit);
    contract.ledger_fields = vec![
        LedgerField {
            source: None,
            id: "flag".into(),
            index: 0,
            path: vec![],
            declaration: LedgerFieldKind::Cell { ty: Type::Boolean },
        },
        LedgerField {
            source: None,
            id: "entries".into(),
            index: 1,
            path: vec![],
            declaration: LedgerFieldKind::Set { ty: Type::Field },
        },
    ];
    contract.stateful_circuits = vec![StatefulCircuit {
        source: None,
        internal: false,
        name: "check".into(),
        parameters: vec![],
        actions: vec![StateAction::CellWrite {
            field: "flag".into(),
            index: 0,
            value: Expr::SetIsEmpty {
                field: "entries".into(),
                index: 1,
            },
        }],
        result: Type::Unit,
        return_value: StateReturn::Unit,
    }];

    let set_source = render(&contract).unwrap();
    assert!(set_source.contains("crate::ledger_slots::entries.is_empty(context)?"));
    assert!(!set_source.contains("context.is_empty_set(1)?"));

    contract.ledger_fields[1].declaration = LedgerFieldKind::Map {
        key: Type::Field,
        value: Type::Field,
    };
    let StateAction::CellWrite { value, .. } = &mut contract.stateful_circuits[0].actions[0] else {
        unreachable!()
    };
    *value = Expr::MapIsEmpty {
        field: "entries".into(),
        index: 1,
    };
    let map_source = render(&contract).unwrap();
    assert!(map_source.contains("crate::ledger_slots::entries.is_empty(context)?"));
    assert!(!map_source.contains("context.is_empty_map(1)?"));

    contract.ledger_fields[1].declaration = LedgerFieldKind::Map {
        key: Type::Field,
        value: Type::LedgerMap {
            key: Box::new(Type::Field),
            value: Box::new(Type::Field),
        },
    };
    let nested_source = render(&contract).unwrap();
    assert!(nested_source.contains("crate::ledger_slots::entries.is_empty(context)?"));
    assert!(nested_source.contains("runtime::slots::MapNode<runtime::Field, runtime::Field>"));
    assert!(nested_source.contains("pub const entries:"));
    assert!(!nested_source.contains("context.is_empty_map(1)?"));
}

#[test]
fn nested_set_size_assertion_records_one_ordered_query() {
    let mut contract = identity(Type::Unit, Expr::Unit);
    contract.ledger_fields = vec![LedgerField {
        source: None,
        id: "entries".into(),
        index: 0,
        path: vec![],
        declaration: LedgerFieldKind::Set { ty: Type::Field },
    }];
    contract.stateful_circuits = vec![StatefulCircuit {
        source: None,
        internal: false,
        name: "check_size".into(),
        parameters: vec![],
        actions: vec![
            StateAction::SetReset {
                field: "entries".into(),
                index: 0,
            },
            StateAction::Assert {
                condition: Expr::Equal {
                    left: Box::new(Expr::SetSize {
                        field: "entries".into(),
                        index: 0,
                    }),
                    right: Box::new(Expr::UnsignedLiteral {
                        value: "0".into(),
                        max: u64::MAX.to_string(),
                    }),
                },
                message: "empty after reset".into(),
            },
        ],
        result: Type::Unit,
        return_value: StateReturn::Unit,
    }];

    let rendered = render(&contract).unwrap();
    assert!(rendered.contains("crate::ledger_slots::entries.size(context)?"));
    assert!(rendered.contains(".record_size(frame)?"));
    assert!(rendered.contains("pub fn check_size<Private>"));
    assert!(rendered.contains("pub mod recorded"));
    let reset = rendered.find("record_reset(frame)?").unwrap();
    let size = rendered.find("record_size(frame)?").unwrap();
    assert!(reset < size);

    contract.stateful_circuits[0].actions[1] = StateAction::Assert {
        condition: Expr::Equal {
            left: Box::new(Expr::SetSize {
                field: "entries".into(),
                index: 1,
            }),
            right: Box::new(Expr::UnsignedLiteral {
                value: "0".into(),
                max: u64::MAX.to_string(),
            }),
        },
        message: "wrong path".into(),
    };
    assert_eq!(
        render(&contract),
        Err(RenderError::UnknownLedgerField("entries".into()))
    );
}

#[test]
fn recorded_set_assertions_preserve_boolean_queries_and_pure_field_bindings() {
    let mut contract = identity(Type::Unit, Expr::Unit);
    contract.ledger_fields = vec![LedgerField {
        source: None,
        id: "entries".into(),
        index: 0,
        path: vec![],
        declaration: LedgerFieldKind::Set { ty: Type::Field },
    }];
    contract.stateful_circuits = vec![StatefulCircuit {
        source: None,
        internal: false,
        name: "check_queries".into(),
        parameters: vec![],
        actions: vec![
            StateAction::Assert {
                condition: Expr::Equal {
                    left: Box::new(Expr::SetIsEmpty {
                        field: "entries".into(),
                        index: 0,
                    }),
                    right: Box::new(Expr::Boolean { value: true }),
                },
                message: "initially empty".into(),
            },
            StateAction::Assert {
                condition: Expr::Equal {
                    left: Box::new(Expr::Let {
                        bindings: vec![LocalBinding {
                            name: "key".into(),
                            ty: Type::Field,
                            value: Expr::FieldLiteral { value: "0".into() },
                        }],
                        body: Box::new(Expr::SetMember {
                            field: "entries".into(),
                            index: 0,
                            value: Box::new(Expr::Parameter { name: "key".into() }),
                        }),
                    }),
                    right: Box::new(Expr::Boolean { value: false }),
                },
                message: "no member".into(),
            },
        ],
        result: Type::Unit,
        return_value: StateReturn::Unit,
    }];

    let rendered = render(&contract).unwrap();
    assert!(rendered.contains("record_is_empty(frame)?"));
    assert!(rendered.contains("record_member(frame,"));
    assert!(rendered.contains("__compact_recorded_static_"));
    assert!(rendered.find("record_is_empty(frame)?") < rendered.find("record_member(frame,"));
}

#[test]
fn enum_set_locals_record_typed_keys_in_source_order() {
    let mut contract = identity(Type::Unit, Expr::Unit);
    contract.circuits.clear();
    let names = Type::Enum {
        name: "Names".into(),
        variants: vec!["bill".into(), "sally".into()],
    };
    contract.ledger_fields = vec![LedgerField {
        source: None,
        id: "c".into(),
        index: 0,
        path: vec![],
        declaration: LedgerFieldKind::Set { ty: names.clone() },
    }];
    let key = Expr::Parameter { name: "one".into() };
    contract.stateful_circuits = vec![StatefulCircuit {
        source: None,
        internal: false,
        name: "test".into(),
        parameters: vec![],
        actions: vec![StateAction::Let {
            bindings: vec![LocalBinding {
                name: "one".into(),
                ty: names.clone(),
                value: Expr::EnumVariant {
                    ty: names.clone(),
                    variant: "bill".into(),
                },
            }],
            action: Box::new(StateAction::Sequence {
                actions: vec![
                    StateAction::SetInsert {
                        field: "c".into(),
                        index: 0,
                        value: key.clone(),
                    },
                    StateAction::Assert {
                        condition: Expr::Equal {
                            left: Box::new(Expr::SetMember {
                                field: "c".into(),
                                index: 0,
                                value: Box::new(key.clone()),
                            }),
                            right: Box::new(Expr::Boolean { value: true }),
                        },
                        message: "member".into(),
                    },
                    StateAction::SetRemove {
                        field: "c".into(),
                        index: 0,
                        value: key,
                    },
                ],
            }),
        }],
        result: Type::Unit,
        return_value: StateReturn::Unit,
    }];

    let rendered = render_with_capabilities(&contract).unwrap();
    assert!(rendered.capabilities.circuits[0].recorded);
    assert!(rendered.capabilities.circuits[0].observed_call);
    let body = rendered.source.split("pub mod recorded").nth(1).unwrap();
    let bound = body.find("__compact_recorded_enum_").unwrap();
    let inserted = body.find("record_insert(frame,").unwrap();
    let observed = body.find("record_member(frame,").unwrap();
    let removed = body.find("record_remove(frame,").unwrap();
    assert!(bound < inserted && inserted < observed && observed < removed);

    // Other native enum bindings keep an explicit recording gap until their
    // source shape has its own replay and proof coverage.
    let StateAction::Let { bindings, .. } = &mut contract.stateful_circuits[0].actions[0] else {
        unreachable!();
    };
    bindings[0].value = Expr::Default { ty: names };
    let defaulted = render_with_capabilities(&contract).unwrap();
    assert!(!defaulted.capabilities.circuits[0].recorded);
    assert_eq!(
        defaulted.capabilities.circuits[0]
            .recording_unavailable
            .as_ref()
            .unwrap()
            .ir_node,
        "StateAction::Let",
    );
}

#[test]
fn closed_literal_vector_call_records_set_key_but_other_pure_calls_remain_unavailable() {
    let mut contract = identity(Type::Unit, Expr::Unit);
    let vector = Type::Vector {
        element: Box::new(Type::Field),
        length: 2,
    };
    contract.circuits = vec![PureCircuit {
        source: None,
        internal: true,
        name: "getVector".into(),
        parameters: vec![],
        result: vector.clone(),
        body: Expr::Vector {
            element: Type::Field,
            elements: vec![
                Expr::FieldLiteral { value: "2".into() },
                Expr::FieldLiteral { value: "12".into() },
            ],
        },
    }];
    contract.ledger_fields = vec![LedgerField {
        source: None,
        id: "c".into(),
        index: 0,
        path: vec![],
        declaration: LedgerFieldKind::Set { ty: vector.clone() },
    }];
    contract.stateful_circuits = vec![StatefulCircuit {
        source: None,
        internal: false,
        name: "test".into(),
        parameters: vec![],
        actions: vec![StateAction::Let {
            bindings: vec![LocalBinding {
                name: "one".into(),
                ty: vector.clone(),
                value: Expr::Call {
                    name: "getVector".into(),
                    arguments: vec![],
                },
            }],
            action: Box::new(StateAction::Sequence {
                actions: vec![
                    StateAction::SetInsert {
                        field: "c".into(),
                        index: 0,
                        value: Expr::Parameter { name: "one".into() },
                    },
                    StateAction::Assert {
                        condition: Expr::Equal {
                            left: Box::new(Expr::SetMember {
                                field: "c".into(),
                                index: 0,
                                value: Box::new(Expr::Parameter { name: "one".into() }),
                            }),
                            right: Box::new(Expr::Boolean { value: true }),
                        },
                        message: "member".into(),
                    },
                ],
            }),
        }],
        result: Type::Unit,
        return_value: StateReturn::Unit,
    }];

    let rendered = render_with_capabilities(&contract).unwrap();
    assert!(rendered.capabilities.circuits[0].recorded);
    assert!(rendered.capabilities.circuits[0].observed_call);
    let body = rendered.source.split("pub mod recorded").nth(1).unwrap();
    let pure = body.find("crate::pure_circuits::getVector()?").unwrap();
    let inserted = body.find("record_insert(frame,").unwrap();
    let observed = body.find("record_member(frame,").unwrap();
    assert!(pure < inserted && inserted < observed);

    contract.circuits[0].body = Expr::Default { ty: vector.clone() };
    let defaulted = render_with_capabilities(&contract).unwrap();
    assert!(!defaulted.capabilities.circuits[0].recorded);
    assert_eq!(
        defaulted.capabilities.circuits[0]
            .recording_unavailable
            .as_ref()
            .unwrap()
            .ir_node,
        "StateAction::Let"
    );

    contract.circuits[0].body = Expr::Vector {
        element: Type::Field,
        elements: vec![
            Expr::FieldLiteral { value: "2".into() },
            Expr::FieldLiteral { value: "12".into() },
        ],
    };
    contract.circuits[0].parameters = vec![Parameter {
        name: "unused".into(),
        ty: Type::Field,
    }];
    let StateAction::Let { bindings, .. } = &mut contract.stateful_circuits[0].actions[0] else {
        unreachable!();
    };
    bindings[0].value = Expr::Call {
        name: "getVector".into(),
        arguments: vec![Expr::FieldLiteral { value: "7".into() }],
    };
    let argument_bearing = render_with_capabilities(&contract).unwrap();
    assert!(!argument_bearing.capabilities.circuits[0].recorded);
}

#[test]
fn set_actions_require_the_declared_element_type() {
    let mut contract = Contract {
        schema_version: SCHEMA_VERSION,
        type_aliases: vec![],
        constructor: None,
        witnesses: vec![],
        ledger_fields: vec![LedgerField {
            source: None,
            id: "seen".into(),
            index: 0,
            path: vec![],
            declaration: LedgerFieldKind::Set { ty: Type::Boolean },
        }],
        circuits: vec![],
        stateful_circuits: vec![StatefulCircuit {
            source: None,
            internal: false,
            name: "add".into(),
            parameters: vec![Parameter {
                name: "value".into(),
                ty: Type::Boolean,
            }],
            actions: vec![StateAction::SetInsert {
                field: "seen".into(),
                index: 0,
                value: Expr::Parameter {
                    name: "value".into(),
                },
            }],
            result: Type::Unit,
            return_value: StateReturn::Unit,
        }],
    };
    let source = render(&contract).unwrap();
    assert!(source.contains("pub const seen: runtime::slots::SetSlot<bool>"));
    assert!(source.contains("pub struct PublicStateView<'a, D:"));
    assert!(source.contains("pub fn seen("));
    assert!(source.contains("runtime::ledger::SetView<'a, bool, D>"));
    assert!(source.contains("crate::ledger_slots::seen.inspect(self.state)"));
    assert!(source.contains("crate::ledger_slots::seen.insert(context, __compact_param_0)?"));
    assert!(
        source.contains(".record_insert(frame, __compact_param_0)?"),
        "{source}"
    );
    contract.stateful_circuits[0].parameters[0].ty = Type::Field;
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Boolean,
            actual: Type::Field,
        })
    );

    contract.stateful_circuits[0].parameters[0].ty = Type::Boolean;
    contract.stateful_circuits[0].actions = vec![StateAction::SetReset {
        field: "seen".into(),
        index: 0,
    }];
    assert!(
        render(&contract)
            .unwrap()
            .contains("crate::ledger_slots::seen.reset(context)?")
    );
    contract.stateful_circuits[0].actions.clear();
    contract.stateful_circuits[0].return_value = StateReturn::SetSize {
        field: "seen".into(),
        index: 0,
    };
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Unsigned {
                max: u64::MAX.to_string()
            },
            actual: Type::Unit,
        })
    );
    contract.stateful_circuits[0].result = Type::Unsigned {
        max: u64::MAX.to_string(),
    };
    let source = render(&contract).unwrap();
    assert!(source.contains("crate::ledger_slots::seen.size(context)?"));
    assert!(source.contains(".record_size(frame)?"), "{source}");

    // The native slot handles this type, but the recorder must not expose a
    // method until its complete typed expression path is supported.
    contract.ledger_fields[0].declaration = LedgerFieldKind::Set {
        ty: Type::Bytes { length: 32 },
    };
    contract.stateful_circuits[0].parameters[0].ty = Type::Bytes { length: 32 };
    contract.stateful_circuits[0].result = Type::Unit;
    contract.stateful_circuits[0].return_value = StateReturn::Unit;
    contract.stateful_circuits[0].actions = vec![StateAction::SetInsert {
        field: "seen".into(),
        index: 0,
        value: Expr::Parameter {
            name: "value".into(),
        },
    }];
    assert!(!render(&contract).unwrap().contains("pub mod recorded"));
}
