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

use std::io::Write;
use std::process::{Command, Stdio};

use compact_rust_backend::ir::{
    Constructor, ConstructorStep, Contract, CounterAmount, Expr, LedgerField, LedgerFieldKind,
    LocalBinding, NativeWitnessBuiltin, Parameter, PureCircuit, SCHEMA_VERSION, SourceLocation,
    StateAction, StateReturn, StatefulCircuit, StructField, Type, TypeAlias, WitnessDeclaration,
};
use compact_rust_backend::{
    RenderError, render, render_with_capabilities, render_with_proof_capabilities,
};

#[test]
fn recorded_mixed_width_guard_requires_exact_public_widths_and_pure_operands() {
    let mut contract = Contract {
        schema_version: SCHEMA_VERSION,
        type_aliases: vec![],
        constructor: None,
        witnesses: vec![],
        ledger_fields: vec![
            LedgerField {
                source: None,
                id: "count".into(),
                index: 0,
                path: vec![],
                declaration: LedgerFieldKind::Counter,
            },
            LedgerField {
                source: None,
                id: "stored".into(),
                index: 1,
                path: vec![],
                declaration: LedgerFieldKind::Cell {
                    ty: Type::Unsigned { max: "255".into() },
                },
            },
        ],
        circuits: vec![],
        stateful_circuits: vec![StatefulCircuit {
            source: None,
            internal: false,
            name: "matching".into(),
            parameters: vec![
                Parameter {
                    name: "small".into(),
                    ty: Type::Unsigned { max: "255".into() },
                },
                Parameter {
                    name: "big".into(),
                    ty: Type::Unsigned {
                        max: "4294967295".into(),
                    },
                },
            ],
            actions: vec![
                StateAction::Assert {
                    condition: Expr::Equal {
                        left: Box::new(Expr::UnsignedCast {
                            max: "4294967295".into(),
                            value: Box::new(Expr::Parameter {
                                name: "small".into(),
                            }),
                        }),
                        right: Box::new(Expr::Parameter { name: "big".into() }),
                    },
                    message: "different".into(),
                },
                StateAction::CounterIncrement {
                    field: "count".into(),
                    index: 0,
                    amount: CounterAmount::Literal { value: 1 },
                },
            ],
            result: Type::Unit,
            return_value: StateReturn::Unit,
        }],
    };
    let recorded = |contract: &Contract| {
        render_with_capabilities(contract)
            .unwrap()
            .capabilities
            .circuits[0]
            .recorded
    };
    assert!(recorded(&contract));
    assert!(
        render_with_capabilities(&contract)
            .unwrap()
            .source
            .contains("__compact_recorded_widened_0")
    );

    contract.stateful_circuits[0].parameters[0].ty = Type::Unsigned {
        max: "65535".into(),
    };
    assert!(!recorded(&contract));
    contract.stateful_circuits[0].parameters[0].ty = Type::Unsigned { max: "255".into() };
    let StateAction::Assert { condition, .. } = &mut contract.stateful_circuits[0].actions[0]
    else {
        unreachable!()
    };
    let Expr::Equal { left, .. } = condition else {
        unreachable!()
    };
    let Expr::UnsignedCast { value, .. } = left.as_mut() else {
        unreachable!()
    };
    **value = Expr::CellRead {
        field: "stored".into(),
        index: 1,
    };
    assert!(!recorded(&contract));
}

#[test]
fn recorded_product_guard_requires_closed_pure_body_and_counter_one() {
    let uint32 = Type::Unsigned {
        max: "4294967295".into(),
    };
    let product = Type::Unsigned {
        max: "17179869180".into(),
    };
    let param = |name: &str| Expr::Parameter { name: name.into() };
    let mut contract = Contract {
        schema_version: SCHEMA_VERSION,
        type_aliases: vec![],
        constructor: None,
        witnesses: vec![],
        ledger_fields: vec![LedgerField {
            source: None,
            id: "count".into(),
            index: 0,
            path: vec![],
            declaration: LedgerFieldKind::Counter,
        }],
        circuits: vec![PureCircuit {
            source: None,
            internal: false,
            name: "guard".into(),
            parameters: vec![
                Parameter {
                    name: "q".into(),
                    ty: uint32.clone(),
                },
                Parameter {
                    name: "y".into(),
                    ty: uint32.clone(),
                },
            ],
            result: Type::Unit,
            body: Expr::Sequence {
                steps: vec![Expr::Assert {
                    condition: Box::new(Expr::Let {
                        bindings: vec![LocalBinding {
                            name: "product".into(),
                            ty: product.clone(),
                            value: Expr::UnsignedMultiply {
                                max: "17179869180".into(),
                                left: Box::new(Expr::UnsignedCast {
                                    max: "17179869180".into(),
                                    value: Box::new(param("q")),
                                }),
                                right: Box::new(Expr::UnsignedLiteral {
                                    value: "4".into(),
                                    max: "17179869180".into(),
                                }),
                            },
                        }],
                        body: Box::new(Expr::Compare {
                            operator: compact_rust_backend::ir::ComparisonOperator::LessEqual,
                            left: Box::new(param("product")),
                            right: Box::new(Expr::UnsignedCast {
                                max: "17179869180".into(),
                                value: Box::new(param("y")),
                            }),
                        }),
                    }),
                    message: "product exceeds bound".into(),
                }],
                value: Box::new(Expr::Unit),
            },
        }],
        stateful_circuits: vec![StatefulCircuit {
            source: None,
            internal: false,
            name: "record".into(),
            parameters: vec![
                Parameter {
                    name: "q".into(),
                    ty: uint32.clone(),
                },
                Parameter {
                    name: "y".into(),
                    ty: uint32.clone(),
                },
            ],
            actions: vec![
                StateAction::PureCall {
                    name: "guard".into(),
                    arguments: vec![
                        Expr::Coerce {
                            value: Box::new(param("q")),
                            ty: uint32.clone(),
                        },
                        Expr::Coerce {
                            value: Box::new(param("y")),
                            ty: uint32.clone(),
                        },
                    ],
                },
                StateAction::Let {
                    bindings: vec![LocalBinding {
                        name: "one".into(),
                        ty: Type::Unsigned {
                            max: "65535".into(),
                        },
                        value: Expr::UnsignedLiteral {
                            value: "1".into(),
                            max: "65535".into(),
                        },
                    }],
                    action: Box::new(StateAction::CounterIncrement {
                        field: "count".into(),
                        index: 0,
                        amount: CounterAmount::Parameter { name: "one".into() },
                    }),
                },
            ],
            result: Type::Unit,
            return_value: StateReturn::Unit,
        }],
    };
    let recorded = |contract: &Contract| {
        render_with_capabilities(contract)
            .unwrap()
            .capabilities
            .circuits[0]
            .recorded
    };
    assert!(recorded(&contract));
    let pristine = contract.clone();

    if let Expr::Sequence { steps, .. } = &mut contract.circuits[0].body {
        steps.push(Expr::Unit);
    }
    assert!(!recorded(&contract));
    contract = pristine.clone();
    if let Expr::Sequence { steps, .. } = &mut contract.circuits[0].body
        && let Expr::Assert { condition, .. } = &mut steps[0]
        && let Expr::Let { bindings, .. } = condition.as_mut()
        && let Expr::UnsignedMultiply { right, .. } = &mut bindings[0].value
    {
        **right = Expr::UnsignedLiteral {
            value: "5".into(),
            max: "17179869180".into(),
        };
    }
    assert!(!recorded(&contract));
    contract = pristine.clone();
    if let StateAction::Let { bindings, .. } = &mut contract.stateful_circuits[0].actions[1] {
        bindings[0].value = Expr::UnsignedLiteral {
            value: "2".into(),
            max: "65535".into(),
        };
    }
    assert!(!recorded(&contract));
    contract = pristine;
    if let StateAction::PureCall { arguments, .. } = &mut contract.stateful_circuits[0].actions[0] {
        arguments.swap(0, 1);
    }
    assert!(!recorded(&contract));
}

#[test]
fn recorded_pair_hash_bindings_require_closed_typed_field_literals() {
    let pair = || Expr::Tuple {
        elements: vec![
            Expr::FieldLiteral { value: "0".into() },
            Expr::FieldLiteral { value: "1".into() },
        ],
    };
    let binding = |name: &str, ty: Type, value: Expr, index: u8| StateAction::Let {
        bindings: vec![LocalBinding {
            name: name.into(),
            ty,
            value,
        }],
        action: Box::new(StateAction::CellWrite {
            field: if index == 0 { "digest" } else { "field" }.into(),
            index,
            value: Expr::Parameter { name: name.into() },
        }),
    };
    let mut contract = Contract {
        schema_version: SCHEMA_VERSION,
        type_aliases: vec![],
        constructor: None,
        witnesses: vec![],
        ledger_fields: vec![
            LedgerField {
                id: "digest".into(),
                index: 0,
                path: vec![],
                source: None,
                declaration: LedgerFieldKind::Cell {
                    ty: Type::Bytes { length: 32 },
                },
            },
            LedgerField {
                id: "field".into(),
                index: 1,
                path: vec![],
                source: None,
                declaration: LedgerFieldKind::Cell { ty: Type::Field },
            },
        ],
        circuits: vec![],
        stateful_circuits: vec![
            StatefulCircuit {
                source: None,
                internal: false,
                name: "persistent".into(),
                parameters: vec![],
                result: Type::Unit,
                return_value: StateReturn::Unit,
                actions: vec![binding(
                    "hash",
                    Type::Bytes { length: 32 },
                    Expr::PersistentHash {
                        value: Box::new(pair()),
                    },
                    0,
                )],
            },
            StatefulCircuit {
                source: None,
                internal: false,
                name: "transient".into(),
                parameters: vec![],
                result: Type::Unit,
                return_value: StateReturn::Unit,
                actions: vec![binding(
                    "hash",
                    Type::Field,
                    Expr::TransientHash {
                        value: Box::new(pair()),
                    },
                    1,
                )],
            },
        ],
    };
    let rendered = render_with_capabilities(&contract).unwrap();
    assert!(rendered.capabilities.circuits.iter().all(|c| c.recorded));
    let recorded = rendered.source.split("pub mod recorded").nth(1).unwrap();
    let persistent = recorded.split("pub fn persistent<Private>").nth(1).unwrap();
    let transient = recorded.split("pub fn transient<Private>").nth(1).unwrap();
    assert!(
        persistent.find("runtime::persistent_hash(").unwrap()
            < persistent.find(".record_write(").unwrap()
    );
    assert!(
        transient.find("runtime::transient_hash(").unwrap()
            < transient.find(".record_write(").unwrap()
    );

    let StateAction::Let { bindings, .. } = &mut contract.stateful_circuits[0].actions[0] else {
        unreachable!()
    };
    bindings[0].value = Expr::PersistentHash {
        value: Box::new(Expr::Tuple {
            elements: vec![
                Expr::CellRead {
                    field: "field".into(),
                    index: 1,
                },
                Expr::FieldLiteral { value: "1".into() },
            ],
        }),
    };
    let rendered = render_with_capabilities(&contract).unwrap();
    assert!(!rendered.capabilities.circuits[0].recorded);
    assert!(rendered.capabilities.circuits[1].recorded);

    let StateAction::Let { bindings, .. } = &mut contract.stateful_circuits[1].actions[0] else {
        unreachable!()
    };
    bindings[0].value = Expr::TransientHash {
        value: Box::new(Expr::Tuple {
            elements: vec![Expr::FieldLiteral { value: "0".into() }],
        }),
    };
    let rendered = render_with_capabilities(&contract).unwrap();
    assert!(rendered.capabilities.circuits.iter().all(|c| !c.recorded));
}

#[test]
fn opaque_string_map_recording_requires_closed_field_lookup_sequence() {
    let mut contract: Contract =
        serde_json::from_str(include_str!("opaque-string-map-schema12-ir.json")).unwrap();
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
        vec![("put".into(), true), ("ensure".into(), true)]
    );

    let original = contract.stateful_circuits[0].actions[0].clone();
    contract.stateful_circuits[0].actions.push(original.clone());
    assert_eq!(
        statuses(&contract),
        vec![("put".into(), false), ("ensure".into(), true)]
    );
    contract.stateful_circuits[0].actions.truncate(1);
    let StateAction::MapInsert { value, .. } = &mut contract.stateful_circuits[0].actions[0] else {
        unreachable!()
    };
    *value = Expr::FieldLiteral { value: "7".into() };
    assert_eq!(
        statuses(&contract),
        vec![("put".into(), false), ("ensure".into(), true)]
    );
    contract.stateful_circuits[0].actions[0] = original;

    let StateReturn::Expression {
        value: Expr::Let { body, .. },
    } = &mut contract.stateful_circuits[1].return_value
    else {
        unreachable!()
    };
    **body = Expr::FieldLiteral { value: "7".into() };
    assert_eq!(
        statuses(&contract),
        vec![("put".into(), true), ("ensure".into(), false)]
    );
}

#[test]
fn asset_removal_recording_requires_scoped_opaque_key_and_exact_order() {
    let mut contract: Contract =
        serde_json::from_str(include_str!("asset-removal-schema12-ir.json")).unwrap();
    contract.schema_version = SCHEMA_VERSION;
    let recorded = |contract: &Contract| {
        let rendered = render_with_capabilities(contract).unwrap();
        let capability = rendered
            .capabilities
            .circuits
            .iter()
            .find(|capability| capability.name == "removeRecord")
            .unwrap();
        (capability.recorded, rendered.source)
    };
    let (available, source) = recorded(&contract);
    assert!(available);
    assert!(source.contains("pub fn removeRecord<Private"));
    assert!(source.contains("record_remove(frame"));

    let mut swapped = contract.clone();
    let circuit = swapped
        .stateful_circuits
        .iter_mut()
        .find(|circuit| circuit.name == "removeRecord")
        .unwrap();
    let StateAction::Let { action, .. } = &mut circuit.actions[0] else {
        unreachable!()
    };
    let StateAction::Sequence { actions } = action.as_mut() else {
        unreachable!()
    };
    actions.swap(3, 4);
    assert!(!recorded(&swapped).0, "mutation order is part of the gate");

    let circuit = contract
        .stateful_circuits
        .iter_mut()
        .find(|circuit| circuit.name == "removeRecord")
        .unwrap();
    let StateAction::Let { action, .. } = &mut circuit.actions[0] else {
        unreachable!()
    };
    let StateAction::Sequence { actions } = action.as_mut() else {
        unreachable!()
    };
    let StateAction::MapRemove { key, .. } = &mut actions[3] else {
        unreachable!()
    };
    *key = Expr::Parameter {
        name: "recordId".into(),
    };
    assert!(!recorded(&contract).0, "Map removal must use the bound key");
}

#[test]
fn opaque_string_set_recording_requires_closed_typed_operations() {
    let mut contract: Contract =
        serde_json::from_str(include_str!("opaque-string-set-schema12-ir.json")).unwrap();
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
fn welcome_organizer_recording_requires_the_closed_witness_hash_guard() {
    let mut contract: Contract =
        serde_json::from_str(include_str!("welcome-organizer-schema12-ir.json")).unwrap();
    contract.schema_version = SCHEMA_VERSION;
    let statuses = |contract: &Contract| {
        let rendered = render_with_capabilities(contract).unwrap();
        rendered
            .capabilities
            .circuits
            .into_iter()
            .map(|capability| {
                (
                    capability.name,
                    capability.recorded,
                    capability.recording_unavailable.map(|gap| gap.path),
                )
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(
        statuses(&contract),
        vec![
            ("add_participant".to_owned(), true, None),
            ("add_organizer".to_owned(), true, None),
            ("check_in".to_owned(), true, None),
        ]
    );

    let hash = contract
        .circuits
        .iter_mut()
        .find(|circuit| circuit.name == "public_key")
        .unwrap();
    hash.body = Expr::Parameter { name: "sk".into() };
    assert_eq!(
        statuses(&contract),
        vec![
            (
                "add_participant".to_owned(),
                false,
                Some("actions[0]".to_owned())
            ),
            (
                "add_organizer".to_owned(),
                false,
                Some("actions[0]".to_owned())
            ),
            ("check_in".to_owned(), true, None),
        ]
    );

    let hash = contract
        .circuits
        .iter_mut()
        .find(|circuit| circuit.name == "public_key")
        .unwrap();
    let original: Contract =
        serde_json::from_str(include_str!("welcome-organizer-schema12-ir.json")).unwrap();
    hash.body = original
        .circuits
        .iter()
        .find(|circuit| circuit.name == "public_key")
        .unwrap()
        .body
        .clone();
    let helper = contract
        .stateful_circuits
        .iter_mut()
        .find(|circuit| circuit.name == "local_sk_or_error")
        .unwrap();
    let StateReturn::Expression {
        value: Expr::Let { body, .. },
    } = &mut helper.return_value
    else {
        unreachable!()
    };
    let Expr::Sequence { steps, .. } = body.as_mut() else {
        unreachable!()
    };
    steps.push(Expr::Assert {
        condition: Box::new(Expr::Boolean { value: true }),
        message: "extra assertion".into(),
    });
    assert_eq!(
        statuses(&contract),
        vec![
            (
                "add_participant".to_owned(),
                false,
                Some("actions[0]".to_owned())
            ),
            (
                "add_organizer".to_owned(),
                false,
                Some("actions[0]".to_owned())
            ),
            ("check_in".to_owned(), true, None),
        ]
    );
}

fn identity(result: Type, body: Expr) -> Contract {
    Contract {
        schema_version: 12,
        type_aliases: vec![],
        constructor: None,
        witnesses: vec![],
        ledger_fields: vec![],
        circuits: vec![PureCircuit {
            source: None,
            internal: false,
            name: "identity".into(),
            parameters: vec![Parameter {
                name: "value".into(),
                ty: Type::Field,
            }],
            result,
            body,
        }],
        stateful_circuits: vec![],
    }
}

#[test]
fn recorded_distinct_struct_constructor_calls_require_exact_projection_and_closed_body() {
    let fixture = || -> Contract {
        serde_json::from_str(include_str!(
            "../fixtures/recorded-struct-constructor-cells.json"
        ))
        .unwrap()
    };
    let contract = fixture();
    let rendered = render_with_capabilities(&contract).unwrap();
    assert!(rendered.capabilities.circuits.iter().all(|c| c.recorded));
    assert!(rendered.source.contains("crate::types::RecCompact1"));
    assert!(rendered.source.contains("crate::types::Rec"));
    assert!(rendered.source.contains("pure_circuits::makeAlpha"));
    assert!(rendered.source.contains("pure_circuits::makeBeta"));

    let mut wrong_identity = fixture();
    let StateAction::Let { bindings, .. } = &mut wrong_identity.stateful_circuits[0].actions[0]
    else {
        unreachable!()
    };
    let Expr::Call { name, .. } = &mut bindings[0].value else {
        unreachable!()
    };
    *name = "makeBeta".into();
    // The typed renderer rejects a cross-module result identity before it
    // can produce either native or recorded Rust.
    assert!(matches!(
        render_with_capabilities(&wrong_identity),
        Err(RenderError::Located { .. })
    ));

    let mut wrong_write = fixture();
    let StateAction::Let { action, .. } = &mut wrong_write.stateful_circuits[0].actions[0] else {
        unreachable!()
    };
    let StateAction::Let { action, .. } = action.as_mut() else {
        unreachable!()
    };
    let StateAction::CellWrite { value, .. } = action.as_mut() else {
        unreachable!()
    };
    *value = Expr::Parameter { name: "x".into() };
    assert!(
        !render_with_capabilities(&wrong_write)
            .unwrap()
            .capabilities
            .circuits[0]
            .recorded
    );

    let mut effectful_helper = fixture();
    let previous = effectful_helper.circuits[0].body.clone();
    effectful_helper.circuits[0].body = Expr::Sequence {
        steps: vec![Expr::Assert {
            condition: Box::new(Expr::Boolean { value: true }),
            message: "side condition".into(),
        }],
        value: Box::new(previous),
    };
    assert!(
        !render_with_capabilities(&effectful_helper)
            .unwrap()
            .capabilities
            .circuits[0]
            .recorded
    );
}

#[test]
fn recorded_nested_uint64_projection_rejects_other_widths_and_effects() {
    let mut contract: Contract = serde_json::from_str(include_str!(
        "../fixtures/recorded-nested-uint64-cell-counter.json"
    ))
    .unwrap();
    let rendered = render_with_capabilities(&contract).unwrap();
    assert!(rendered.capabilities.circuits[0].recorded);
    assert!(
        rendered
            .source
            .contains("__compact_recorded_nested_uint64_")
    );

    let closed = contract.clone();
    let StateAction::Let { action, .. } = &mut contract.stateful_circuits[0].actions[0] else {
        unreachable!()
    };
    let StateAction::Sequence { actions } = action.as_mut() else {
        unreachable!()
    };
    let StateAction::Let { bindings, .. } = &mut actions[0] else {
        unreachable!()
    };
    bindings[0].ty = Type::Unsigned { max: "255".into() };
    let Expr::UnsignedCast { max, .. } = &mut bindings[0].value else {
        unreachable!()
    };
    *max = "255".into();
    let LedgerFieldKind::Cell { ty } = &mut contract.ledger_fields[0].declaration else {
        unreachable!()
    };
    *ty = Type::Unsigned { max: "255".into() };
    assert!(
        !render_with_capabilities(&contract)
            .unwrap()
            .capabilities
            .circuits[0]
            .recorded
    );

    let mut contract = closed.clone();
    let StateAction::Let { action, .. } = &mut contract.stateful_circuits[0].actions[0] else {
        unreachable!()
    };
    let StateAction::Sequence { actions } = action.as_mut() else {
        unreachable!()
    };
    let StateAction::Let { bindings, .. } = &mut actions[1] else {
        unreachable!()
    };
    bindings[0].value = Expr::UnsignedLiteral {
        value: "2".into(),
        max: "65535".into(),
    };
    assert!(
        !render_with_capabilities(&contract)
            .unwrap()
            .capabilities
            .circuits[0]
            .recorded
    );

    let mut contract = closed;
    contract.witnesses.push(WitnessDeclaration {
        source: None,
        name: "dynamicFlag".into(),
        parameters: vec![],
        result: Type::Boolean,
    });
    let StateAction::Let { bindings, .. } = &mut contract.stateful_circuits[0].actions[0] else {
        unreachable!()
    };
    let Expr::If { condition, .. } = &mut bindings[0].value else {
        unreachable!()
    };
    **condition = Expr::WitnessCall {
        name: "dynamicFlag".into(),
        arguments: vec![],
    };
    assert!(
        !render_with_capabilities(&contract)
            .unwrap()
            .capabilities
            .circuits[0]
            .recorded
    );
}

#[test]
fn recorded_conditional_field_pair_rejects_unrecorded_predicates_and_arms() {
    let mut contract: Contract = serde_json::from_str(include_str!(
        "../fixtures/recorded-closed-conditional-field-pair.json"
    ))
    .unwrap();
    let rendered = render_with_capabilities(&contract).unwrap();
    assert!(rendered.capabilities.circuits[0].recorded);
    assert!(
        rendered
            .source
            .contains("__compact_recorded_conditional_pair_")
    );

    let closed = contract.clone();
    let StateAction::Let { action, .. } = &mut contract.stateful_circuits[0].actions[0] else {
        unreachable!()
    };
    let StateAction::Let { bindings, .. } = action.as_mut() else {
        unreachable!()
    };
    let Expr::Vector { elements, .. } = &mut bindings[0].value else {
        unreachable!()
    };
    let Expr::If { condition, .. } = &mut elements[0] else {
        unreachable!()
    };
    **condition = Expr::CellRead {
        field: "flag".into(),
        index: 0,
    };
    assert!(render_with_capabilities(&contract).is_err());

    let mut contract = closed;
    contract.witnesses.push(WitnessDeclaration {
        source: None,
        name: "dynamicField".into(),
        parameters: vec![],
        result: Type::Field,
    });
    let StateAction::Let { action, .. } = &mut contract.stateful_circuits[0].actions[0] else {
        unreachable!()
    };
    let StateAction::Let { bindings, .. } = action.as_mut() else {
        unreachable!()
    };
    let Expr::Vector { elements, .. } = &mut bindings[0].value else {
        unreachable!()
    };
    let Expr::If { then, .. } = &mut elements[0] else {
        unreachable!()
    };
    **then = Expr::Coerce {
        value: Box::new(Expr::WitnessCall {
            name: "dynamicField".into(),
            arguments: vec![],
        }),
        ty: Type::Field,
    };
    assert!(render_with_capabilities(&contract).is_err());
}

#[test]
fn recorded_annotated_uint8_rejects_unrecorded_predicates_and_arms() {
    let mut contract: Contract = serde_json::from_str(include_str!(
        "../fixtures/recorded-annotated-uint8-conditional.json"
    ))
    .unwrap();
    let rendered = render_with_capabilities(&contract).unwrap();
    assert!(rendered.capabilities.circuits[0].recorded);
    assert!(
        rendered
            .source
            .contains("__compact_recorded_annotated_uint8_")
    );
    assert!(rendered.source.contains("runtime::Field::from"));

    let closed = contract.clone();
    let StateAction::Let { bindings, .. } = &mut contract.stateful_circuits[0].actions[0] else {
        unreachable!()
    };
    let Expr::UnsignedCast { value, .. } = &mut bindings[0].value else {
        unreachable!()
    };
    let Expr::If { condition, .. } = value.as_mut() else {
        unreachable!()
    };
    **condition = Expr::CellRead {
        field: "flag".into(),
        index: 0,
    };
    assert!(
        !render_with_capabilities(&contract)
            .unwrap()
            .capabilities
            .circuits[0]
            .recorded
    );

    let mut contract = closed;
    contract.witnesses.push(WitnessDeclaration {
        source: None,
        name: "dynamicArm".into(),
        parameters: vec![],
        result: Type::Unsigned { max: "2".into() },
    });
    let StateAction::Let { bindings, .. } = &mut contract.stateful_circuits[0].actions[0] else {
        unreachable!()
    };
    let Expr::UnsignedCast { value, .. } = &mut bindings[0].value else {
        unreachable!()
    };
    let Expr::If { then, .. } = value.as_mut() else {
        unreachable!()
    };
    **then = Expr::Coerce {
        value: Box::new(Expr::WitnessCall {
            name: "dynamicArm".into(),
            arguments: vec![],
        }),
        ty: Type::Unsigned { max: "2".into() },
    };
    assert!(
        !render_with_capabilities(&contract)
            .unwrap()
            .capabilities
            .circuits[0]
            .recorded
    );
}

#[test]
fn recorded_closed_curve_argument_rejects_effectful_predicates_and_arms() {
    let mut contract: Contract = serde_json::from_str(include_str!(
        "../fixtures/recorded-closed-curve-argument.json"
    ))
    .unwrap();
    let rendered = render_with_capabilities(&contract).unwrap();
    assert!(rendered.capabilities.circuits[0].recorded);
    assert!(rendered.source.contains("runtime::hash_to_curve"));
    assert!(rendered.source.contains("runtime::jubjub_point_x"));

    let closed = contract.clone();
    let StateAction::Let { bindings, .. } = &mut contract.stateful_circuits[0].actions[0] else {
        unreachable!()
    };
    let Expr::HashToCurve { value } = &mut bindings[0].value else {
        unreachable!()
    };
    let Expr::FieldCast { value } = value.as_mut() else {
        unreachable!()
    };
    let Expr::If { condition, .. } = value.as_mut() else {
        unreachable!()
    };
    **condition = Expr::CellRead {
        field: "flag".into(),
        index: 0,
    };
    assert!(
        !render_with_capabilities(&contract)
            .unwrap()
            .capabilities
            .circuits[0]
            .recorded
    );

    let mut contract = closed;
    contract.witnesses.push(WitnessDeclaration {
        source: None,
        name: "dynamicArm".into(),
        parameters: vec![],
        result: Type::Unsigned { max: "2".into() },
    });
    let StateAction::Let { bindings, .. } = &mut contract.stateful_circuits[0].actions[0] else {
        unreachable!()
    };
    let Expr::HashToCurve { value } = &mut bindings[0].value else {
        unreachable!()
    };
    let Expr::FieldCast { value } = value.as_mut() else {
        unreachable!()
    };
    let Expr::If { then, .. } = value.as_mut() else {
        unreachable!()
    };
    **then = Expr::Coerce {
        value: Box::new(Expr::WitnessCall {
            name: "dynamicArm".into(),
            arguments: vec![],
        }),
        ty: Type::Unsigned { max: "2".into() },
    };
    assert!(
        !render_with_capabilities(&contract)
            .unwrap()
            .capabilities
            .circuits[0]
            .recorded
    );
}

#[test]
fn typed_field_helper_hash_then_cell_read_preserves_order_and_rejects_effects() {
    let pair = Type::Vector {
        length: 2,
        element: Box::new(Type::Field),
    };
    let mut contract = Contract {
        schema_version: SCHEMA_VERSION,
        type_aliases: vec![],
        constructor: None,
        witnesses: vec![],
        ledger_fields: vec![
            LedgerField {
                source: None,
                id: "source".into(),
                index: 0,
                path: vec![],
                declaration: LedgerFieldKind::Cell { ty: Type::Field },
            },
            LedgerField {
                source: None,
                id: "target".into(),
                index: 1,
                path: vec![],
                declaration: LedgerFieldKind::Cell { ty: Type::Field },
            },
        ],
        circuits: vec![PureCircuit {
            source: None,
            internal: false,
            name: "hashPair".into(),
            parameters: vec![Parameter {
                name: "pair".into(),
                ty: pair.clone(),
            }],
            result: Type::Field,
            body: Expr::TransientHash {
                value: Box::new(Expr::Parameter {
                    name: "pair".into(),
                }),
            },
        }],
        stateful_circuits: vec![
            StatefulCircuit {
                source: None,
                internal: true,
                name: "helper".into(),
                parameters: vec![Parameter {
                    name: "pair".into(),
                    ty: pair.clone(),
                }],
                result: Type::Field,
                return_value: StateReturn::Expression {
                    value: Expr::Add {
                        left: Box::new(Expr::Call {
                            name: "hashPair".into(),
                            arguments: vec![Expr::Coerce {
                                value: Box::new(Expr::Parameter {
                                    name: "pair".into(),
                                }),
                                ty: pair.clone(),
                            }],
                        }),
                        right: Box::new(Expr::CellRead {
                            field: "source".into(),
                            index: 0,
                        }),
                    },
                },
                actions: vec![],
            },
            StatefulCircuit {
                source: None,
                internal: false,
                name: "save".into(),
                parameters: vec![],
                result: Type::Unit,
                return_value: StateReturn::Unit,
                actions: vec![StateAction::Let {
                    bindings: vec![LocalBinding {
                        name: "value".into(),
                        ty: Type::Field,
                        value: Expr::Call {
                            name: "helper".into(),
                            arguments: vec![Expr::Coerce {
                                value: Box::new(Expr::Tuple {
                                    elements: vec![
                                        Expr::FieldLiteral { value: "0".into() },
                                        Expr::FieldLiteral { value: "1".into() },
                                    ],
                                }),
                                ty: pair,
                            }],
                        },
                    }],
                    action: Box::new(StateAction::CellWrite {
                        field: "target".into(),
                        index: 1,
                        value: Expr::Parameter {
                            name: "value".into(),
                        },
                    }),
                }],
            },
        ],
    };
    let rendered = render_with_capabilities(&contract).unwrap();
    assert!(rendered.capabilities.circuits[0].recorded);
    assert!(rendered.capabilities.circuits[0].observed_call);
    let recording = rendered.source.split("pub mod recorded").nth(1).unwrap();
    let pure = recording.find("crate::pure_circuits::hashPair(").unwrap();
    let read = recording.find("record_read(frame)").unwrap();
    let write = recording.find("record_write(frame").unwrap();
    assert!(recording.contains("crate::ledger_slots::source"));
    assert!(recording.contains("crate::ledger_slots::target"));
    assert!(pure < read && read < write);

    contract.stateful_circuits[0]
        .actions
        .push(StateAction::CellWrite {
            field: "source".into(),
            index: 0,
            value: Expr::FieldLiteral { value: "2".into() },
        });
    assert!(
        !render_with_capabilities(&contract)
            .unwrap()
            .capabilities
            .circuits[0]
            .recorded
    );
    contract.stateful_circuits[0].actions.clear();
    contract.circuits[0].body = Expr::FieldLiteral { value: "9".into() };
    assert!(
        !render_with_capabilities(&contract)
            .unwrap()
            .capabilities
            .circuits[0]
            .recorded
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
        schema_version: 12,
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
fn recorded_nested_uint4_accepts_only_closed_typed_arms() {
    let mut contract: Contract =
        serde_json::from_str(include_str!("../fixtures/recorded-nested-uint4.json")).unwrap();
    let rendered = render_with_capabilities(&contract).unwrap();
    assert!(rendered.capabilities.circuits[0].recorded);
    assert!(rendered.source.contains("BoundedUint::<4>::new"));
    assert!(rendered.source.contains("__compact_recorded_nested_field_"));

    let closed = contract.clone();

    let StateAction::Let { bindings, .. } = &mut contract.stateful_circuits[0].actions[0] else {
        unreachable!()
    };
    let Expr::If { then, .. } = &mut bindings[0].value else {
        unreachable!()
    };
    let Expr::Coerce { value, .. } = then.as_mut() else {
        unreachable!()
    };
    let Expr::UnsignedCast { value, .. } = value.as_mut() else {
        unreachable!()
    };
    let Expr::If { then, .. } = value.as_mut() else {
        unreachable!()
    };
    **then = Expr::Coerce {
        value: Box::new(Expr::UnsignedAdd {
            max: "2".into(),
            left: Box::new(Expr::UnsignedLiteral {
                value: "1".into(),
                max: "2".into(),
            }),
            right: Box::new(Expr::UnsignedLiteral {
                value: "1".into(),
                max: "2".into(),
            }),
        }),
        ty: Type::Unsigned { max: "2".into() },
    };
    assert!(
        !render_with_capabilities(&contract)
            .unwrap()
            .capabilities
            .circuits[0]
            .recorded
    );

    let mut contract = closed;
    contract.witnesses.push(WitnessDeclaration {
        source: None,
        name: "dynamicArm".into(),
        parameters: vec![],
        result: Type::Unsigned { max: "2".into() },
    });
    let StateAction::Let { bindings, .. } = &mut contract.stateful_circuits[0].actions[0] else {
        unreachable!()
    };
    let Expr::If { then, .. } = &mut bindings[0].value else {
        unreachable!()
    };
    let Expr::Coerce { value, .. } = then.as_mut() else {
        unreachable!()
    };
    let Expr::UnsignedCast { value, .. } = value.as_mut() else {
        unreachable!()
    };
    let Expr::If { then, .. } = value.as_mut() else {
        unreachable!()
    };
    **then = Expr::Coerce {
        value: Box::new(Expr::WitnessCall {
            name: "dynamicArm".into(),
            arguments: vec![],
        }),
        ty: Type::Unsigned { max: "2".into() },
    };
    assert!(
        !render_with_capabilities(&contract)
            .unwrap()
            .capabilities
            .circuits[0]
            .recorded
    );
}

#[test]
fn closed_unsigned_ternary_comparison_records_only_matching_literal_arms() {
    let uint = Type::Unsigned { max: "255".into() };
    let narrow = Type::Unsigned { max: "1".into() };
    let arm = |value: &str| Expr::Coerce {
        value: Box::new(Expr::UnsignedLiteral {
            value: value.into(),
            max: "1".into(),
        }),
        ty: narrow.clone(),
    };
    let selected = Expr::UnsignedCast {
        max: "255".into(),
        value: Box::new(Expr::If {
            condition: Box::new(Expr::Parameter { name: "c".into() }),
            then: Box::new(arm("1")),
            otherwise: Box::new(arm("0")),
        }),
    };
    let mut contract = identity(Type::Unit, Expr::Unit);
    contract.stateful_circuits = vec![StatefulCircuit {
        source: None,
        internal: false,
        name: "test".into(),
        parameters: vec![
            Parameter {
                name: "c".into(),
                ty: Type::Boolean,
            },
            Parameter {
                name: "x".into(),
                ty: uint,
            },
            Parameter {
                name: "y".into(),
                ty: narrow.clone(),
            },
        ],
        actions: vec![StateAction::Assert {
            condition: Expr::Equal {
                left: Box::new(Expr::Parameter { name: "x".into() }),
                right: Box::new(selected),
            },
            message: "comparison".into(),
        }],
        result: Type::Unit,
        return_value: StateReturn::Unit,
    }];
    let rendered = render_with_capabilities(&contract).unwrap();
    assert!(rendered.capabilities.circuits[0].recorded);

    let StateAction::Assert { condition, .. } = &mut contract.stateful_circuits[0].actions[0]
    else {
        unreachable!()
    };
    let Expr::Equal { right, .. } = condition else {
        unreachable!()
    };
    let Expr::UnsignedCast { value, .. } = right.as_mut() else {
        unreachable!()
    };
    let Expr::If { then, .. } = value.as_mut() else {
        unreachable!()
    };
    **then = Expr::Coerce {
        value: Box::new(Expr::Parameter { name: "y".into() }),
        ty: narrow,
    };
    let rendered = render_with_capabilities(&contract).unwrap();
    assert!(!rendered.capabilities.circuits[0].recorded);
}

#[test]
fn closed_ternary_struct_member_records_only_literal_field_values() {
    let box_ty = Type::Struct {
        name: "Box".into(),
        fields: vec![StructField {
            name: "f".into(),
            ty: Type::Field,
        }],
    };
    let uint = Type::Unsigned { max: "2".into() };
    let arm = |value: &str| Expr::Coerce {
        value: Box::new(Expr::UnsignedLiteral {
            value: value.into(),
            max: "2".into(),
        }),
        ty: uint.clone(),
    };
    let mut contract = identity(Type::Unit, Expr::Unit);
    contract.ledger_fields = vec![LedgerField {
        source: None,
        id: "out".into(),
        index: 0,
        path: vec![],
        declaration: LedgerFieldKind::Cell { ty: Type::Field },
    }];
    contract.stateful_circuits = vec![StatefulCircuit {
        source: None,
        internal: false,
        name: "test".into(),
        parameters: vec![
            Parameter {
                name: "c".into(),
                ty: Type::Boolean,
            },
            Parameter {
                name: "dynamic".into(),
                ty: uint.clone(),
            },
        ],
        actions: vec![StateAction::Let {
            bindings: vec![LocalBinding {
                name: "box".into(),
                ty: box_ty.clone(),
                value: Expr::StructLiteral {
                    ty: box_ty,
                    fields: vec![Expr::FieldCast {
                        value: Box::new(Expr::If {
                            condition: Box::new(Expr::Parameter { name: "c".into() }),
                            then: Box::new(arm("1")),
                            otherwise: Box::new(arm("2")),
                        }),
                    }],
                },
            }],
            action: Box::new(StateAction::Let {
                bindings: vec![LocalBinding {
                    name: "member".into(),
                    ty: Type::Field,
                    value: Expr::StructField {
                        value: Box::new(Expr::Parameter { name: "box".into() }),
                        field: "f".into(),
                        index: 0,
                    },
                }],
                action: Box::new(StateAction::CellWrite {
                    field: "out".into(),
                    index: 0,
                    value: Expr::Parameter {
                        name: "member".into(),
                    },
                }),
            }),
        }],
        result: Type::Unit,
        return_value: StateReturn::Unit,
    }];
    let rendered = render_with_capabilities(&contract).unwrap();
    assert!(rendered.capabilities.circuits[0].recorded);
    assert!(rendered.source.contains("__compact_recorded_struct_"));

    let StateAction::Let { bindings, .. } = &mut contract.stateful_circuits[0].actions[0] else {
        unreachable!()
    };
    let Expr::StructLiteral { fields, .. } = &mut bindings[0].value else {
        unreachable!()
    };
    let Expr::FieldCast { value } = &mut fields[0] else {
        unreachable!()
    };
    let Expr::If { then, .. } = value.as_mut() else {
        unreachable!()
    };
    **then = Expr::Coerce {
        value: Box::new(Expr::Parameter {
            name: "dynamic".into(),
        }),
        ty: uint,
    };
    let rendered = render_with_capabilities(&contract).unwrap();
    assert!(!rendered.capabilities.circuits[0].recorded);
}

#[test]
fn native_own_public_key_is_a_private_effect_without_a_user_witness() {
    let contract = Contract {
        schema_version: 12,
        type_aliases: vec![],
        constructor: None,
        witnesses: vec![],
        ledger_fields: vec![],
        circuits: vec![],
        stateful_circuits: vec![StatefulCircuit {
            source: None,
            internal: false,
            name: "test1".into(),
            parameters: vec![],
            result: Type::Unit,
            return_value: StateReturn::Unit,
            actions: vec![StateAction::NativeWitnessCall {
                builtin: NativeWitnessBuiltin::OwnPublicKey,
            }],
        }],
    };
    let source = render(&contract).unwrap();
    assert!(source.contains("own_coin_public_key()?"));
    assert!(source.contains("private_transcript_outputs"));
    assert!(source.contains("runtime::fab::AlignedValue::from"));
    assert!(source.contains("pub fn test1<Private>"));
    assert!(!source.contains("Witnesses::ownPublicKey"));
    assert!(!source.contains("pub fn test1<Private, W:"));
}

#[test]
fn native_private_output_flows_through_a_stateful_caller() {
    let helper = StatefulCircuit {
        source: None,
        internal: true,
        name: "helper".into(),
        parameters: vec![],
        result: Type::Unit,
        return_value: StateReturn::Unit,
        actions: vec![StateAction::NativeWitnessCall {
            builtin: NativeWitnessBuiltin::OwnPublicKey,
        }],
    };
    let caller = StatefulCircuit {
        source: None,
        internal: false,
        name: "caller".into(),
        parameters: vec![],
        result: Type::Unit,
        return_value: StateReturn::Unit,
        actions: vec![StateAction::CircuitCall {
            name: "helper".into(),
            arguments: vec![],
        }],
    };
    let contract = Contract {
        schema_version: 12,
        type_aliases: vec![],
        constructor: None,
        witnesses: vec![],
        ledger_fields: vec![],
        circuits: vec![],
        stateful_circuits: vec![helper, caller],
    };
    let source = render(&contract).unwrap();
    assert!(source.contains("call_step.private_transcript_outputs"));
    assert!(source.contains("pub fn caller<Private>"));
    assert!(!source.contains("pub fn caller<Private, W:"));
}

#[test]
fn native_public_key_expression_retains_value_and_private_effect() {
    let native = Expr::NativeWitnessCall {
        builtin: NativeWitnessBuiltin::OwnPublicKey,
    };
    let contract = Contract {
        schema_version: 12,
        type_aliases: vec![],
        constructor: None,
        witnesses: vec![],
        ledger_fields: vec![],
        circuits: vec![],
        stateful_circuits: vec![
            StatefulCircuit {
                source: None,
                internal: false,
                name: "key".into(),
                parameters: vec![],
                actions: vec![],
                result: NativeWitnessBuiltin::OwnPublicKey.result_type(),
                return_value: StateReturn::Expression {
                    value: native.clone(),
                },
            },
            StatefulCircuit {
                source: None,
                internal: false,
                name: "key_bytes".into(),
                parameters: vec![],
                actions: vec![],
                result: Type::Bytes { length: 32 },
                return_value: StateReturn::Expression {
                    value: Expr::StructField {
                        value: Box::new(native),
                        field: "bytes".into(),
                        index: 0,
                    },
                },
            },
        ],
    };
    let source = render(&contract).unwrap();
    assert_eq!(source.matches("context.own_coin_public_key()?").count(), 2);
    assert!(source.contains("pub struct ZswapCoinPublicKey"));
    assert!(source.contains("pub fn key<Private>"));
    assert!(source.contains("pub fn key_bytes<Private>"));
    assert!(!source.contains("pub fn key<Private, W:"));
    assert!(!source.contains("pub fn key_bytes<Private, W:"));
}

#[test]
fn pure_unit_circuits_keep_effects_in_statement_order() {
    let mut contract = identity(Type::Unit, Expr::Unit);
    let literal = render(&contract).unwrap();
    assert!(literal.contains("pub fn identity(value: runtime::Field) -> Result<(), runtime::CompactError> {\n        Ok(())"));

    contract.circuits[0].body = Expr::Sequence {
        steps: vec![Expr::Assert {
            condition: Box::new(Expr::Boolean { value: true }),
            message: "first".into(),
        }],
        value: Box::new(Expr::Unit),
    };
    let assertions = render(&contract).unwrap();
    assert!(assertions.contains("if !(true)"));
    assert!(assertions.contains("AssertionFailed(\"first\".to_owned())"));
    assert!(!assertions.contains("Ok({"));

    contract.circuits.push(PureCircuit {
        source: None,
        internal: false,
        name: "target".into(),
        parameters: vec![Parameter {
            name: "value".into(),
            ty: Type::Field,
        }],
        result: Type::Unit,
        body: Expr::Unit,
    });
    contract.circuits[0].body = Expr::If {
        condition: Box::new(Expr::Boolean { value: true }),
        then: Box::new(Expr::Call {
            name: "target".into(),
            arguments: vec![Expr::Parameter {
                name: "value".into(),
            }],
        }),
        otherwise: Box::new(Expr::Unit),
    };
    let conditional_call = render(&contract).unwrap();
    assert!(conditional_call.contains("if true"));
    assert!(conditional_call.contains("crate::pure_circuits::target(value)?;"));
    assert!(conditional_call.contains("Ok(())"));
    assert!(!conditional_call.contains("else {\n            ()"));

    contract.circuits[0].body = Expr::If {
        condition: Box::new(Expr::Boolean { value: true }),
        then: Box::new(Expr::Assert {
            condition: Box::new(Expr::Boolean { value: false }),
            message: "guarded".into(),
        }),
        otherwise: Box::new(Expr::Unit),
    };
    let guarded_assertion = render(&contract).unwrap();
    assert!(guarded_assertion.contains("if (true) && (!(false))"));
    assert!(!guarded_assertion.contains("if true {\n            if"));
}

#[test]
fn pure_fallible_tail_returns_its_result_without_an_extra_try() {
    let mut contract = identity(
        Type::Field,
        Expr::Call {
            name: "target".into(),
            arguments: vec![Expr::Parameter {
                name: "value".into(),
            }],
        },
    );
    contract.circuits.push(PureCircuit {
        source: None,
        internal: false,
        name: "target".into(),
        parameters: contract.circuits[0].parameters.clone(),
        result: Type::Field,
        body: Expr::Parameter {
            name: "value".into(),
        },
    });
    let called = render(&contract).unwrap();
    assert!(called.contains("crate::pure_circuits::target(value)\n"));
    assert!(!called.contains("Ok(crate::pure_circuits::target(value)?)"));
    assert!(called.contains("Ok(value)"));

    contract.circuits.pop();
    contract.circuits[0].parameters[0].ty = Type::Unsigned { max: "4".into() };
    contract.circuits[0].result = Type::Unsigned { max: "255".into() };
    contract.circuits[0].body = Expr::UnsignedCast {
        max: "255".into(),
        value: Box::new(Expr::Parameter {
            name: "value".into(),
        }),
    };
    let cast = render(&contract).unwrap();
    assert!(cast.contains("runtime::cast_unsigned::<4, 255>(value)"));
    assert!(!cast.contains("runtime::cast_unsigned::<4, 255>(value)?"));
}

#[test]
fn opposite_boolean_literal_branches_share_one_ast_normalization() {
    let mut contract = identity(
        Type::Boolean,
        Expr::If {
            condition: Box::new(Expr::Parameter {
                name: "value".into(),
            }),
            then: Box::new(Expr::Boolean { value: true }),
            otherwise: Box::new(Expr::Boolean { value: false }),
        },
    );
    contract.circuits[0].parameters[0].ty = Type::Boolean;
    let positive = render(&contract).unwrap();
    assert!(positive.contains("Ok(value)"));
    assert!(!positive.contains("if value { true } else { false }"));

    contract.circuits[0].body = Expr::If {
        condition: Box::new(Expr::Parameter {
            name: "value".into(),
        }),
        then: Box::new(Expr::Boolean { value: false }),
        otherwise: Box::new(Expr::Boolean { value: true }),
    };
    let negative = render(&contract).unwrap();
    assert!(negative.contains("Ok(!(value))"));

    contract.circuits[0].body = Expr::If {
        condition: Box::new(Expr::Parameter {
            name: "value".into(),
        }),
        then: Box::new(Expr::Parameter {
            name: "value".into(),
        }),
        otherwise: Box::new(Expr::Boolean { value: false }),
    };
    let nonliteral = render(&contract).unwrap();
    assert!(nonliteral.contains("if value { value } else { false }"));

    contract.circuits[0].body = Expr::If {
        condition: Box::new(Expr::Parameter {
            name: "value".into(),
        }),
        then: Box::new(Expr::Sequence {
            steps: vec![Expr::Assert {
                condition: Box::new(Expr::Boolean { value: true }),
                message: "keep assertion".into(),
            }],
            value: Box::new(Expr::Boolean { value: true }),
        }),
        otherwise: Box::new(Expr::Boolean { value: false }),
    };
    let effectful = render(&contract).unwrap();
    assert!(effectful.contains("keep assertion"));
    assert!(effectful.contains("if value"));

    contract.circuits[0].result = Type::Unit;
    contract.circuits[0].body = Expr::Assert {
        condition: Box::new(Expr::If {
            condition: Box::new(Expr::Parameter {
                name: "value".into(),
            }),
            then: Box::new(Expr::Boolean { value: false }),
            otherwise: Box::new(Expr::Boolean { value: true }),
        }),
        message: "one read".into(),
    };
    let assertion = render(&contract).unwrap();
    assert!(assertion.contains("if value {"));
    assert!(!assertion.contains("!(!"));
}

#[test]
fn identical_if_arms_keep_one_condition_and_one_arm() {
    let mut contract = identity(
        Type::Boolean,
        Expr::If {
            condition: Box::new(Expr::Call {
                name: "probe".into(),
                arguments: vec![Expr::Parameter {
                    name: "value".into(),
                }],
            }),
            then: Box::new(Expr::Boolean { value: true }),
            otherwise: Box::new(Expr::Boolean { value: true }),
        },
    );
    contract.circuits[0].parameters[0].ty = Type::Boolean;
    contract.circuits.push(PureCircuit {
        source: None,
        internal: false,
        name: "probe".into(),
        parameters: contract.circuits[0].parameters.clone(),
        result: Type::Boolean,
        body: Expr::Parameter {
            name: "value".into(),
        },
    });
    let pure = render(&contract).unwrap();
    assert_eq!(
        pure.matches("crate::pure_circuits::probe(value)?").count(),
        1
    );
    assert!(!pure.contains("if crate::pure_circuits::probe(value)?"));

    contract.circuits[0].body = Expr::If {
        condition: Box::new(Expr::Parameter {
            name: "value".into(),
        }),
        then: Box::new(Expr::Boolean { value: true }),
        otherwise: Box::new(Expr::Boolean { value: true }),
    };
    let effect_free = render(&contract).unwrap();
    assert!(effect_free.contains("Ok(true)"));
    assert!(!effect_free.contains("let _ = value;"));

    contract.circuits[0].body = Expr::If {
        condition: Box::new(Expr::Call {
            name: "probe".into(),
            arguments: vec![Expr::Parameter {
                name: "value".into(),
            }],
        }),
        then: Box::new(Expr::Parameter {
            name: "value".into(),
        }),
        otherwise: Box::new(Expr::Boolean { value: false }),
    };
    let different = render(&contract).unwrap();
    assert!(different.contains("if crate::pure_circuits::probe(value)?"));

    contract.witnesses.push(WitnessDeclaration {
        source: None,
        name: "observed".into(),
        parameters: vec![],
        result: Type::Boolean,
    });
    contract.stateful_circuits.push(StatefulCircuit {
        source: None,
        internal: false,
        name: "record".into(),
        parameters: vec![],
        actions: vec![StateAction::Expression {
            value: Expr::If {
                condition: Box::new(Expr::WitnessCall {
                    name: "observed".into(),
                    arguments: vec![],
                }),
                then: Box::new(Expr::Boolean { value: true }),
                otherwise: Box::new(Expr::Boolean { value: true }),
            },
        }],
        result: Type::Unit,
        return_value: StateReturn::Unit,
    });
    let stateful = render(&contract).unwrap();
    assert_eq!(stateful.matches(".observed(").count(), 1);
    assert!(stateful.contains("private_transcript_outputs"));
}

#[test]
fn pure_block_conditions_are_lifted_before_if_and_assertions() {
    let block_condition = || Expr::Let {
        bindings: vec![LocalBinding {
            name: "flag".into(),
            ty: Type::Boolean,
            value: Expr::Parameter {
                name: "value".into(),
            },
        }],
        body: Box::new(Expr::Parameter {
            name: "flag".into(),
        }),
    };
    let mut contract = identity(
        Type::Boolean,
        Expr::If {
            condition: Box::new(block_condition()),
            then: Box::new(Expr::Boolean { value: true }),
            otherwise: Box::new(Expr::Boolean { value: false }),
        },
    );
    contract.circuits[0].parameters[0].ty = Type::Boolean;
    let conditional = render(&contract).unwrap();
    assert!(conditional.contains("let __compact_condition: bool = {"));
    assert!(!conditional.contains("if {"));

    contract.circuits[0].result = Type::Unit;
    contract.circuits[0].body = Expr::Assert {
        condition: Box::new(Expr::If {
            condition: Box::new(block_condition()),
            then: Box::new(Expr::Boolean { value: false }),
            otherwise: Box::new(Expr::Boolean { value: true }),
        }),
        message: "lifted".into(),
    };
    let assertion = render(&contract).unwrap();
    assert!(assertion.contains("let __compact_condition: bool = {"));
    assert!(assertion.contains("AssertionFailed(\"lifted\".to_owned())"));
    assert!(!assertion.contains("if {"));

    contract.circuits[0].parameters[0].name = "__compact_condition".into();
    contract.circuits[0].body = Expr::If {
        condition: Box::new(Expr::Let {
            bindings: vec![LocalBinding {
                name: "flag".into(),
                ty: Type::Boolean,
                value: Expr::Parameter {
                    name: "__compact_condition".into(),
                },
            }],
            body: Box::new(Expr::Parameter {
                name: "flag".into(),
            }),
        }),
        then: Box::new(Expr::Unit),
        otherwise: Box::new(Expr::Unit),
    };
    let collision = render(&contract).unwrap();
    assert!(collision.contains("let __compact_condition_1: bool = {"));

    contract.circuits[0].result = Type::Boolean;
    contract.circuits[0].body = Expr::If {
        condition: Box::new(Expr::Parameter {
            name: "__compact_condition".into(),
        }),
        then: Box::new(Expr::Parameter {
            name: "__compact_condition".into(),
        }),
        otherwise: Box::new(Expr::Boolean { value: false }),
    };
    let simple = render(&contract).unwrap();
    assert!(!simple.contains("let __compact_condition_1"));
}

#[test]
fn public_state_getters_follow_declared_types_and_escaped_names() {
    let mut contract = identity(Type::Unit, Expr::Unit);
    contract.ledger_fields = vec![
        LedgerField {
            source: None,
            id: "type".into(),
            index: 0,
            path: vec![],
            declaration: LedgerFieldKind::Counter,
        },
        LedgerField {
            source: None,
            id: "flag".into(),
            index: 1,
            path: vec![],
            declaration: LedgerFieldKind::Cell { ty: Type::Boolean },
        },
        LedgerField {
            source: None,
            id: "from".into(),
            index: 2,
            path: vec![],
            declaration: LedgerFieldKind::Cell { ty: Type::Boolean },
        },
    ];
    let source = render(&contract).unwrap();
    assert!(source.contains("pub struct PublicStateView<'a, D:"));
    assert!(source.contains("pub fn r#type("));
    assert!(source.contains("pub fn flag(&self)"));
    assert!(source.contains("crate::ledger_slots::r#type.inspect(self.state)"));
    assert!(source.contains("crate::ledger_slots::flag.inspect(self.state)"));
    assert!(source.contains("pub fn from(&self)"));
    assert!(source.contains("runtime::public_state::PublicStateSource"));
    assert!(source.contains("source.public_state()"));
}

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
fn generated_unit_enum_uses_checked_derive_without_handwritten_codecs() {
    let choice = Type::Enum {
        name: "Choice".into(),
        variants: vec!["yes".into(), "no".into()],
    };
    let mut contract = identity(
        choice.clone(),
        Expr::Parameter {
            name: "value".into(),
        },
    );
    contract.circuits[0].parameters[0].ty = choice;
    let source = render(&contract).unwrap();
    assert!(source.contains("CompactCellValue, CompactEnum"));
    assert!(source.contains("pub enum Choice"));
    assert!(source.contains("RUST_RUNTIME_ABI == 37"));
    assert!(!source.contains("impl FieldRepr for Choice"));
    assert!(!source.contains("impl BinaryHashRepr for Choice"));
    assert!(!source.contains("impl FromFieldRepr for Choice"));
}

#[test]
fn witness_collection_and_merkle_getters_use_declared_slots() {
    let mut contract = identity(Type::Unit, Expr::Unit);
    contract.witnesses.push(WitnessDeclaration {
        source: None,
        name: "observe".into(),
        parameters: vec![],
        result: Type::Unit,
    });
    contract.ledger_fields = vec![
        LedgerField {
            source: None,
            id: "seen".into(),
            index: 0,
            path: vec![],
            declaration: LedgerFieldKind::Set { ty: Type::Boolean },
        },
        LedgerField {
            source: None,
            id: "values".into(),
            index: 1,
            path: vec![],
            declaration: LedgerFieldKind::Map {
                key: Type::Boolean,
                value: Type::Boolean,
            },
        },
        LedgerField {
            source: None,
            id: "queue".into(),
            index: 2,
            path: vec![],
            declaration: LedgerFieldKind::List { ty: Type::Boolean },
        },
        LedgerField {
            source: None,
            id: "plain".into(),
            index: 3,
            path: vec![],
            declaration: LedgerFieldKind::MerkleTree {
                depth: 3,
                ty: Type::Boolean,
            },
        },
        LedgerField {
            source: None,
            id: "historic".into(),
            index: 4,
            path: vec![],
            declaration: LedgerFieldKind::HistoricMerkleTree {
                depth: 4,
                ty: Type::Boolean,
            },
        },
    ];
    let source = render(&contract).unwrap();
    syn::parse_file(&source).unwrap();
    for field in ["seen", "values", "queue", "plain", "historic"] {
        assert!(source.contains(&format!(
            "crate::ledger_slots::{field}.witness_view(self.meter)"
        )));
    }
    assert!(!source.contains("runtime::ledger::metered_"));
}

#[test]
fn native_merkle_calls_use_declared_typed_slots() {
    let mut contract = identity(Type::Unit, Expr::Unit);
    let leaf = Type::Unsigned { max: "255".into() };
    contract.ledger_fields = vec![
        LedgerField {
            source: None,
            id: "plain".into(),
            index: 0,
            path: vec![],
            declaration: LedgerFieldKind::MerkleTree {
                ty: leaf.clone(),
                depth: 3,
            },
        },
        LedgerField {
            source: None,
            id: "historic".into(),
            index: 1,
            path: vec![],
            declaration: LedgerFieldKind::HistoricMerkleTree { ty: leaf, depth: 4 },
        },
    ];
    contract.stateful_circuits = vec![
        StatefulCircuit {
            source: None,
            internal: false,
            name: "plain_full".into(),
            parameters: vec![Parameter {
                name: "value".into(),
                ty: Type::Unsigned { max: "255".into() },
            }],
            actions: vec![StateAction::MerkleInsert {
                field: "plain".into(),
                index: 0,
                value: Expr::Parameter {
                    name: "value".into(),
                },
            }],
            result: Type::Boolean,
            return_value: StateReturn::MerkleIsFull {
                field: "plain".into(),
                index: 0,
            },
        },
        StatefulCircuit {
            source: None,
            internal: false,
            name: "reset_history".into(),
            parameters: vec![],
            actions: vec![StateAction::HistoricMerkleResetHistory {
                field: "historic".into(),
                index: 1,
            }],
            result: Type::Boolean,
            return_value: StateReturn::HistoricMerkleIsFull {
                field: "historic".into(),
                index: 1,
            },
        },
    ];
    let source = render(&contract).unwrap();
    assert!(source.contains(
        "pub const plain: runtime::slots::MerkleSlot<runtime::BoundedUint<255>, 3u8, false>"
    ));
    assert!(source.contains("pub const historic: runtime::slots::MerkleSlot<"));
    assert!(source.contains("4u8,\n        true,"));
    assert!(source.contains("pub struct PublicStateView<'a, D:"));
    assert!(source.contains("pub fn plain("));
    assert!(source.contains("pub fn historic("));
    assert!(source.contains("runtime::slots::PlainMerkleStateView<"));
    assert!(source.contains("runtime::slots::HistoricMerkleStateView<"));
    assert!(source.contains("crate::ledger_slots::plain.inspect(self.state)"));
    assert!(source.contains("crate::ledger_slots::historic.inspect(self.state)"));
    assert!(source.contains("crate::ledger_slots::plain.insert(context, __compact_param_0)?"));
    assert!(source.contains("crate::ledger_slots::plain.is_full(context)?"));
    assert!(source.contains("crate::ledger_slots::historic.reset_history(context)?"));
    assert!(source.contains("crate::ledger_slots::historic.is_full(context)?"));
    assert!(!source.contains("context.merkle_insert(0,"));
}

#[test]
fn plain_merkle_hash_recording_requires_declared_slot_and_exact_bytes32_source() {
    let mut contract = identity(Type::Unit, Expr::Unit);
    contract.ledger_fields = vec![LedgerField {
        source: None,
        id: "tree".into(),
        index: 0,
        path: vec![],
        declaration: LedgerFieldKind::MerkleTree {
            ty: Type::Unsigned { max: "255".into() },
            depth: 3,
        },
    }];
    contract.stateful_circuits = vec![StatefulCircuit {
        source: None,
        internal: false,
        name: "append_hash".into(),
        parameters: vec![Parameter {
            name: "hash".into(),
            ty: Type::Bytes { length: 32 },
        }],
        actions: vec![StateAction::MerkleInsertHash {
            field: "tree".into(),
            index: 0,
            hash: Expr::Parameter {
                name: "hash".into(),
            },
        }],
        result: Type::Unit,
        return_value: StateReturn::Unit,
    }];
    let rendered = render_with_capabilities(&contract).unwrap();
    assert!(rendered.capabilities.circuits[0].recorded);
    assert!(rendered.capabilities.circuits[0].observed_call);
    assert!(rendered.source.contains(".record_insert_hash(frame,"));

    if let StateAction::MerkleInsertHash { index, .. } =
        &mut contract.stateful_circuits[0].actions[0]
    {
        *index = 1;
    }
    assert!(render_with_capabilities(&contract).is_err());

    if let StateAction::MerkleInsertHash { index, .. } =
        &mut contract.stateful_circuits[0].actions[0]
    {
        *index = 0;
    }
    contract.stateful_circuits[0].parameters[0].ty = Type::Bytes { length: 31 };
    assert!(render_with_capabilities(&contract).is_err());
    contract.stateful_circuits[0].parameters[0].ty = Type::Bytes { length: 32 };
    contract.ledger_fields[0].declaration = LedgerFieldKind::HistoricMerkleTree {
        ty: Type::Unsigned { max: "255".into() },
        depth: 3,
    };
    assert!(render_with_capabilities(&contract).is_err());
    contract.ledger_fields[0].declaration = LedgerFieldKind::MerkleTree {
        ty: Type::Unsigned { max: "255".into() },
        depth: 3,
    };
    let StateAction::MerkleInsertHash { hash, .. } = &mut contract.stateful_circuits[0].actions[0]
    else {
        unreachable!()
    };
    *hash = Expr::If {
        condition: Box::new(Expr::Boolean { value: true }),
        then: Box::new(Expr::Parameter {
            name: "hash".into(),
        }),
        otherwise: Box::new(Expr::Parameter {
            name: "hash".into(),
        }),
    };
    let rejected = render_with_capabilities(&contract).unwrap();
    assert!(!rejected.capabilities.circuits[0].recorded);
}

#[test]
fn plain_merkle_indexed_hash_recording_requires_exact_hash_and_uint64_position() {
    let mut contract = identity(Type::Unit, Expr::Unit);
    contract.ledger_fields = vec![LedgerField {
        source: None,
        id: "tree".into(),
        index: 0,
        path: vec![],
        declaration: LedgerFieldKind::MerkleTree {
            ty: Type::Unsigned { max: "255".into() },
            depth: 3,
        },
    }];
    contract.stateful_circuits = vec![StatefulCircuit {
        source: None,
        internal: false,
        name: "place_hash".into(),
        parameters: vec![
            Parameter {
                name: "hash".into(),
                ty: Type::Bytes { length: 32 },
            },
            Parameter {
                name: "position".into(),
                ty: Type::Unsigned {
                    max: u64::MAX.to_string(),
                },
            },
        ],
        actions: vec![StateAction::MerkleInsertHashIndex {
            field: "tree".into(),
            index: 0,
            hash: Expr::Parameter {
                name: "hash".into(),
            },
            position: Expr::Parameter {
                name: "position".into(),
            },
        }],
        result: Type::Unit,
        return_value: StateReturn::Unit,
    }];
    let rendered = render_with_capabilities(&contract).unwrap();
    assert!(rendered.capabilities.circuits[0].recorded);
    assert!(rendered.capabilities.circuits[0].observed_call);
    assert!(rendered.source.contains(".record_insert_hash_index(frame,"));

    contract.ledger_fields[0].path = vec![0, 0];
    assert!(render_with_capabilities(&contract).is_err());
    contract.ledger_fields[0].path.clear();
    contract.stateful_circuits[0].parameters[0].ty = Type::Bytes { length: 31 };
    assert!(render_with_capabilities(&contract).is_err());
    contract.stateful_circuits[0].parameters[0].ty = Type::Bytes { length: 32 };
    contract.stateful_circuits[0].parameters[1].ty = Type::Unsigned { max: "255".into() };
    assert!(render_with_capabilities(&contract).is_err());
    contract.stateful_circuits[0].parameters[1].ty = Type::Unsigned {
        max: u64::MAX.to_string(),
    };
    contract.ledger_fields[0].declaration = LedgerFieldKind::HistoricMerkleTree {
        ty: Type::Unsigned { max: "255".into() },
        depth: 3,
    };
    assert!(render_with_capabilities(&contract).is_err());
    contract.ledger_fields[0].declaration = LedgerFieldKind::MerkleTree {
        ty: Type::Unsigned { max: "255".into() },
        depth: 3,
    };
    let StateAction::MerkleInsertHashIndex { position, .. } =
        &mut contract.stateful_circuits[0].actions[0]
    else {
        unreachable!()
    };
    *position = Expr::If {
        condition: Box::new(Expr::Boolean { value: true }),
        then: Box::new(Expr::Parameter {
            name: "position".into(),
        }),
        otherwise: Box::new(Expr::Parameter {
            name: "position".into(),
        }),
    };
    let rejected = render_with_capabilities(&contract).unwrap();
    assert!(!rejected.capabilities.circuits[0].recorded);
}

#[test]
fn indexed_merkle_recording_requires_typed_leaf_and_position_sources() {
    let leaf = Type::Unsigned { max: "255".into() };
    let position = Type::Unsigned {
        max: u64::MAX.to_string(),
    };
    let mut contract = identity(Type::Unit, Expr::Unit);
    contract.ledger_fields = vec![LedgerField {
        source: None,
        id: "plain".into(),
        index: 0,
        path: vec![],
        declaration: LedgerFieldKind::MerkleTree {
            ty: leaf.clone(),
            depth: 3,
        },
    }];
    contract.stateful_circuits = vec![
        StatefulCircuit {
            source: None,
            internal: false,
            name: "place".into(),
            parameters: vec![
                Parameter {
                    name: "value".into(),
                    ty: leaf,
                },
                Parameter {
                    name: "position".into(),
                    ty: position.clone(),
                },
            ],
            actions: vec![StateAction::MerkleInsertIndex {
                field: "plain".into(),
                index: 0,
                value: Expr::Parameter {
                    name: "value".into(),
                },
                position: Expr::Parameter {
                    name: "position".into(),
                },
            }],
            result: Type::Unit,
            return_value: StateReturn::Unit,
        },
        StatefulCircuit {
            source: None,
            internal: false,
            name: "default_at".into(),
            parameters: vec![Parameter {
                name: "position".into(),
                ty: position,
            }],
            actions: vec![StateAction::MerkleInsertIndexDefault {
                field: "plain".into(),
                index: 0,
                position: Expr::Parameter {
                    name: "position".into(),
                },
            }],
            result: Type::Unit,
            return_value: StateReturn::Unit,
        },
    ];
    let rendered = render_with_capabilities(&contract).unwrap();
    assert!(
        rendered
            .capabilities
            .circuits
            .iter()
            .all(|c| c.recorded && c.observed_call)
    );
    assert!(rendered.source.contains(".record_insert_index("));
    assert!(rendered.source.contains(".record_insert_index_default("));

    let StateAction::MerkleInsertIndex { position, .. } =
        &mut contract.stateful_circuits[0].actions[0]
    else {
        unreachable!()
    };
    *position = Expr::If {
        condition: Box::new(Expr::Boolean { value: true }),
        then: Box::new(Expr::Parameter {
            name: "position".into(),
        }),
        otherwise: Box::new(Expr::Parameter {
            name: "position".into(),
        }),
    };
    let rejected = render_with_capabilities(&contract).unwrap();
    assert!(!rejected.capabilities.circuits[0].recorded);
    assert_eq!(
        rejected.capabilities.circuits[0]
            .recording_unavailable
            .as_ref()
            .unwrap()
            .ir_node,
        "StateAction::MerkleInsertIndex"
    );
    assert!(rejected.capabilities.circuits[1].recorded);

    let StateAction::MerkleInsertIndex {
        value, position, ..
    } = &mut contract.stateful_circuits[0].actions[0]
    else {
        unreachable!()
    };
    *position = Expr::Parameter {
        name: "position".into(),
    };
    *value = Expr::If {
        condition: Box::new(Expr::Boolean { value: true }),
        then: Box::new(Expr::Parameter {
            name: "value".into(),
        }),
        otherwise: Box::new(Expr::Parameter {
            name: "value".into(),
        }),
    };
    let rejected = render_with_capabilities(&contract).unwrap();
    assert!(!rejected.capabilities.circuits[0].recorded);
    assert!(rejected.capabilities.circuits[1].recorded);
}

#[test]
fn indexed_merkle_recording_accepts_only_typed_literal_position_binding() {
    let position = Type::Unsigned {
        max: u64::MAX.to_string(),
    };
    let mut contract = identity(Type::Unit, Expr::Unit);
    contract.ledger_fields = vec![LedgerField {
        source: None,
        id: "tree".into(),
        index: 0,
        path: vec![],
        declaration: LedgerFieldKind::MerkleTree {
            depth: 3,
            ty: Type::Unsigned { max: "255".into() },
        },
    }];
    contract.stateful_circuits = vec![StatefulCircuit {
        source: None,
        name: "replace".into(),
        internal: false,
        parameters: vec![Parameter {
            name: "value".into(),
            ty: Type::Unsigned { max: "255".into() },
        }],
        actions: vec![StateAction::Let {
            bindings: vec![LocalBinding {
                name: "position".into(),
                ty: position.clone(),
                value: Expr::UnsignedLiteral {
                    value: "0".into(),
                    max: u64::MAX.to_string(),
                },
            }],
            action: Box::new(StateAction::MerkleInsertIndex {
                field: "tree".into(),
                index: 0,
                value: Expr::Parameter {
                    name: "value".into(),
                },
                position: Expr::Parameter {
                    name: "position".into(),
                },
            }),
        }],
        result: Type::Unit,
        return_value: StateReturn::Unit,
    }];
    let rendered = render_with_capabilities(&contract).unwrap();
    assert!(rendered.capabilities.circuits[0].recorded);
    assert!(rendered.capabilities.circuits[0].observed_call);
    assert!(rendered.source.contains(".record_insert_index("));

    let StateAction::Let { bindings, .. } = &mut contract.stateful_circuits[0].actions[0] else {
        unreachable!()
    };
    bindings[0].value = Expr::If {
        condition: Box::new(Expr::Boolean { value: true }),
        then: Box::new(Expr::UnsignedLiteral {
            value: "0".into(),
            max: u64::MAX.to_string(),
        }),
        otherwise: Box::new(Expr::UnsignedLiteral {
            value: "1".into(),
            max: u64::MAX.to_string(),
        }),
    };
    let rejected = render_with_capabilities(&contract).unwrap();
    assert!(!rejected.capabilities.circuits[0].recorded);
    assert!(!rejected.capabilities.circuits[0].observed_call);
}

#[test]
fn direct_merkle_fullness_reads_record_only_matching_declared_slots() {
    let mut contract = identity(Type::Unit, Expr::Unit);
    contract.ledger_fields = vec![
        LedgerField {
            source: None,
            id: "plain".into(),
            index: 0,
            path: vec![],
            declaration: LedgerFieldKind::MerkleTree {
                ty: Type::Boolean,
                depth: 3,
            },
        },
        LedgerField {
            source: None,
            id: "historic".into(),
            index: 1,
            path: vec![],
            declaration: LedgerFieldKind::HistoricMerkleTree {
                ty: Type::Boolean,
                depth: 3,
            },
        },
    ];
    contract.stateful_circuits = vec![
        StatefulCircuit {
            source: None,
            internal: false,
            name: "plain_full".into(),
            parameters: vec![],
            actions: vec![],
            result: Type::Boolean,
            return_value: StateReturn::MerkleIsFull {
                field: "plain".into(),
                index: 0,
            },
        },
        StatefulCircuit {
            source: None,
            internal: false,
            name: "historic_full".into(),
            parameters: vec![],
            actions: vec![],
            result: Type::Boolean,
            return_value: StateReturn::HistoricMerkleIsFull {
                field: "historic".into(),
                index: 1,
            },
        },
    ];
    let rendered = render_with_capabilities(&contract).unwrap();
    assert_eq!(rendered.source.matches("record_is_full(frame)?").count(), 2);
    let fullness = rendered
        .capabilities
        .circuits
        .iter()
        .filter(|circuit| circuit.name.ends_with("_full"))
        .collect::<Vec<_>>();
    assert_eq!(fullness.len(), 2);
    for circuit in fullness {
        assert!(circuit.recorded && circuit.observed_call);
    }

    contract.stateful_circuits[0].return_value = StateReturn::HistoricMerkleIsFull {
        field: "plain".into(),
        index: 0,
    };
    assert_eq!(
        render(&contract),
        Err(RenderError::UnknownLedgerField("plain".into()))
    );
    contract.stateful_circuits[0].return_value = StateReturn::MerkleIsFull {
        field: "plain".into(),
        index: 1,
    };
    assert_eq!(
        render(&contract),
        Err(RenderError::UnknownLedgerField("plain".into()))
    );
}

#[test]
fn exported_alias_is_typed_and_reexported() {
    let mut contract = identity(
        Type::Field,
        Expr::Parameter {
            name: "value".into(),
        },
    );
    let alias = TypeAlias {
        source: None,
        name: "Tag".into(),
        ty: Type::Bytes { length: 8 },
    };
    contract.type_aliases.push(alias.clone());
    let source = render(&contract).unwrap();
    assert!(source.contains("pub type Tag = runtime::FixedBytes<8>;"));
    assert!(source.contains("pub use types::Tag;"));
    contract.type_aliases.push(alias);
    assert_eq!(
        render(&contract),
        Err(RenderError::ConflictingTypeAlias("Tag".into()))
    );
}

#[test]
fn compact_dollar_names_and_rust_reserved_names_render_as_identifiers() {
    let mut contract = identity(
        Type::Field,
        Expr::Parameter {
            name: "final".into(),
        },
    );
    contract.circuits[0].name = "vote$commit".into();
    contract.circuits[0].parameters[0].name = "final".into();
    let source = render(&contract).unwrap();
    assert!(source.contains("pub fn vote_commit"), "{source}");
    assert!(source.contains("r#final: runtime::Field"), "{source}");
}

#[test]
fn struct_field_projection_checks_declared_name_and_position() {
    let mut contract = identity(
        Type::Field,
        Expr::StructField {
            value: Box::new(Expr::Parameter {
                name: "value".into(),
            }),
            field: "amount".into(),
            index: 0,
        },
    );
    contract.circuits[0].parameters[0].ty = Type::Struct {
        name: "Record".into(),
        fields: vec![StructField {
            name: "amount".into(),
            ty: Type::Field,
        }],
    };
    assert!(!render(&contract).unwrap().contains(".amount.clone()"));
    let vector = Type::Vector {
        element: Box::new(Type::Field),
        length: 2,
    };
    contract.circuits[0].result = vector.clone();
    contract.circuits[0].parameters[0].ty = Type::Struct {
        name: "Record".into(),
        fields: vec![StructField {
            name: "amount".into(),
            ty: vector,
        }],
    };
    assert!(render(&contract).unwrap().contains(".amount).clone()"));
    let Expr::StructField { index, .. } = &mut contract.circuits[0].body else {
        unreachable!()
    };
    *index = 1;
    assert_eq!(
        render(&contract),
        Err(RenderError::InvalidStructField("amount".into()))
    );
}

#[test]
fn struct_literal_uses_shorthand_only_for_an_uncloned_field_name() {
    let field = Type::Struct {
        name: "Record".into(),
        fields: vec![StructField {
            name: "value".into(),
            ty: Type::Field,
        }],
    };
    let mut contract = identity(
        field.clone(),
        Expr::StructLiteral {
            ty: field,
            fields: vec![Expr::Parameter {
                name: "value".into(),
            }],
        },
    );
    let source = render(&contract).unwrap();
    assert!(
        source.contains("crate::types::Record { value }"),
        "{source}"
    );
    assert!(!source.contains("value: value"));

    let vector = Type::Vector {
        element: Box::new(Type::Field),
        length: 2,
    };
    let record = Type::Struct {
        name: "Record".into(),
        fields: vec![StructField {
            name: "value".into(),
            ty: vector.clone(),
        }],
    };
    contract.circuits[0].parameters[0].ty = vector;
    contract.circuits[0].result = record.clone();
    contract.circuits[0].body = Expr::StructLiteral {
        ty: record,
        fields: vec![Expr::Parameter {
            name: "value".into(),
        }],
    };
    let source = render(&contract).unwrap();
    assert!(source.contains("value: value.clone()"), "{source}");
}

#[test]
fn struct_literal_checks_field_value_types() {
    let ty = Type::Struct {
        name: "Record".into(),
        fields: vec![StructField {
            name: "amount".into(),
            ty: Type::Field,
        }],
    };
    let mut contract = identity(
        ty.clone(),
        Expr::StructLiteral {
            ty,
            fields: vec![Expr::FieldLiteral { value: "7".into() }],
        },
    );
    assert!(render(&contract).is_ok());
    let Expr::StructLiteral { fields, .. } = &mut contract.circuits[0].body else {
        unreachable!()
    };
    fields[0] = Expr::Boolean { value: true };
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Field,
            actual: Type::Boolean,
        })
    );
}

#[test]
fn repeated_vector_parameters_keep_value_semantics() {
    let vector = Type::Vector {
        element: Box::new(Type::Field),
        length: 2,
    };
    let mut contract = identity(
        Type::Tuple {
            elements: vec![vector.clone(), vector.clone()],
        },
        Expr::Tuple {
            elements: vec![
                Expr::Parameter {
                    name: "value".into(),
                },
                Expr::Parameter {
                    name: "value".into(),
                },
            ],
        },
    );
    contract.circuits[0].parameters[0].ty = vector;
    let source = render(&contract).unwrap();
    assert_eq!(source.matches("value.clone()").count(), 2);
}

#[test]
fn observed_call_input_clones_non_copy_values_but_not_copy_values() {
    let mut contract = identity(Type::Unit, Expr::Unit);
    let vector = Type::Vector {
        element: Box::new(Type::Field),
        length: 2,
    };
    contract.ledger_fields = vec![LedgerField {
        source: None,
        id: "stored".into(),
        index: 0,
        path: vec![],
        declaration: LedgerFieldKind::List { ty: vector.clone() },
    }];
    contract.stateful_circuits = vec![StatefulCircuit {
        source: None,
        internal: false,
        name: "write".into(),
        parameters: vec![Parameter {
            name: "value".into(),
            ty: vector,
        }],
        result: Type::Unit,
        return_value: StateReturn::Unit,
        actions: vec![StateAction::ListPushFront {
            field: "stored".into(),
            index: 0,
            value: Expr::Parameter {
                name: "value".into(),
            },
        }],
    }];
    let source = render(&contract).unwrap();
    assert!(
        source.contains("AlignedValue::from((value).clone())"),
        "{source}"
    );

    contract.ledger_fields[0].declaration = LedgerFieldKind::List { ty: Type::Field };
    contract.stateful_circuits[0].parameters[0].ty = Type::Field;
    let source = render(&contract).unwrap();
    assert!(source.contains("AlignedValue::from(value)"), "{source}");
    assert!(!source.contains("AlignedValue::from((value).clone())"));
}

#[test]
fn vector_coercion_rejects_length_changes() {
    let target = Type::Vector {
        element: Box::new(Type::Field),
        length: 2,
    };
    let mut contract = identity(
        target.clone(),
        Expr::Coerce {
            value: Box::new(Expr::Parameter {
                name: "value".into(),
            }),
            ty: target,
        },
    );
    let source = Type::Vector {
        element: Box::new(Type::Unsigned { max: "255".into() }),
        length: 2,
    };
    contract.circuits[0].parameters[0].ty = source.clone();
    assert!(render(&contract).unwrap().contains("into_array()"));
    let Expr::Coerce { ty, .. } = &mut contract.circuits[0].body else {
        unreachable!()
    };
    *ty = Type::Vector {
        element: Box::new(Type::Field),
        length: 3,
    };
    let expected = ty.clone();
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected,
            actual: source,
        })
    );
}

#[test]
fn tuple_coercion_widens_uints_but_rejects_narrowing() {
    let source = Type::Tuple {
        elements: vec![
            Type::Unsigned { max: "255".into() },
            Type::Unsigned { max: "255".into() },
        ],
    };
    let target = Type::Tuple {
        elements: vec![
            Type::Field,
            Type::Unsigned {
                max: "65535".into(),
            },
        ],
    };
    let mut contract = identity(
        target.clone(),
        Expr::Coerce {
            value: Box::new(Expr::Parameter {
                name: "value".into(),
            }),
            ty: target,
        },
    );
    contract.circuits[0].parameters[0].ty = source;
    assert!(
        render(&contract)
            .unwrap()
            .contains("cast_unsigned::<255, 65535>")
    );
    let Expr::Coerce { ty, .. } = &mut contract.circuits[0].body else {
        unreachable!()
    };
    *ty = Type::Tuple {
        elements: vec![Type::Field, Type::Unsigned { max: "1".into() }],
    };
    contract.circuits[0].result = ty.clone();
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Unsigned { max: "1".into() },
            actual: Type::Unsigned { max: "255".into() },
        })
    );
}

#[test]
fn constructor_cell_parameters_are_typed_and_validated() {
    let mut contract = identity(
        Type::Field,
        Expr::Parameter {
            name: "value".into(),
        },
    );
    contract.ledger_fields = vec![LedgerField {
        source: None,
        id: "stored".into(),
        index: 0,
        path: vec![],
        declaration: LedgerFieldKind::Cell { ty: Type::Field },
    }];
    contract.constructor = Some(Constructor {
        source: None,
        parameters: vec![Parameter {
            name: "seed".into(),
            ty: Type::Field,
        }],
        steps: vec![ConstructorStep::CellWrite {
            field: "stored".into(),
            index: 0,
            value: Expr::Parameter {
                name: "seed".into(),
            },
        }],
    });
    let source = render(&contract).unwrap();
    assert!(source.contains("__compact_constructor_param_0: runtime::Field"));
    assert!(source.contains("let __compact_constructor_value_0 = __compact_constructor_param_0;"));
    assert!(!source.contains("__compact_constructor_value_0.clone()"));
    assert!(
        source.contains(
            "Result<runtime::context::ConstructorResult<Private>, runtime::CompactError>"
        )
    );

    let constructor = contract.constructor.as_mut().unwrap();
    let ConstructorStep::CellWrite { index, .. } = &mut constructor.steps[0] else {
        panic!("expected Cell write")
    };
    *index = 1;
    assert_eq!(render(&contract), Err(RenderError::InvalidLedgerIndex(1)));
    let ConstructorStep::CellWrite { index, .. } =
        &mut contract.constructor.as_mut().unwrap().steps[0]
    else {
        panic!("expected Cell write")
    };
    *index = 0;
    contract.constructor.as_mut().unwrap().parameters[0].ty = Type::Boolean;
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Field,
            actual: Type::Boolean
        })
    );
    contract.constructor.as_mut().unwrap().parameters[0].ty = Type::Field;
    let duplicate = contract.constructor.as_ref().unwrap().steps[0].clone();
    contract.constructor.as_mut().unwrap().steps.push(duplicate);
    let source = render(&contract).unwrap();
    assert_eq!(source.matches("context.write_cell(0,").count(), 2);
}

#[test]
fn constructor_counter_steps_use_typed_vm_calls() {
    let mut contract = identity(
        Type::Field,
        Expr::Parameter {
            name: "value".into(),
        },
    );
    contract.ledger_fields = vec![LedgerField {
        source: None,
        id: "count".into(),
        index: 0,
        path: vec![],
        declaration: LedgerFieldKind::Counter,
    }];
    contract.constructor = Some(Constructor {
        source: None,
        parameters: vec![Parameter {
            name: "amount".into(),
            ty: Type::Unsigned {
                max: "65535".into(),
            },
        }],
        steps: vec![
            ConstructorStep::CounterIncrement {
                field: "count".into(),
                index: 0,
                amount: CounterAmount::Parameter {
                    name: "amount".into(),
                },
            },
            ConstructorStep::CounterIncrement {
                field: "count".into(),
                index: 0,
                amount: CounterAmount::Literal { value: 3 },
            },
            ConstructorStep::CounterReset {
                field: "count".into(),
                index: 0,
            },
            ConstructorStep::CounterDecrement {
                field: "count".into(),
                index: 0,
                amount: CounterAmount::Literal { value: 1 },
            },
        ],
    });
    let source = render(&contract).unwrap();
    assert!(source.contains("increment_counter(0, __compact_constructor_param_0.value() as u16)?"));
    assert!(source.contains("context.increment_counter(0, 3u16)?"));
    assert!(source.contains("context.write_cell(0, 0_u64)?"));
    assert!(source.contains("context.decrement_counter(0, 1u16)?"));
    assert!(source.contains("context.into_constructor_result()"));
    contract.constructor.as_mut().unwrap().parameters[0].ty = Type::Field;
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Unsigned {
                max: "65535".into()
            },
            actual: Type::Field,
        })
    );
}

#[test]
fn constructor_for_each_checks_element_type_and_loop_binding() {
    let mut contract = identity(
        Type::Field,
        Expr::Parameter {
            name: "value".into(),
        },
    );
    contract.ledger_fields = vec![LedgerField {
        source: None,
        id: "count".into(),
        index: 0,
        path: vec![],
        declaration: LedgerFieldKind::Counter,
    }];
    contract.constructor = Some(Constructor {
        source: None,
        parameters: vec![],
        steps: vec![ConstructorStep::ForEach {
            binding: Parameter {
                name: "item".into(),
                ty: Type::Unsigned {
                    max: "65535".into(),
                },
            },
            values: vec![Expr::UnsignedLiteral {
                value: "2".into(),
                max: "65535".into(),
            }],
            steps: vec![ConstructorStep::CounterIncrement {
                field: "count".into(),
                index: 0,
                amount: CounterAmount::Parameter {
                    name: "item".into(),
                },
            }],
        }],
    });
    let source = render(&contract).unwrap();
    assert!(source.contains("for __compact_constructor_item_0 in __compact_constructor_values_0"));
    assert!(source.contains("increment_counter(0, __compact_constructor_item_0.value() as u16)?"));
    let ConstructorStep::ForEach { binding, steps, .. } =
        &mut contract.constructor.as_mut().unwrap().steps[0]
    else {
        unreachable!()
    };
    binding.name = "_".into();
    let ConstructorStep::CounterIncrement { amount, .. } = &mut steps[0] else {
        unreachable!()
    };
    *amount = CounterAmount::Parameter { name: "_".into() };
    assert!(
        render(&contract)
            .unwrap()
            .contains("__compact_constructor_item_0")
    );
    let ConstructorStep::ForEach { values, .. } =
        &mut contract.constructor.as_mut().unwrap().steps[0]
    else {
        unreachable!()
    };
    values[0] = Expr::Boolean { value: true };
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Unsigned {
                max: "65535".into()
            },
            actual: Type::Boolean,
        })
    );
}

#[test]
fn constructor_vector_parameter_loop_checks_shape_and_conditional_order() {
    let mut contract = identity(
        Type::Field,
        Expr::Parameter {
            name: "value".into(),
        },
    );
    contract.ledger_fields = vec![LedgerField {
        source: None,
        id: "count".into(),
        index: 0,
        path: vec![],
        declaration: LedgerFieldKind::Counter,
    }];
    contract.constructor = Some(Constructor {
        source: None,
        parameters: vec![Parameter {
            name: "initial".into(),
            ty: Type::Vector {
                element: Box::new(Type::Boolean),
                length: 2,
            },
        }],
        steps: vec![ConstructorStep::ForEachVector {
            binding: Parameter {
                name: "item".into(),
                ty: Type::Boolean,
            },
            source: Expr::Parameter {
                name: "initial".into(),
            },
            length: 2,
            steps: vec![ConstructorStep::If {
                condition: Expr::Parameter {
                    name: "item".into(),
                },
                then_steps: vec![ConstructorStep::CounterIncrement {
                    field: "count".into(),
                    index: 0,
                    amount: CounterAmount::Literal { value: 1 },
                }],
                otherwise_steps: vec![],
            }],
        }],
    });
    let source = render(&contract).unwrap();
    assert!(source.contains("__compact_constructor_param_0"));
    assert!(source.contains(".iter()"));
    assert!(!source.contains(".cloned()"));
    assert!(!source.contains("into_array()"));
    assert!(source.contains("for __compact_constructor_item_0 in"));
    assert!(source.contains("if __compact_constructor_item_0"));
    assert!(source.contains("context.increment_counter(0, 1u16)?"));

    {
        let ConstructorStep::ForEachVector { length, .. } =
            &mut contract.constructor.as_mut().unwrap().steps[0]
        else {
            unreachable!()
        };
        *length = 3;
    }
    assert!(matches!(
        render(&contract),
        Err(RenderError::TypeMismatch { .. })
    ));
    let ConstructorStep::ForEachVector { length, .. } =
        &mut contract.constructor.as_mut().unwrap().steps[0]
    else {
        unreachable!()
    };
    *length = 2;
    let ConstructorStep::ForEachVector { steps, .. } =
        &mut contract.constructor.as_mut().unwrap().steps[0]
    else {
        unreachable!()
    };
    let ConstructorStep::If { condition, .. } = &mut steps[0] else {
        unreachable!()
    };
    *condition = Expr::SetMember {
        field: "count".into(),
        index: 0,
        value: Box::new(Expr::Boolean { value: true }),
    };
    assert_eq!(
        render(&contract),
        Err(RenderError::InvalidConstructorInitializer("if".into()))
    );
}

#[test]
fn constructor_set_steps_validate_values_and_use_vm_methods() {
    let mut contract = identity(
        Type::Field,
        Expr::Parameter {
            name: "value".into(),
        },
    );
    contract.ledger_fields = vec![LedgerField {
        source: None,
        id: "seen".into(),
        index: 0,
        path: vec![],
        declaration: LedgerFieldKind::Set { ty: Type::Boolean },
    }];
    contract.constructor = Some(Constructor {
        source: None,
        parameters: vec![],
        steps: vec![
            ConstructorStep::SetInsert {
                field: "seen".into(),
                index: 0,
                value: Expr::Boolean { value: true },
            },
            ConstructorStep::SetRemove {
                field: "seen".into(),
                index: 0,
                value: Expr::Boolean { value: false },
            },
            ConstructorStep::SetReset {
                field: "seen".into(),
                index: 0,
            },
        ],
    });
    let source = render(&contract).unwrap();
    assert!(source.contains("context.insert_set(0, true)?"));
    assert!(source.contains("context.remove_set(0, false)?"));
    assert!(source.contains("context.reset_set(0)?"));
    let ConstructorStep::SetInsert { value, .. } =
        &mut contract.constructor.as_mut().unwrap().steps[0]
    else {
        unreachable!()
    };
    *value = Expr::FieldLiteral { value: "1".into() };
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Boolean,
            actual: Type::Field,
        })
    );
}

#[test]
fn constructor_list_steps_validate_values_and_use_vm_methods() {
    let mut contract = identity(
        Type::Field,
        Expr::Parameter {
            name: "value".into(),
        },
    );
    contract.ledger_fields = vec![LedgerField {
        source: None,
        id: "items".into(),
        index: 0,
        path: vec![],
        declaration: LedgerFieldKind::List { ty: Type::Field },
    }];
    contract.constructor = Some(Constructor {
        source: None,
        parameters: vec![],
        steps: vec![
            ConstructorStep::ListPushFront {
                field: "items".into(),
                index: 0,
                value: Expr::FieldLiteral { value: "3".into() },
            },
            ConstructorStep::ListPopFront {
                field: "items".into(),
                index: 0,
            },
            ConstructorStep::ListReset {
                field: "items".into(),
                index: 0,
            },
        ],
    });
    let source = render(&contract).unwrap();
    assert!(source.contains("push_front_list(0,"));
    assert!(source.contains("context.pop_front_list(0)?"));
    assert!(source.contains("context.reset_list(0)?"));
    let ConstructorStep::ListPushFront { value, .. } =
        &mut contract.constructor.as_mut().unwrap().steps[0]
    else {
        unreachable!()
    };
    *value = Expr::Boolean { value: true };
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Field,
            actual: Type::Boolean,
        })
    );
}

#[test]
fn constructor_map_steps_validate_keys_values_and_use_vm_methods() {
    let mut contract = identity(
        Type::Field,
        Expr::Parameter {
            name: "value".into(),
        },
    );
    contract.ledger_fields = vec![LedgerField {
        source: None,
        id: "table".into(),
        index: 0,
        path: vec![],
        declaration: LedgerFieldKind::Map {
            key: Type::Boolean,
            value: Type::Field,
        },
    }];
    contract.constructor = Some(Constructor {
        source: None,
        parameters: vec![],
        steps: vec![
            ConstructorStep::MapInsert {
                field: "table".into(),
                index: 0,
                key: Expr::Boolean { value: true },
                value: Expr::FieldLiteral { value: "7".into() },
            },
            ConstructorStep::MapRemove {
                field: "table".into(),
                index: 0,
                key: Expr::Boolean { value: false },
            },
            ConstructorStep::MapReset {
                field: "table".into(),
                index: 0,
            },
            ConstructorStep::MapInsertDefault {
                field: "table".into(),
                index: 0,
                key: Expr::Boolean { value: false },
            },
        ],
    });
    let source = render(&contract).unwrap();
    for method in ["insert_map(0,", "remove_map(0,", "reset_map(0)?"] {
        assert!(source.contains(method), "missing {method}");
    }
    let ConstructorStep::MapInsert { value, .. } =
        &mut contract.constructor.as_mut().unwrap().steps[0]
    else {
        unreachable!()
    };
    *value = Expr::Boolean { value: true };
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Field,
            actual: Type::Boolean,
        })
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
fn direct_boolean_witness_assertion_records_before_cell_write() {
    let mut contract = identity(Type::Unit, Expr::Unit);
    contract.circuits.clear();
    contract.ledger_fields = vec![LedgerField {
        source: None,
        id: "cell".into(),
        index: 0,
        path: vec![],
        declaration: LedgerFieldKind::Cell { ty: Type::Field },
    }];
    contract.witnesses = vec![WitnessDeclaration {
        source: None,
        name: "echo".into(),
        parameters: vec![Parameter {
            name: "flag".into(),
            ty: Type::Boolean,
        }],
        result: Type::Boolean,
    }];
    contract.stateful_circuits = vec![StatefulCircuit {
        source: None,
        internal: false,
        name: "checked_write".into(),
        parameters: vec![
            Parameter {
                name: "flag".into(),
                ty: Type::Boolean,
            },
            Parameter {
                name: "value".into(),
                ty: Type::Field,
            },
        ],
        actions: vec![
            StateAction::Assert {
                condition: Expr::WitnessCall {
                    name: "echo".into(),
                    arguments: vec![Expr::Parameter {
                        name: "flag".into(),
                    }],
                },
                message: "write denied".into(),
            },
            StateAction::CellWrite {
                field: "cell".into(),
                index: 0,
                value: Expr::Parameter {
                    name: "value".into(),
                },
            },
        ],
        result: Type::Unit,
        return_value: StateReturn::Unit,
    }];

    let rendered = render_with_capabilities(&contract).unwrap();
    assert!(rendered.capabilities.circuits[0].recorded);
    assert!(rendered.capabilities.circuits[0].observed_call);
    let recorded = rendered.source.split("pub mod recorded").nth(1).unwrap();
    let witness = recorded
        .find(".try_witness_metered(|context, meter|")
        .unwrap();
    let assertion = recorded.find("AssertionFailed(\"write denied\"").unwrap();
    let write = recorded.find(".record_write(frame,").unwrap();
    assert!(witness < assertion && assertion < write);
    assert!(recorded.contains("let __compact_recorded_arg_0: bool = __compact_param_0;"));
    assert!(recorded.contains("pub fn checked_write_call<'observed, Private>"));
}

#[test]
fn vector_expression_preserves_element_type() {
    let mut contract = identity(
        Type::Vector {
            element: Box::new(Type::Field),
            length: 2,
        },
        Expr::Vector {
            element: Type::Field,
            elements: vec![
                Expr::FieldLiteral { value: "3".into() },
                Expr::FieldLiteral { value: "5".into() },
            ],
        },
    );
    assert!(
        render(&contract)
            .unwrap()
            .contains("runtime::FixedVector::new")
    );
    contract.circuits[0].body = Expr::Vector {
        element: Type::Field,
        elements: vec![
            Expr::Boolean { value: true },
            Expr::FieldLiteral { value: "5".into() },
        ],
    };
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Field,
            actual: Type::Boolean
        })
    );
}

#[test]
fn byte_literals_keep_their_exact_length() {
    let mut contract = identity(
        Type::Bytes { length: 3 },
        Expr::BytesLiteral {
            bytes: vec![1, 2, 0],
        },
    );
    assert!(
        render(&contract)
            .unwrap()
            .contains("runtime::FixedBytes::new([1u8, 2u8, 0u8])")
    );
    contract.circuits[0].result = Type::Bytes { length: 2 };
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Bytes { length: 2 },
            actual: Type::Bytes { length: 3 },
        })
    );
}

#[test]
fn field_cast_requires_an_unsigned_operand() {
    let contract = identity(
        Type::Field,
        Expr::FieldCast {
            value: Box::new(Expr::Parameter {
                name: "value".into(),
            }),
        },
    );
    assert_eq!(
        render(&contract),
        Err(RenderError::ExpectedUnsigned(Type::Field))
    );
}

#[test]
fn emits_a_pure_field_circuit_as_parseable_rust() {
    let contract = identity(
        Type::Field,
        Expr::Parameter {
            name: "value".into(),
        },
    );
    let source = render(&contract).unwrap();
    syn::parse_file(&source).unwrap();
    assert!(source.contains("pub use midnight_compact_runtime as runtime;"));
    assert!(source.contains("pub fn identity("));
    assert!(source.contains("value: runtime::Field"));
    assert!(source.contains("Ok(value)"));
}

#[test]
fn cell_and_counter_declarations_emit_typed_slots() {
    let cell: Contract = serde_json::from_str(include_str!("../fixtures/cell_boolean.json"))
        .expect("Cell fixture parses");
    let source = render(&cell).unwrap();
    assert!(source.contains("pub mod ledger_slots"));
    assert!(source.contains("pub const flag: runtime::slots::CellSlot<bool>"));
    assert!(source.contains("&[0u8]"));

    let counter: Contract = serde_json::from_str(include_str!("../fixtures/counter.json"))
        .expect("Counter fixture parses");
    let source = render(&counter).unwrap();
    assert!(source.contains("pub const round: runtime::slots::CounterSlot"));
    assert!(source.contains("&[0u8]"));
}

#[test]
fn single_element_tuple_is_a_tuple_in_type_and_expression() {
    let contract = identity(
        Type::Tuple {
            elements: vec![Type::Field],
        },
        Expr::Tuple {
            elements: vec![Expr::Parameter {
                name: "value".into(),
            }],
        },
    );
    let source = render(&contract).unwrap();
    let file = syn::parse_file(&source).unwrap();
    let module = file
        .items
        .iter()
        .find_map(|item| match item {
            syn::Item::Mod(module) if module.ident == "pure_circuits" => Some(module),
            _ => None,
        })
        .expect("expected pure circuits module");
    let function = module
        .content
        .as_ref()
        .unwrap()
        .1
        .iter()
        .find_map(|item| match item {
            syn::Item::Fn(function) => Some(function),
            _ => None,
        })
        .expect("expected function");
    let syn::ReturnType::Type(_, result) = &function.sig.output else {
        panic!("expected return type")
    };
    let syn::Type::Path(result) = result.as_ref() else {
        panic!("expected Result")
    };
    let syn::PathArguments::AngleBracketed(arguments) =
        &result.path.segments.last().unwrap().arguments
    else {
        panic!("expected type arguments")
    };
    let syn::GenericArgument::Type(syn::Type::Tuple(tuple)) = &arguments.args[0] else {
        panic!("expected tuple")
    };
    assert_eq!(tuple.elems.len(), 1);
    assert!(tuple.elems.trailing_punct());
    assert!(source.contains("Ok((value,))"));
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
    contract.circuits[0].body = Expr::Parameter {
        name: "missing".into(),
    };
    assert_eq!(
        render(&contract),
        Err(RenderError::UnknownParameter("missing".into()))
    );
}

#[test]
fn rejects_wrong_native_hash_opening_and_conversion_types() {
    let field = Expr::Parameter {
        name: "value".into(),
    };
    let contract = identity(
        Type::Bytes { length: 32 },
        Expr::PersistentCommit {
            value: Box::new(field.clone()),
            opening: Box::new(field.clone()),
        },
    );
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Bytes { length: 32 },
            actual: Type::Field,
        })
    );

    let contract = identity(
        Type::Field,
        Expr::DegradeToTransient {
            value: Box::new(field),
        },
    );
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Bytes { length: 32 },
            actual: Type::Field,
        })
    );
}

#[test]
fn rejects_field_as_jubjub_point_coordinate_input() {
    let contract = identity(
        Type::Field,
        Expr::JubjubPointX {
            value: Box::new(Expr::Parameter {
                name: "value".into(),
            }),
        },
    );
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::JubjubPoint,
            actual: Type::Field,
        })
    );
}

#[test]
fn rejects_type_mismatch_and_invalid_identifier() {
    let mut contract = identity(
        Type::Boolean,
        Expr::Parameter {
            name: "value".into(),
        },
    );
    assert!(matches!(
        render(&contract),
        Err(RenderError::TypeMismatch { .. })
    ));
    contract.circuits[0].name = "bad-name".into();
    assert_eq!(
        render(&contract),
        Err(RenderError::InvalidIdentifier("bad-name".into()))
    );
}

#[test]
fn unsigned_add_and_cast_reject_field_inputs() {
    let mut contract = identity(
        Type::Unsigned { max: "511".into() },
        Expr::UnsignedAdd {
            max: "511".into(),
            left: Box::new(Expr::Parameter {
                name: "value".into(),
            }),
            right: Box::new(Expr::UnsignedLiteral {
                value: "1".into(),
                max: "255".into(),
            }),
        },
    );
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Unsigned { max: "511".into() },
            actual: Type::Field,
        })
    );
    contract.circuits[0].body = Expr::UnsignedCast {
        max: "511".into(),
        value: Box::new(Expr::Parameter {
            name: "value".into(),
        }),
    };
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Unsigned { max: "511".into() },
            actual: Type::Field,
        })
    );
}

#[test]
fn unsigned_literal_checks_the_declared_maximum() {
    let mut contract = identity(
        Type::Unsigned { max: "255".into() },
        Expr::UnsignedLiteral {
            value: "255".into(),
            max: "255".into(),
        },
    );
    assert!(
        render(&contract)
            .unwrap()
            .contains("BoundedUint::<255>::new(255u128)")
    );
    contract.circuits[0].body = Expr::UnsignedLiteral {
        value: "256".into(),
        max: "255".into(),
    };
    assert_eq!(
        render(&contract),
        Err(RenderError::InvalidUnsignedLiteral {
            value: "256".into(),
            max: "255".into(),
        })
    );
}

#[test]
fn field_literal_requires_canonical_ledger_field_value() {
    let mut contract = identity(Type::Field, Expr::FieldLiteral { value: "42".into() });
    assert!(
        render(&contract)
            .unwrap()
            .contains("runtime::Field::from(42u128)")
    );
    contract.circuits[0].body = Expr::FieldLiteral {
        value: "042".into(),
    };
    assert_eq!(
        render(&contract),
        Err(RenderError::InvalidFieldLiteral("042".into()))
    );
    contract.circuits[0].body = Expr::FieldLiteral {
        value: "340282366920938463463374607431768211456".into(),
    };
    assert!(
        render(&contract)
            .unwrap()
            .contains("runtime::Field::from_le_bytes")
    );
    contract.circuits[0].body = Expr::FieldLiteral {
        value: "115792089237316195423570985008687907853269984665640564039457584007913129639935"
            .into(),
    };
    assert_eq!(
        render(&contract),
        Err(RenderError::InvalidFieldLiteral(
            "115792089237316195423570985008687907853269984665640564039457584007913129639935".into()
        ))
    );
}

#[test]
fn pure_call_checks_target_arity_and_argument_types() {
    let mut contract = identity(
        Type::Field,
        Expr::Call {
            name: "target".into(),
            arguments: vec![Expr::Parameter {
                name: "value".into(),
            }],
        },
    );
    assert_eq!(
        render(&contract),
        Err(RenderError::UnknownCircuit("target".into()))
    );
    contract.circuits.push(PureCircuit {
        source: None,
        internal: false,
        name: "target".into(),
        parameters: vec![Parameter {
            name: "flag".into(),
            ty: Type::Boolean,
        }],
        result: Type::Field,
        body: Expr::Parameter {
            name: "flag".into(),
        },
    });
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Boolean,
            actual: Type::Field,
        })
    );
    contract.circuits[1].parameters[0].ty = Type::Field;
    if let Expr::Call { arguments, .. } = &mut contract.circuits[0].body {
        arguments.clear();
    }
    assert_eq!(
        render(&contract),
        Err(RenderError::ArgumentCount {
            circuit: "target".into(),
            expected: 1,
            actual: 0,
        })
    );
    if let Expr::Call { arguments, .. } = &mut contract.circuits[0].body {
        arguments.push(Expr::Parameter {
            name: "value".into(),
        });
    }
    assert!(
        render(&contract)
            .unwrap()
            .contains("crate::pure_circuits::target(value)")
    );
}

#[test]
fn local_binding_checks_declared_type_and_scope() {
    let mut contract = identity(
        Type::Field,
        Expr::Let {
            bindings: vec![LocalBinding {
                name: "saved".into(),
                ty: Type::Boolean,
                value: Expr::Parameter {
                    name: "value".into(),
                },
            }],
            body: Box::new(Expr::Parameter {
                name: "saved".into(),
            }),
        },
    );
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Boolean,
            actual: Type::Field,
        })
    );
    let Expr::Let { bindings, .. } = &mut contract.circuits[0].body else {
        unreachable!()
    };
    bindings[0].ty = Type::Field;
    let source = render(&contract).unwrap();
    assert!(source.contains("let __compact_local_saved: runtime::Field = value;"));
    assert!(source.contains("Ok({"));
}

#[test]
fn conditional_requires_boolean_condition_and_equal_branch_types() {
    let mut contract = identity(
        Type::Field,
        Expr::If {
            condition: Box::new(Expr::Parameter {
                name: "value".into(),
            }),
            then: Box::new(Expr::Parameter {
                name: "value".into(),
            }),
            otherwise: Box::new(Expr::Parameter {
                name: "value".into(),
            }),
        },
    );
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Boolean,
            actual: Type::Field,
        })
    );
    let Expr::If {
        condition,
        otherwise,
        ..
    } = &mut contract.circuits[0].body
    else {
        unreachable!()
    };
    **condition = Expr::Boolean { value: true };
    **otherwise = Expr::Boolean { value: false };
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Field,
            actual: Type::Boolean,
        })
    );
}

#[test]
fn refuses_to_add_a_boolean_to_a_field() {
    let contract = identity(
        Type::Field,
        Expr::Add {
            left: Box::new(Expr::Parameter {
                name: "value".into(),
            }),
            right: Box::new(Expr::Boolean { value: true }),
        },
    );
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Field,
            actual: Type::Boolean,
        })
    );
}

#[test]
fn rejects_noncanonical_or_unsupported_unsigned_maxima() {
    for max in [
        "08",
        "-1",
        "452312848583266388373324160190187140051835877600158453279131187530910662656",
    ] {
        let contract = Contract {
            schema_version: 12,
            type_aliases: vec![],
            constructor: None,
            witnesses: vec![],
            ledger_fields: vec![],
            circuits: vec![PureCircuit {
                source: None,
                internal: false,
                name: "id_u".into(),
                parameters: vec![Parameter {
                    name: "value".into(),
                    ty: Type::Unsigned { max: max.into() },
                }],
                result: Type::Unsigned { max: max.into() },
                body: Expr::Parameter {
                    name: "value".into(),
                },
            }],
            stateful_circuits: vec![],
        };
        assert_eq!(
            render(&contract),
            Err(RenderError::InvalidUnsignedMaximum(max.into()))
        );
    }
}

#[test]
fn unknown_json_fields_are_rejected() {
    let json = r#"{"schema_version":4,"ledger_fields":[],"circuits":[],"stateful_circuits":[],"rust_source":"panic!()"}"#;
    assert!(serde_json::from_str::<Contract>(json).is_err());

    let json = r#"{"schema_version":4,"ledger_fields":[{"id":"round","index":0,"declaration":{"kind":"counter"},"rust_source":"panic!()"}],"circuits":[],"stateful_circuits":[]}"#;
    assert!(serde_json::from_str::<Contract>(json).is_err());
}

#[test]
fn witness_calls_require_a_declared_witness_and_matching_signature() {
    let mut contract = Contract {
        schema_version: 12,
        type_aliases: vec![],
        constructor: None,
        ledger_fields: vec![],
        witnesses: vec![WitnessDeclaration {
            source: None,
            name: "secret".into(),
            parameters: vec![Parameter {
                name: "value".into(),
                ty: Type::Boolean,
            }],
            result: Type::Field,
        }],
        circuits: vec![],
        stateful_circuits: vec![StatefulCircuit {
            source: None,
            internal: false,
            name: "read_secret".into(),
            parameters: vec![Parameter {
                name: "flag".into(),
                ty: Type::Boolean,
            }],
            actions: vec![],
            result: Type::Field,
            return_value: StateReturn::Expression {
                value: Expr::WitnessCall {
                    name: "secret".into(),
                    arguments: vec![Expr::Parameter {
                        name: "flag".into(),
                    }],
                },
            },
        }],
    };
    let source = render(&contract).unwrap();
    syn::parse_file(&source).unwrap();
    assert!(source.contains("pub trait Witnesses<Private>"));
    assert!(source.contains("#[runtime::compact_witness_bridge]"));
    assert!(source.contains("private_transcript_outputs"));
    assert!(source.contains("pub struct Contract<W>"));
    assert!(source.contains("W: TryWitnesses<Private>"));
    assert!(source.contains("crate::ledger_contract::read_secret("));
    assert!(source.contains("&self.witnesses"));

    contract.witnesses.clear();
    assert_eq!(
        render(&contract),
        Err(RenderError::UnknownWitness("secret".into()))
    );
    contract.witnesses.push(WitnessDeclaration {
        source: None,
        name: "secret".into(),
        parameters: vec![],
        result: Type::Field,
    });
    assert_eq!(
        render(&contract),
        Err(RenderError::ArgumentCount {
            circuit: "secret".into(),
            expected: 0,
            actual: 1,
        })
    );
}

#[test]
fn witness_cell_and_counter_getters_use_declared_composite_slots() {
    let contract = Contract {
        schema_version: 12,
        type_aliases: vec![],
        constructor: None,
        witnesses: vec![WitnessDeclaration {
            source: None,
            name: "observe".into(),
            parameters: vec![],
            result: Type::Unit,
        }],
        ledger_fields: (0..16)
            .map(|position| LedgerField {
                source: None,
                id: match position {
                    0 => "flag".into(),
                    1 => "round".into(),
                    _ => format!("filler_{position}"),
                },
                index: u8::from(position != 0),
                path: if position == 0 {
                    vec![0, 0]
                } else {
                    vec![1, position - 1]
                },
                declaration: if position == 1 {
                    LedgerFieldKind::Counter
                } else {
                    LedgerFieldKind::Cell { ty: Type::Boolean }
                },
            })
            .collect(),
        circuits: vec![],
        stateful_circuits: vec![],
    };

    let source = render(&contract).unwrap();
    syn::parse_file(&source).unwrap();
    let compact = source
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect::<String>();
    assert!(compact.contains(
        "pubconstflag:runtime::slots::CellSlot<bool>=runtime::slots::CellSlot::new(&[0u8,0u8]"
    ));
    assert!(compact.contains(
        "pubconstround:runtime::slots::CounterSlot=runtime::slots::CounterSlot::new(&[1u8,0u8]"
    ));
    assert!(source.contains("crate::ledger_slots::flag.witness_read(self.meter)"));
    assert!(source.contains("crate::ledger_slots::round.witness_read(self.meter)?"));
    assert!(!source.contains("self.meter.read_cell::<"));
}

#[test]
fn witnessed_field_cell_and_nested_call_use_native_frame() {
    let mut contract = identity(Type::Unit, Expr::Unit);
    contract.circuits.clear();
    contract.ledger_fields = vec![LedgerField {
        source: None,
        id: "cell".into(),
        index: 0,
        path: vec![],
        declaration: LedgerFieldKind::Cell { ty: Type::Field },
    }];
    contract.witnesses = vec![WitnessDeclaration {
        source: None,
        name: "secret".into(),
        parameters: vec![Parameter {
            name: "seed".into(),
            ty: Type::Field,
        }],
        result: Type::Field,
    }];
    let seed = Expr::Parameter {
        name: "seed".into(),
    };
    contract.stateful_circuits = vec![
        StatefulCircuit {
            source: None,
            internal: true,
            name: "inner".into(),
            parameters: vec![Parameter {
                name: "seed".into(),
                ty: Type::Field,
            }],
            actions: vec![StateAction::Let {
                bindings: vec![LocalBinding {
                    name: "value".into(),
                    ty: Type::Field,
                    value: Expr::WitnessCall {
                        name: "secret".into(),
                        arguments: vec![seed.clone()],
                    },
                }],
                action: Box::new(StateAction::CellWrite {
                    field: "cell".into(),
                    index: 0,
                    value: Expr::Parameter {
                        name: "value".into(),
                    },
                }),
            }],
            result: Type::Unit,
            return_value: StateReturn::Unit,
        },
        StatefulCircuit {
            source: None,
            internal: false,
            name: "outer".into(),
            parameters: vec![Parameter {
                name: "seed".into(),
                ty: Type::Field,
            }],
            actions: vec![StateAction::CircuitCall {
                name: "inner".into(),
                arguments: vec![seed],
            }],
            result: Type::Unit,
            return_value: StateReturn::Unit,
        },
    ];
    let source = render(&contract).unwrap();
    syn::parse_file(&source).unwrap();
    assert!(source.contains("pub fn recording(&self) -> recorded::BorrowedContract<'_, W>"));
    assert_eq!(source.matches("CircuitFrame::new(context)").count(), 2);
    assert!(source.contains("meter: &'a runtime::context::WitnessReadMeter<'a>"));
    assert!(source.contains("crate::ledger_slots::cell.witness_read(self.meter)"));
    assert!(source.contains(".try_witness_metered(|context, meter|"));
    assert!(source.contains("#[runtime::compact_witness_bridge]"));
    assert!(!source.contains("pub trait TryWitnesses<Private>"));
    assert!(source.contains("ledger_slots::cell.write(context"));
    assert!(source.contains(".apply(|context|"));
    assert!(source.contains("self::inner(context, witnesses"));
    assert!(source.contains("Ok(frame.finish(()))"));

    let mut two_parameter_witness = contract.clone();
    two_parameter_witness.stateful_circuits[1]
        .parameters
        .push(Parameter {
            name: "extra".into(),
            ty: Type::Boolean,
        });
    let witnessed_source = render(&two_parameter_witness).unwrap();
    syn::parse_file(&witnessed_source).unwrap();
    assert!(witnessed_source.contains("pub fn outer_call<'observed, Private>("));
    assert!(witnessed_source.contains("let input = runtime::fab::AlignedValue::from(("));
    assert!(witnessed_source.contains("W: super::TryWitnesses<Private>"));

    let mut second_caller = contract.stateful_circuits[1].clone();
    second_caller.name = "outer_again".into();
    contract.stateful_circuits.push(second_caller);
    let shared = render(&contract).unwrap();
    syn::parse_file(&shared).unwrap();
    assert_eq!(
        shared.matches("fn __compact_recorded_body_inner<").count(),
        1
    );
    assert_eq!(shared.matches("__compact_recorded_body_inner").count(), 3);

    let mut name_collision = contract.stateful_circuits[1].clone();
    name_collision.name = "__compact_recorded_body_inner".into();
    contract.stateful_circuits.push(name_collision);
    let generated = syn::parse_file(&render(&contract).unwrap()).unwrap();
    let ledger = generated
        .items
        .iter()
        .find_map(|item| match item {
            syn::Item::Mod(module) if module.ident == "ledger_contract" => Some(module),
            _ => None,
        })
        .unwrap();
    let recorded = ledger
        .content
        .as_ref()
        .unwrap()
        .1
        .iter()
        .find_map(|item| match item {
            syn::Item::Mod(module) if module.ident == "recorded" => Some(module),
            _ => None,
        })
        .unwrap();
    let mut function_names = std::collections::HashSet::new();
    for item in &recorded.content.as_ref().unwrap().1 {
        if let syn::Item::Fn(function) = item {
            assert!(function_names.insert(function.sig.ident.to_string()));
        }
    }
}

#[test]
fn recorded_field_returning_helper_is_shared_across_callers() {
    let mut contract = identity(Type::Unit, Expr::Unit);
    contract.ledger_fields = vec![LedgerField {
        source: None,
        id: "cell".into(),
        index: 0,
        path: vec![],
        declaration: LedgerFieldKind::Cell { ty: Type::Field },
    }];
    contract.witnesses = vec![WitnessDeclaration {
        source: None,
        name: "secret".into(),
        parameters: vec![],
        result: Type::Field,
    }];
    let caller = |name: &str| StatefulCircuit {
        source: None,
        internal: false,
        name: name.into(),
        parameters: vec![],
        result: Type::Unit,
        return_value: StateReturn::Unit,
        actions: vec![StateAction::Let {
            bindings: vec![LocalBinding {
                name: "next".into(),
                ty: Type::Field,
                value: Expr::Call {
                    name: "innerValue".into(),
                    arguments: vec![],
                },
            }],
            action: Box::new(StateAction::CellWrite {
                field: "cell".into(),
                index: 0,
                value: Expr::Parameter {
                    name: "next".into(),
                },
            }),
        }],
    };
    contract.stateful_circuits = vec![
        StatefulCircuit {
            source: None,
            internal: true,
            name: "innerValue".into(),
            parameters: vec![],
            result: Type::Field,
            return_value: StateReturn::Expression {
                value: Expr::WitnessCall {
                    name: "secret".into(),
                    arguments: vec![],
                },
            },
            actions: vec![],
        },
        caller("outerA"),
        caller("outerB"),
    ];
    let source = render(&contract).unwrap();
    syn::parse_file(&source).unwrap();
    assert_eq!(
        source
            .matches("fn __compact_recorded_body_innerValue<")
            .count(),
        1
    );
    assert_eq!(
        source.matches("__compact_recorded_body_innerValue").count(),
        3
    );
    assert!(source.contains("pub fn outerA<Private"));
    assert!(source.contains("pub fn outerB<Private"));

    contract.stateful_circuits[0].parameters = vec![
        Parameter {
            name: "first".into(),
            ty: Type::Field,
        },
        Parameter {
            name: "second".into(),
            ty: Type::Field,
        },
    ];
    contract.stateful_circuits[0].return_value = StateReturn::Expression {
        value: Expr::Add {
            left: Box::new(Expr::WitnessCall {
                name: "secret".into(),
                arguments: vec![],
            }),
            right: Box::new(Expr::Add {
                left: Box::new(Expr::Parameter {
                    name: "first".into(),
                }),
                right: Box::new(Expr::Parameter {
                    name: "second".into(),
                }),
            }),
        },
    };
    for circuit in &mut contract.stateful_circuits[1..] {
        let StateAction::Let { bindings, .. } = &mut circuit.actions[0] else {
            unreachable!()
        };
        bindings[0].value = Expr::Call {
            name: "innerValue".into(),
            arguments: vec![
                Expr::FieldLiteral { value: "1".into() },
                Expr::FieldLiteral { value: "2".into() },
            ],
        };
    }
    let source = render(&contract).unwrap();
    syn::parse_file(&source).unwrap();
    assert_eq!(
        source
            .matches("fn __compact_recorded_body_innerValue<")
            .count(),
        1
    );
    assert_eq!(
        source.matches("__compact_recorded_body_innerValue").count(),
        3
    );
    assert!(source.contains("__compact_param_0: runtime::Field"));
    assert!(source.contains("__compact_param_1: runtime::Field"));

    // The compiler lowers `value = innerValue(1, 2) + 3` to a Field
    // action binding with an expression-nested call. Reuse the same body.
    let mut nested = contract.stateful_circuits[1].clone();
    nested.name = "outerNested".into();
    let StateAction::Let { bindings, .. } = &mut nested.actions[0] else {
        unreachable!()
    };
    bindings[0].value = Expr::Add {
        left: Box::new(Expr::Call {
            name: "innerValue".into(),
            arguments: vec![
                Expr::FieldLiteral { value: "1".into() },
                Expr::FieldLiteral { value: "2".into() },
            ],
        }),
        right: Box::new(Expr::FieldLiteral { value: "3".into() }),
    };
    contract.stateful_circuits.push(nested);
    let source = render(&contract).unwrap();
    syn::parse_file(&source).unwrap();
    assert_eq!(
        source
            .matches("fn __compact_recorded_body_innerValue<")
            .count(),
        1
    );
    assert_eq!(
        source.matches("__compact_recorded_body_innerValue").count(),
        4
    );
    assert!(source.contains("pub fn outerNested<Private"));

    contract
        .stateful_circuits
        .retain(|circuit| circuit.internal || circuit.name == "outerNested");
    let nested_only = render(&contract).unwrap();
    assert_eq!(
        nested_only
            .matches("fn __compact_recorded_body_innerValue<")
            .count(),
        1
    );
    assert_eq!(
        nested_only
            .matches("__compact_recorded_body_innerValue")
            .count(),
        2
    );

    contract.stateful_circuits[0].return_value = StateReturn::Expression {
        value: Expr::Multiply {
            left: Box::new(Expr::Parameter {
                name: "first".into(),
            }),
            right: Box::new(Expr::Parameter {
                name: "second".into(),
            }),
        },
    };
    let multiplied = render(&contract).unwrap();
    assert!(multiplied.contains("pub mod recorded"));
    assert!(multiplied.contains("__compact_recorded_product_"));

    contract.stateful_circuits[0].return_value = StateReturn::Expression {
        value: Expr::Add {
            left: Box::new(Expr::Call {
                name: "innerValue".into(),
                arguments: vec![
                    Expr::Parameter {
                        name: "first".into(),
                    },
                    Expr::Parameter {
                        name: "second".into(),
                    },
                ],
            }),
            right: Box::new(Expr::FieldLiteral { value: "1".into() }),
        },
    };
    assert!(matches!(
        render(&contract),
        Err(RenderError::UnsupportedStatefulCall(name)) if name == "innerValue"
    ));
}

#[test]
fn state_action_must_reference_the_declared_ledger_field_and_index() {
    let mut contract = Contract {
        schema_version: 12,
        type_aliases: vec![],
        constructor: None,
        witnesses: vec![],
        ledger_fields: vec![LedgerField {
            source: None,
            id: "round".into(),
            index: 0,
            path: vec![],
            declaration: LedgerFieldKind::Counter,
        }],
        circuits: vec![],
        stateful_circuits: vec![StatefulCircuit {
            source: None,
            internal: false,
            name: "increment".into(),
            parameters: vec![],
            result: Type::Unit,
            return_value: StateReturn::Unit,
            actions: vec![StateAction::CounterIncrement {
                field: "round".into(),
                index: 0,
                amount: CounterAmount::Literal { value: 1 },
            }],
        }],
    };
    let source = render(&contract).unwrap();
    assert!(source.contains("crate::ledger_slots::round.increment(context, 1)?"));
    assert!(source.contains("pub mod recorded"));
    assert!(source.contains("pub fn recording(&self) -> &recorded::Contract"));
    assert!(source.contains("&self.recording"));
    assert!(source.contains("crate::ledger_slots::round.record_increment(frame, 1u16)?"));
    assert!(source.contains("Ok(frame.finish(()))"));
    assert!(source.contains("pub fn increment_call<'observed, Private>("));
    assert!(source.contains("runtime::transaction::RecordedCall::new("));
    assert!(source.contains("\"increment\","));

    let mut recording_circuit = contract.clone();
    recording_circuit.stateful_circuits[0].name = "recording".into();
    let source = render(&recording_circuit).unwrap();
    assert!(source.contains("pub fn recording<Private>("));
    assert!(!source.contains("pub fn recording(&self) -> &recorded::Contract"));

    contract.stateful_circuits[0].actions[0] = StateAction::CounterDecrement {
        field: "round".into(),
        index: 0,
        amount: CounterAmount::Literal { value: 1 },
    };
    assert!(
        render(&contract)
            .unwrap()
            .contains("crate::ledger_slots::round.decrement(context, 1)?")
    );
    assert!(
        render(&contract)
            .unwrap()
            .contains("crate::ledger_slots::round.record_decrement(frame, 1u16)?")
    );
    contract.stateful_circuits[0].actions[0] = StateAction::CounterReset {
        field: "round".into(),
        index: 0,
    };
    assert!(
        render(&contract)
            .unwrap()
            .contains("crate::ledger_slots::round.reset(context)?")
    );
    assert!(
        render(&contract)
            .unwrap()
            .contains("crate::ledger_slots::round.record_reset(frame)?")
    );
    assert!(render(&contract).unwrap().contains("pub fn increment_call"));

    contract.stateful_circuits[0].actions[0] = StateAction::CounterReset {
        field: "round".into(),
        index: 1,
    };
    assert_eq!(
        render(&contract),
        Err(RenderError::UnknownLedgerField("round".into()))
    );
    contract.ledger_fields[0].declaration = LedgerFieldKind::Cell { ty: Type::Boolean };
    contract.stateful_circuits[0].actions[0] = StateAction::CounterReset {
        field: "round".into(),
        index: 0,
    };
    assert_eq!(
        render(&contract),
        Err(RenderError::UnknownLedgerField("round".into()))
    );
    contract.ledger_fields[0].declaration = LedgerFieldKind::Counter;

    contract.stateful_circuits[0].actions[0] = StateAction::CounterIncrement {
        field: "missing".into(),
        index: 0,
        amount: CounterAmount::Literal { value: 1 },
    };
    assert_eq!(
        render(&contract),
        Err(RenderError::UnknownLedgerField("missing".into()))
    );

    contract.stateful_circuits[0].actions[0] = StateAction::CounterIncrement {
        field: "round".into(),
        index: 1,
        amount: CounterAmount::Literal { value: 1 },
    };
    assert_eq!(
        render(&contract),
        Err(RenderError::UnknownLedgerField("round".into()))
    );

    contract.ledger_fields[0].index = 1;
    assert_eq!(render(&contract), Err(RenderError::InvalidLedgerIndex(1)));
}

#[test]
fn nested_call_exposes_a_trace_only_when_every_step_is_supported() {
    let mut contract = Contract {
        schema_version: 12,
        type_aliases: vec![],
        constructor: None,
        witnesses: vec![],
        ledger_fields: vec![LedgerField {
            source: None,
            id: "round".into(),
            index: 0,
            path: vec![],
            declaration: LedgerFieldKind::Counter,
        }],
        circuits: vec![],
        stateful_circuits: vec![
            StatefulCircuit {
                source: None,
                internal: true,
                name: "inner".into(),
                parameters: vec![],
                result: Type::Unit,
                return_value: StateReturn::Unit,
                actions: vec![StateAction::CounterReset {
                    field: "round".into(),
                    index: 0,
                }],
            },
            StatefulCircuit {
                source: None,
                internal: false,
                name: "outer".into(),
                parameters: vec![],
                result: Type::Unit,
                return_value: StateReturn::Unit,
                actions: vec![StateAction::CircuitCall {
                    name: "inner".into(),
                    arguments: vec![],
                }],
            },
        ],
    };
    let rendered = render_with_capabilities(&contract).unwrap();
    assert!(rendered.source.contains("pub mod recorded"));
    assert_eq!(rendered.capabilities.circuits.len(), 1);
    assert_eq!(rendered.capabilities.circuits[0].name, "outer");
    assert!(rendered.capabilities.circuits[0].recorded);
    assert!(rendered.capabilities.circuits[0].observed_call);

    contract.stateful_circuits[0].actions.clear();
    contract.stateful_circuits[0].result = Type::Unsigned {
        max: u64::MAX.to_string(),
    };
    contract.stateful_circuits[0].return_value = StateReturn::CounterRead {
        field: "round".into(),
        index: 0,
    };
    assert!(!render(&contract).unwrap().contains("pub mod recorded"));
}

#[test]
fn closed_pure_field_call_records_let_value_without_admitting_hashes() {
    let mut contract = Contract {
        schema_version: 12,
        type_aliases: vec![],
        constructor: None,
        witnesses: vec![],
        ledger_fields: vec![LedgerField {
            source: None,
            id: "value".into(),
            index: 0,
            path: vec![],
            declaration: LedgerFieldKind::Cell { ty: Type::Field },
        }],
        circuits: vec![
            PureCircuit {
                source: None,
                internal: false,
                name: "identityField".into(),
                parameters: vec![Parameter {
                    name: "input".into(),
                    ty: Type::Field,
                }],
                result: Type::Field,
                body: Expr::Parameter {
                    name: "input".into(),
                },
            },
            PureCircuit {
                source: None,
                internal: false,
                name: "literalField".into(),
                parameters: vec![],
                result: Type::Field,
                body: Expr::Call {
                    name: "identityField".into(),
                    arguments: vec![Expr::FieldLiteral {
                        value: "819310549611346726241370945440405716213240158234039660170669895299022906775".into(),
                    }],
                },
            },
        ],
        stateful_circuits: vec![StatefulCircuit {
            source: None,
            internal: false,
            name: "writeLiteral".into(),
            parameters: vec![],
            result: Type::Unit,
            return_value: StateReturn::Unit,
            actions: vec![StateAction::Let {
                bindings: vec![LocalBinding {
                    name: "computed".into(),
                    ty: Type::Field,
                    value: Expr::Call {
                        name: "literalField".into(),
                        arguments: vec![],
                    },
                }],
                action: Box::new(StateAction::CellWrite {
                    field: "value".into(),
                    index: 0,
                    value: Expr::Parameter {
                        name: "computed".into(),
                    },
                }),
            }],
        }],
    };
    let rendered = render_with_capabilities(&contract).unwrap();
    assert!(rendered.capabilities.circuits[0].recorded);
    assert!(rendered.capabilities.circuits[0].observed_call);
    assert!(rendered
        .source
        .contains("let __compact_recorded_pure_field_0: runtime::Field = crate::pure_circuits::literalField()?;"));
    assert!(rendered.source.contains("record_write(frame"));

    contract.stateful_circuits[0].result = Type::Field;
    contract.stateful_circuits[0].return_value = StateReturn::Expression {
        value: Expr::Call {
            name: "literalField".into(),
            arguments: vec![],
        },
    };
    let returned = render_with_capabilities(&contract).unwrap();
    assert!(returned.capabilities.circuits[0].recorded);
    assert!(returned
        .source
        .contains("let __compact_recorded_pure_return_1: runtime::Field = crate::pure_circuits::literalField()?;"));

    contract.stateful_circuits[0].actions = vec![StateAction::CellWrite {
        field: "value".into(),
        index: 0,
        value: Expr::FieldLiteral { value: "7".into() },
    }];

    contract.circuits[1].body = Expr::TransientHash {
        value: Box::new(Expr::FieldLiteral { value: "5".into() }),
    };
    let rejected = render_with_capabilities(&contract).unwrap();
    assert!(!rejected.capabilities.circuits[0].recorded);
    assert_eq!(
        rejected.capabilities.circuits[0]
            .recording_unavailable
            .as_ref()
            .unwrap()
            .ir_node,
        "StateReturn::Expression"
    );
}

#[test]
fn closed_field_pair_hash_call_records_typed_bridge_without_admitting_other_hashes() {
    let vector = Type::Vector {
        element: Box::new(Type::Field),
        length: 2,
    };
    let tuple = Type::Tuple {
        elements: vec![Type::Field, Type::Field],
    };
    let pair = Expr::Tuple {
        elements: vec![
            Expr::FieldLiteral { value: "3".into() },
            Expr::FieldLiteral { value: "4".into() },
        ],
    };
    let mut contract = Contract {
        schema_version: 12,
        type_aliases: vec![],
        constructor: None,
        witnesses: vec![],
        ledger_fields: vec![LedgerField {
            source: None,
            id: "value".into(),
            index: 0,
            path: vec![],
            declaration: LedgerFieldKind::Cell { ty: Type::Field },
        }],
        circuits: vec![
            PureCircuit {
                source: None,
                internal: false,
                name: "hashVector".into(),
                parameters: vec![Parameter {
                    name: "input".into(),
                    ty: vector.clone(),
                }],
                result: Type::Field,
                body: Expr::TransientHash {
                    value: Box::new(Expr::Parameter {
                        name: "input".into(),
                    }),
                },
            },
            PureCircuit {
                source: None,
                internal: false,
                name: "bridge".into(),
                parameters: vec![],
                result: Type::Field,
                body: Expr::Let {
                    bindings: vec![LocalBinding {
                        name: "tuple".into(),
                        ty: tuple.clone(),
                        value: pair,
                    }],
                    body: Box::new(Expr::Call {
                        name: "hashVector".into(),
                        arguments: vec![Expr::Coerce {
                            value: Box::new(Expr::Parameter {
                                name: "tuple".into(),
                            }),
                            ty: vector.clone(),
                        }],
                    }),
                },
            },
        ],
        stateful_circuits: vec![StatefulCircuit {
            source: None,
            internal: false,
            name: "writeHash".into(),
            parameters: vec![],
            result: Type::Unit,
            return_value: StateReturn::Unit,
            actions: vec![StateAction::Let {
                bindings: vec![LocalBinding {
                    name: "computed".into(),
                    ty: Type::Field,
                    value: Expr::Call {
                        name: "bridge".into(),
                        arguments: vec![],
                    },
                }],
                action: Box::new(StateAction::CellWrite {
                    field: "value".into(),
                    index: 0,
                    value: Expr::Parameter {
                        name: "computed".into(),
                    },
                }),
            }],
        }],
    };
    let rendered = render_with_capabilities(&contract).unwrap();
    assert!(rendered.capabilities.circuits[0].recorded);
    assert!(rendered.capabilities.circuits[0].observed_call);
    let body = rendered.source.split("pub mod recorded").nth(1).unwrap();
    assert!(
        body.find("pure_circuits::bridge()?").unwrap() < body.find("record_write(frame").unwrap()
    );

    contract.circuits[0].body = Expr::TransientHash {
        value: Box::new(Expr::FieldLiteral { value: "3".into() }),
    };
    let rejected = render_with_capabilities(&contract).unwrap();
    assert!(!rejected.capabilities.circuits[0].recorded);
    assert_eq!(
        rejected.capabilities.circuits[0]
            .recording_unavailable
            .as_ref()
            .unwrap()
            .ir_node,
        "Expr::Call"
    );
}

#[test]
fn typed_pair_hash_call_records_shared_unit_helper_and_conditional_caller() {
    let vector = Type::Vector {
        element: Box::new(Type::Field),
        length: 2,
    };
    let argument = Expr::Vector {
        element: Type::Field,
        elements: vec![
            Expr::FieldLiteral { value: "0".into() },
            Expr::FieldLiteral { value: "1".into() },
        ],
    };
    let call = StateAction::CircuitCall {
        name: "storeVector".into(),
        arguments: vec![argument],
    };
    let mut contract = Contract {
        schema_version: 12,
        type_aliases: vec![],
        constructor: None,
        witnesses: vec![],
        ledger_fields: vec![
            LedgerField {
                source: None,
                id: "value".into(),
                index: 0,
                path: vec![],
                declaration: LedgerFieldKind::Cell { ty: Type::Field },
            },
            LedgerField {
                source: None,
                id: "flag".into(),
                index: 1,
                path: vec![],
                declaration: LedgerFieldKind::Cell { ty: Type::Boolean },
            },
        ],
        circuits: vec![PureCircuit {
            source: None,
            internal: false,
            name: "hashVector".into(),
            parameters: vec![Parameter {
                name: "input".into(),
                ty: vector.clone(),
            }],
            result: Type::Field,
            body: Expr::TransientHash {
                value: Box::new(Expr::Parameter {
                    name: "input".into(),
                }),
            },
        }],
        stateful_circuits: vec![
            StatefulCircuit {
                source: None,
                internal: true,
                name: "storeVector".into(),
                parameters: vec![Parameter {
                    name: "v".into(),
                    ty: vector,
                }],
                result: Type::Unit,
                return_value: StateReturn::Unit,
                actions: vec![StateAction::Let {
                    bindings: vec![LocalBinding {
                        name: "hash".into(),
                        ty: Type::Field,
                        value: Expr::Call {
                            name: "hashVector".into(),
                            arguments: vec![Expr::Parameter { name: "v".into() }],
                        },
                    }],
                    action: Box::new(StateAction::CellWrite {
                        field: "value".into(),
                        index: 0,
                        value: Expr::Parameter {
                            name: "hash".into(),
                        },
                    }),
                }],
            },
            StatefulCircuit {
                source: None,
                internal: false,
                name: "bare".into(),
                parameters: vec![],
                result: Type::Unit,
                return_value: StateReturn::Unit,
                actions: vec![call.clone()],
            },
            StatefulCircuit {
                source: None,
                internal: false,
                name: "conditional".into(),
                parameters: vec![],
                result: Type::Unit,
                return_value: StateReturn::Unit,
                actions: vec![StateAction::If {
                    condition: Expr::CellRead {
                        field: "flag".into(),
                        index: 1,
                    },
                    then: Box::new(call),
                    otherwise: Box::new(StateAction::Sequence { actions: vec![] }),
                }],
            },
        ],
    };
    let rendered = render_with_capabilities(&contract).unwrap();
    assert_eq!(rendered.capabilities.circuits.len(), 2);
    assert!(
        rendered
            .capabilities
            .circuits
            .iter()
            .all(|c| c.recorded && c.observed_call)
    );
    assert!(
        rendered
            .source
            .contains("fn __compact_recorded_body_storeVector")
    );
    assert!(rendered.source.contains("pure_circuits::hashVector("));
    assert!(rendered.source.contains("__compact_recorded_pair_arg_"));

    contract.circuits[0].body = Expr::TransientHash {
        value: Box::new(Expr::FieldLiteral { value: "7".into() }),
    };
    let rejected = render_with_capabilities(&contract).unwrap();
    assert!(rejected.capabilities.circuits.iter().all(|c| !c.recorded));
    assert!(rejected.capabilities.circuits.iter().all(|c| {
        c.recording_unavailable
            .as_ref()
            .is_some_and(|gap| gap.ir_node == "Expr::Call")
    }));
}

#[test]
fn unsupported_pure_call_and_supported_field_arithmetic_have_exact_capabilities() {
    let mut contract = Contract {
        schema_version: 12,
        type_aliases: vec![],
        constructor: None,
        witnesses: vec![],
        ledger_fields: vec![LedgerField {
            source: None,
            id: "value".into(),
            index: 0,
            path: vec![],
            declaration: LedgerFieldKind::Cell { ty: Type::Field },
        }],
        circuits: vec![PureCircuit {
            source: None,
            internal: false,
            name: "square".into(),
            parameters: vec![Parameter {
                name: "input".into(),
                ty: Type::Field,
            }],
            result: Type::Field,
            body: Expr::Multiply {
                left: Box::new(Expr::Parameter {
                    name: "input".into(),
                }),
                right: Box::new(Expr::Parameter {
                    name: "input".into(),
                }),
            },
        }],
        stateful_circuits: vec![StatefulCircuit {
            source: Some(SourceLocation {
                file: "pure-call.compact".into(),
                line: 4,
                column: 1,
            }),
            internal: false,
            name: "write".into(),
            parameters: vec![
                Parameter {
                    name: "left".into(),
                    ty: Type::Field,
                },
                Parameter {
                    name: "right".into(),
                    ty: Type::Field,
                },
            ],
            result: Type::Unit,
            return_value: StateReturn::Unit,
            actions: vec![StateAction::CellWrite {
                field: "value".into(),
                index: 0,
                value: Expr::Call {
                    name: "square".into(),
                    arguments: vec![Expr::Parameter {
                        name: "left".into(),
                    }],
                },
            }],
        }],
    };
    let rendered = render_with_capabilities(&contract).unwrap();
    assert!(!rendered.source.contains("pub mod recorded"));
    assert!(!rendered.source.contains("pub fn write_call"));
    let report = serde_json::to_value(&rendered.capabilities).unwrap();
    assert_eq!(report["schema_version"], 2);
    assert_eq!(report["circuits"][0]["name"], "write");
    assert_eq!(report["circuits"][0]["source"]["file"], "pure-call.compact");
    assert_eq!(report["circuits"][0]["source"]["line"], 4);
    assert_eq!(report["circuits"][0]["recorded"], false);
    assert_eq!(report["circuits"][0]["observed_call"], false);
    assert_eq!(
        report["circuits"][0]["recording_unavailable"]["code"],
        "unsupported_expression"
    );
    assert_eq!(
        report["circuits"][0]["recording_unavailable"]["path"],
        "actions[0].value"
    );
    assert_eq!(
        report["circuits"][0]["observed_call_unavailable"]["code"],
        "recording_unavailable"
    );
    let published = render_with_proof_capabilities(
        &contract,
        &serde_json::json!({"circuits": [
            {"name": "square", "proof": false},
            {"name": "write", "proof": true}
        ]}),
    )
    .unwrap();
    assert_eq!(published.source, rendered.source);
    let published_report = serde_json::to_value(&published.capabilities).unwrap();
    assert_eq!(published_report["schema_version"], 3);
    assert_eq!(published_report["circuits"][0]["proof_required"], true);
    assert_eq!(
        published_report["circuits"][0]["recording_status"],
        "unavailable"
    );
    assert!(matches!(
        render_with_proof_capabilities(&contract, &serde_json::json!({"circuits": []})),
        Err(RenderError::ProofApplicability(_))
    ));

    let StateAction::CellWrite { value, .. } = &mut contract.stateful_circuits[0].actions[0] else {
        unreachable!()
    };
    let left = Box::new(Expr::Parameter {
        name: "left".into(),
    });
    let right = Box::new(Expr::Parameter {
        name: "right".into(),
    });
    *value = Expr::Subtract {
        left: left.clone(),
        right: right.clone(),
    };
    let rendered = render_with_capabilities(&contract).unwrap();
    assert!(rendered.source.contains("pub mod recorded"));
    assert!(rendered.source.contains("__compact_recorded_difference_0"));
    assert!(rendered.capabilities.circuits[0].recorded);
    assert!(rendered.capabilities.circuits[0].observed_call);
    let StateAction::CellWrite { value, .. } = &mut contract.stateful_circuits[0].actions[0] else {
        unreachable!()
    };
    *value = Expr::Multiply { left, right };
    let rendered = render_with_capabilities(&contract).unwrap();
    assert!(rendered.source.contains("__compact_recorded_product_0"));
    assert!(rendered.capabilities.circuits[0].recorded);
    assert!(rendered.capabilities.circuits[0].observed_call);
}

#[test]
fn standalone_unit_witness_is_recorded_only_with_its_exact_signature() {
    let mut contract = Contract {
        schema_version: 12,
        type_aliases: vec![],
        constructor: None,
        witnesses: vec![WitnessDeclaration {
            source: None,
            name: "private_increment".into(),
            parameters: vec![],
            result: Type::Unit,
        }],
        ledger_fields: vec![LedgerField {
            source: None,
            id: "round".into(),
            index: 0,
            path: vec![],
            declaration: LedgerFieldKind::Counter,
        }],
        circuits: vec![],
        stateful_circuits: vec![StatefulCircuit {
            source: None,
            internal: false,
            name: "increment".into(),
            parameters: vec![],
            result: Type::Unit,
            return_value: StateReturn::Unit,
            actions: vec![
                StateAction::CounterIncrement {
                    field: "round".into(),
                    index: 0,
                    amount: CounterAmount::Literal { value: 1 },
                },
                StateAction::Expression {
                    value: Expr::WitnessCall {
                        name: "private_increment".into(),
                        arguments: vec![],
                    },
                },
            ],
        }],
    };
    let rendered = render_with_capabilities(&contract).unwrap();
    assert!(rendered.capabilities.circuits[0].recorded);
    assert!(rendered.capabilities.circuits[0].observed_call);
    let counter = rendered
        .source
        .find("record_increment(frame, 1u16)")
        .unwrap();
    let witness = rendered.source.find(".try_witness_metered").unwrap();
    assert!(counter < witness);
    assert!(rendered.source.contains(".private_increment("));

    contract.witnesses[0].result = Type::Boolean;
    let rejected = render_with_capabilities(&contract).unwrap();
    assert!(!rejected.capabilities.circuits[0].recorded);
    assert_eq!(
        rejected.capabilities.circuits[0]
            .recording_unavailable
            .as_ref()
            .unwrap()
            .path,
        "actions[1]"
    );
    contract.witnesses[0].result = Type::Unit;
    contract.stateful_circuits[0].actions[1] = StateAction::Expression {
        value: Expr::Boolean { value: true },
    };
    assert!(
        !render_with_capabilities(&contract)
            .unwrap()
            .capabilities
            .circuits[0]
            .recorded
    );
}

#[test]
fn standalone_field_witness_records_typed_argument_but_rejects_nested_effects() {
    let mut contract = Contract {
        schema_version: 12,
        type_aliases: vec![],
        constructor: None,
        witnesses: vec![WitnessDeclaration {
            source: None,
            name: "secret".into(),
            parameters: vec![Parameter {
                name: "seed".into(),
                ty: Type::Field,
            }],
            result: Type::Field,
        }],
        ledger_fields: vec![LedgerField {
            source: None,
            id: "round".into(),
            index: 0,
            path: vec![],
            declaration: LedgerFieldKind::Counter,
        }],
        circuits: vec![],
        stateful_circuits: vec![StatefulCircuit {
            source: None,
            internal: false,
            name: "run".into(),
            parameters: vec![],
            result: Type::Unit,
            return_value: StateReturn::Unit,
            actions: vec![
                StateAction::Expression {
                    value: Expr::WitnessCall {
                        name: "secret".into(),
                        arguments: vec![Expr::FieldLiteral { value: "1".into() }],
                    },
                },
                StateAction::CounterIncrement {
                    field: "round".into(),
                    index: 0,
                    amount: CounterAmount::Literal { value: 1 },
                },
            ],
        }],
    };
    let rendered = render_with_capabilities(&contract).unwrap();
    assert!(rendered.capabilities.circuits[0].recorded);
    assert!(rendered.capabilities.circuits[0].observed_call);
    assert!(rendered.source.contains("__compact_recorded_witness_arg_0"));
    assert!(rendered.source.contains(".try_witness_metered"));

    let StateAction::Expression {
        value: Expr::WitnessCall { arguments, .. },
    } = &mut contract.stateful_circuits[0].actions[0]
    else {
        unreachable!()
    };
    arguments[0] = Expr::WitnessCall {
        name: "secret".into(),
        arguments: vec![Expr::FieldLiteral { value: "2".into() }],
    };
    let rejected = render_with_capabilities(&contract).unwrap();
    let gap = rejected.capabilities.circuits[0]
        .recording_unavailable
        .as_ref()
        .unwrap();
    assert_eq!(gap.code.as_str(), "unsupported_expression");
    assert_eq!(gap.path, "actions[0].value.arguments[0]");
}

#[test]
fn boolean_pair_hash_cell_assertion_records_read_before_counter_write() {
    let vector = Type::Vector {
        element: Box::new(Type::Field),
        length: 2,
    };
    let mut contract = Contract {
        schema_version: 12,
        type_aliases: vec![],
        constructor: None,
        witnesses: vec![],
        ledger_fields: vec![
            LedgerField {
                source: None,
                id: "value".into(),
                index: 0,
                path: vec![],
                declaration: LedgerFieldKind::Cell { ty: Type::Field },
            },
            LedgerField {
                source: None,
                id: "asserts".into(),
                index: 1,
                path: vec![],
                declaration: LedgerFieldKind::Counter,
            },
        ],
        circuits: vec![],
        stateful_circuits: vec![
            StatefulCircuit {
                source: None,
                internal: true,
                name: "different".into(),
                parameters: vec![Parameter {
                    name: "v".into(),
                    ty: vector,
                }],
                result: Type::Boolean,
                return_value: StateReturn::Expression {
                    value: Expr::NotEqual {
                        left: Box::new(Expr::TransientHash {
                            value: Box::new(Expr::Parameter { name: "v".into() }),
                        }),
                        right: Box::new(Expr::CellRead {
                            field: "value".into(),
                            index: 0,
                        }),
                    },
                },
                actions: vec![],
            },
            StatefulCircuit {
                source: None,
                internal: false,
                name: "check".into(),
                parameters: vec![],
                result: Type::Unit,
                return_value: StateReturn::Unit,
                actions: vec![
                    StateAction::Assert {
                        condition: Expr::Call {
                            name: "different".into(),
                            arguments: vec![Expr::Vector {
                                element: Type::Field,
                                elements: vec![
                                    Expr::FieldLiteral { value: "0".into() },
                                    Expr::FieldLiteral { value: "1".into() },
                                ],
                            }],
                        },
                        message: "different".into(),
                    },
                    StateAction::CounterIncrement {
                        field: "asserts".into(),
                        index: 1,
                        amount: CounterAmount::Literal { value: 1 },
                    },
                ],
            },
        ],
    };
    let rendered = render_with_capabilities(&contract).unwrap();
    assert!(rendered.capabilities.circuits[0].recorded);
    assert!(rendered.capabilities.circuits[0].observed_call);
    let source = rendered.source.split("pub mod recorded").nth(1).unwrap();
    let hash = source.find("runtime::transient_hash(").unwrap();
    let read = source.find("record_read(frame)").unwrap();
    let write = source.find("record_increment(frame").unwrap();
    assert!(hash < read && read < write);

    let StateReturn::Expression { value } = &mut contract.stateful_circuits[0].return_value else {
        unreachable!();
    };
    let Expr::NotEqual { left, .. } = value else {
        unreachable!()
    };
    **left = Expr::TransientHash {
        value: Box::new(Expr::FieldLiteral { value: "7".into() }),
    };
    let rejected = render_with_capabilities(&contract).unwrap();
    assert!(!rejected.capabilities.circuits[0].recorded);
    assert_eq!(
        rejected.capabilities.circuits[0]
            .recording_unavailable
            .as_ref()
            .unwrap()
            .ir_node,
        "StateAction::Assert"
    );
}

#[test]
fn recording_gaps_follow_the_first_definite_ir_failure() {
    let mut contract = Contract {
        schema_version: 12,
        type_aliases: vec![],
        constructor: None,
        witnesses: vec![],
        ledger_fields: vec![LedgerField {
            source: None,
            id: "round".into(),
            index: 0,
            path: vec![],
            declaration: LedgerFieldKind::Counter,
        }],
        circuits: vec![],
        stateful_circuits: vec![StatefulCircuit {
            source: None,
            internal: false,
            name: "run".into(),
            parameters: vec![],
            result: Type::Unit,
            return_value: StateReturn::Unit,
            actions: vec![StateAction::Sequence {
                actions: vec![
                    StateAction::CounterReset {
                        field: "round".into(),
                        index: 0,
                    },
                    StateAction::Expression {
                        value: Expr::Boolean { value: true },
                    },
                    StateAction::Expression {
                        value: Expr::Boolean { value: false },
                    },
                ],
            }],
        }],
    };
    let report = render_with_capabilities(&contract).unwrap().capabilities;
    let gap = report.circuits[0].recording_unavailable.as_ref().unwrap();
    assert_eq!(gap.code.as_str(), "unsupported_action");
    assert_eq!(gap.ir_node, "StateAction::Expression");
    assert_eq!(gap.path, "actions[0].actions[1]");

    contract.stateful_circuits[0].actions.clear();
    let report = render_with_capabilities(&contract).unwrap().capabilities;
    assert_eq!(
        report.circuits[0]
            .recording_unavailable
            .as_ref()
            .unwrap()
            .code
            .as_str(),
        "no_recorded_effect"
    );

    contract.stateful_circuits[0].result = Type::Boolean;
    contract.stateful_circuits[0].return_value = StateReturn::Expression {
        value: Expr::Boolean { value: true },
    };
    let report = render_with_capabilities(&contract).unwrap().capabilities;
    let gap = report.circuits[0].recording_unavailable.as_ref().unwrap();
    assert_eq!(gap.code.as_str(), "unsupported_return");
    assert_eq!(gap.ir_node, "StateReturn::Expression");
    assert_eq!(gap.path, "return_value");
}

#[test]
fn conditional_recording_rejects_an_unsupported_unselected_branch() {
    let contract = Contract {
        schema_version: 12,
        type_aliases: vec![],
        constructor: None,
        witnesses: vec![],
        ledger_fields: vec![LedgerField {
            source: None,
            id: "round".into(),
            index: 0,
            path: vec![],
            declaration: LedgerFieldKind::Counter,
        }],
        circuits: vec![],
        stateful_circuits: vec![StatefulCircuit {
            source: None,
            internal: false,
            name: "run".into(),
            parameters: vec![],
            result: Type::Unit,
            return_value: StateReturn::Unit,
            actions: vec![StateAction::If {
                condition: Expr::Boolean { value: true },
                then: Box::new(StateAction::CounterReset {
                    field: "round".into(),
                    index: 0,
                }),
                otherwise: Box::new(StateAction::Expression {
                    value: Expr::Boolean { value: false },
                }),
            }],
        }],
    };
    let report = render_with_capabilities(&contract).unwrap().capabilities;
    let capability = &report.circuits[0];
    assert!(!capability.recorded);
    assert!(!capability.observed_call);
    let gap = capability.recording_unavailable.as_ref().unwrap();
    assert_eq!(gap.code.as_str(), "unsupported_action");
    assert_eq!(gap.ir_node, "StateAction::Expression");
    assert_eq!(gap.path, "actions[0].otherwise");
}

#[test]
fn recording_gaps_include_unsupported_type_and_called_callee_path() {
    let unsupported_cell_type = Type::OpaqueString;
    let mut contract = Contract {
        schema_version: 12,
        type_aliases: vec![],
        constructor: None,
        witnesses: vec![],
        ledger_fields: vec![LedgerField {
            source: None,
            id: "small".into(),
            index: 0,
            path: vec![],
            declaration: LedgerFieldKind::Cell {
                ty: unsupported_cell_type.clone(),
            },
        }],
        circuits: vec![],
        stateful_circuits: vec![StatefulCircuit {
            source: None,
            internal: false,
            name: "set".into(),
            parameters: vec![],
            result: Type::Unit,
            return_value: StateReturn::Unit,
            actions: vec![StateAction::CellWrite {
                field: "small".into(),
                index: 0,
                value: Expr::Default {
                    ty: unsupported_cell_type,
                },
            }],
        }],
    };
    let report = render_with_capabilities(&contract).unwrap().capabilities;
    let gap = report.circuits[0].recording_unavailable.as_ref().unwrap();
    assert_eq!(gap.code.as_str(), "unsupported_type");
    assert_eq!(gap.ir_node, "StateAction::CellWrite");
    assert_eq!(gap.path, "actions[0].value");

    contract.ledger_fields[0].declaration = LedgerFieldKind::Counter;
    contract.stateful_circuits[0].name = "inner".into();
    contract.stateful_circuits[0].internal = true;
    contract.stateful_circuits[0].actions = vec![StateAction::Expression {
        value: Expr::Boolean { value: true },
    }];
    let mut outer = contract.stateful_circuits[0].clone();
    outer.name = "outer".into();
    outer.internal = false;
    outer.actions = vec![StateAction::CircuitCall {
        name: "inner".into(),
        arguments: vec![],
    }];
    contract.stateful_circuits.push(outer);
    let report = render_with_capabilities(&contract).unwrap().capabilities;
    let gap = report.circuits[0].recording_unavailable.as_ref().unwrap();
    assert_eq!(gap.code.as_str(), "unsupported_action");
    assert_eq!(gap.path, "callee[inner].actions[0]");
    assert_eq!(gap.ir_node, "StateAction::Expression");
    for circuit in report.circuits {
        assert_eq!(circuit.recorded, circuit.recording_unavailable.is_none());
        assert_eq!(
            circuit.observed_call,
            circuit.observed_call_unavailable.is_none()
        );
    }
}

#[test]
fn stateful_parameters_are_checked_before_cell_writes() {
    let mut contract = Contract {
        schema_version: 12,
        type_aliases: vec![],
        constructor: None,
        witnesses: vec![],
        ledger_fields: vec![LedgerField {
            source: None,
            id: "flag".into(),
            index: 0,
            path: vec![],
            declaration: LedgerFieldKind::Cell { ty: Type::Boolean },
        }],
        circuits: vec![],
        stateful_circuits: vec![StatefulCircuit {
            source: None,
            internal: false,
            name: "set_flag".into(),
            parameters: vec![Parameter {
                name: "value".into(),
                ty: Type::Boolean,
            }],
            result: Type::Unit,
            return_value: StateReturn::Unit,
            actions: vec![StateAction::CellWrite {
                field: "flag".into(),
                index: 0,
                value: Expr::Parameter {
                    name: "value".into(),
                },
            }],
        }],
    };
    let source = render(&contract).unwrap();
    syn::parse_file(&source).unwrap();
    assert!(source.contains("__compact_param_0: bool"));
    assert!(source.contains("crate::ledger_slots::flag.write(context, __compact_param_0)?"));

    contract.stateful_circuits[0].parameters[0].ty = Type::Field;
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Boolean,
            actual: Type::Field,
        })
    );

    contract.stateful_circuits[0].parameters[0].ty = Type::Boolean;
    let duplicate = contract.stateful_circuits[0].parameters[0].clone();
    contract.stateful_circuits[0].parameters.push(duplicate);
    assert_eq!(
        render(&contract),
        Err(RenderError::DuplicateParameter("value".into()))
    );
}

#[test]
fn counter_parameter_requires_uint16_and_a_known_name() {
    let mut contract = Contract {
        schema_version: 12,
        type_aliases: vec![],
        constructor: None,
        witnesses: vec![],
        ledger_fields: vec![LedgerField {
            source: None,
            id: "round".into(),
            index: 0,
            path: vec![],
            declaration: LedgerFieldKind::Counter,
        }],
        circuits: vec![],
        stateful_circuits: vec![StatefulCircuit {
            source: None,
            internal: false,
            name: "increment_by".into(),
            parameters: vec![Parameter {
                name: "amount".into(),
                ty: Type::Unsigned {
                    max: "65535".into(),
                },
            }],
            result: Type::Unit,
            return_value: StateReturn::Unit,
            actions: vec![StateAction::CounterIncrement {
                field: "round".into(),
                index: 0,
                amount: CounterAmount::Parameter {
                    name: "amount".into(),
                },
            }],
        }],
    };
    let source = render(&contract).unwrap();
    assert!(source.contains("__compact_param_0.value() as u16"));
    assert!(source.contains("pub fn increment_by_call<'observed, Private>("));
    assert!(source.contains("let input = runtime::fab::AlignedValue::from(amount);"));
    assert_eq!(source.matches("amount:").count(), 3);

    let mut two_parameters = contract.clone();
    two_parameters.stateful_circuits[0]
        .parameters
        .push(Parameter {
            name: "unused".into(),
            ty: Type::Boolean,
        });
    let two_parameter_source = render(&two_parameters).unwrap();
    assert!(two_parameter_source.contains("pub mod recorded"));
    assert!(two_parameter_source.contains("pub fn increment_by_call<'observed"));
    assert!(two_parameter_source.contains("let input = runtime::fab::AlignedValue::from(("));
    assert!(two_parameter_source.contains("amount, unused"));

    let mut three_parameters = two_parameters.clone();
    three_parameters.stateful_circuits[0]
        .parameters
        .push(Parameter {
            name: "another_unused".into(),
            ty: Type::Field,
        });
    let three_parameter_source = render(&three_parameters).unwrap();
    assert!(three_parameter_source.contains("pub fn increment_by_call<'observed"));
    assert!(three_parameter_source.contains("runtime::fab::AlignedValue::concat"));
    assert!(three_parameter_source.contains("AlignedValue::from(another_unused)"));

    let mut four_parameters = three_parameters.clone();
    four_parameters.stateful_circuits[0]
        .parameters
        .push(Parameter {
            name: "unused_3".into(),
            ty: Type::Field,
        });
    let four_parameter_source = render(&four_parameters).unwrap();
    assert!(!four_parameter_source.contains("clippy::too_many_arguments"));

    let mut five_parameters = four_parameters.clone();
    five_parameters.stateful_circuits[0]
        .parameters
        .push(Parameter {
            name: "unused_4".into(),
            ty: Type::Field,
        });
    let five_parameter_source = render(&five_parameters).unwrap();
    assert!(five_parameter_source.contains("clippy::too_many_arguments"));

    let mut twelve_parameters = five_parameters.clone();
    for index in 5..12 {
        twelve_parameters.stateful_circuits[0]
            .parameters
            .push(Parameter {
                name: format!("unused_{index}"),
                ty: Type::Field,
            });
    }
    let twelve_parameter_source = render(&twelve_parameters).unwrap();
    syn::parse_file(&twelve_parameter_source).unwrap();
    assert!(twelve_parameter_source.contains("pub fn increment_by_call<'observed"));
    assert!(twelve_parameter_source.contains("AlignedValue::from(unused_11)"));
    assert!(
        twelve_parameter_source
            .matches("clippy::too_many_arguments")
            .count()
            >= 4
    );

    let mut collision = two_parameters.clone();
    let mut exported = collision.stateful_circuits[0].clone();
    exported.name = "increment_by_call".into();
    exported.parameters.clear();
    exported.actions[0] = StateAction::CounterIncrement {
        field: "round".into(),
        index: 0,
        amount: CounterAmount::Literal { value: 1 },
    };
    collision.stateful_circuits.push(exported);
    let rendered = render_with_capabilities(&collision).unwrap();
    assert!(
        !rendered
            .source
            .contains("pub fn increment_by_call<'observed")
    );
    assert_eq!(
        rendered
            .capabilities
            .circuits
            .iter()
            .map(|circuit| (
                circuit.name.as_str(),
                circuit.recorded,
                circuit.observed_call
            ))
            .collect::<Vec<_>>(),
        [
            ("increment_by", true, false),
            ("increment_by_call", true, true)
        ]
    );
    let gap = rendered.capabilities.circuits[0]
        .observed_call_unavailable
        .as_ref()
        .unwrap();
    assert_eq!(gap.code.as_str(), "name_collision");
    assert_eq!(gap.path, "name");

    contract.stateful_circuits[0].parameters[0].ty = Type::Unsigned { max: "255".into() };
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Unsigned {
                max: "65535".into()
            },
            actual: Type::Unsigned { max: "255".into() },
        })
    );

    contract.stateful_circuits[0].parameters[0].ty = Type::Unsigned {
        max: "65535".into(),
    };
    contract.stateful_circuits[0].actions[0] = StateAction::CounterIncrement {
        field: "round".into(),
        index: 0,
        amount: CounterAmount::Parameter {
            name: "missing".into(),
        },
    };
    assert_eq!(
        render(&contract),
        Err(RenderError::UnknownParameter("missing".into()))
    );
}

#[test]
fn ledger_read_return_must_match_the_declared_cell() {
    let mut contract = Contract {
        schema_version: 12,
        type_aliases: vec![],
        constructor: None,
        witnesses: vec![],
        ledger_fields: vec![LedgerField {
            source: None,
            id: "flag".into(),
            index: 0,
            path: vec![],
            declaration: LedgerFieldKind::Cell { ty: Type::Boolean },
        }],
        circuits: vec![],
        stateful_circuits: vec![StatefulCircuit {
            source: None,
            internal: false,
            name: "read_flag".into(),
            parameters: vec![],
            actions: vec![],
            result: Type::Boolean,
            return_value: StateReturn::CellRead {
                field: "flag".into(),
                index: 0,
            },
        }],
    };
    let source = render(&contract).unwrap();
    assert!(source.contains("crate::ledger_slots::flag.read(context)?"));
    assert!(source.contains("let result = read_step.result"));

    contract.stateful_circuits[0].result = Type::Field;
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Field,
            actual: Type::Boolean,
        })
    );
    contract.stateful_circuits[0].result = Type::Boolean;
    contract.stateful_circuits[0].return_value = StateReturn::CellRead {
        field: "flag".into(),
        index: 1,
    };
    assert_eq!(
        render(&contract),
        Err(RenderError::UnknownLedgerField("flag".into()))
    );
}

#[test]
fn counter_read_returns_uint64() {
    let mut contract = Contract {
        schema_version: 12,
        type_aliases: vec![],
        constructor: None,
        witnesses: vec![],
        ledger_fields: vec![LedgerField {
            source: None,
            id: "round".into(),
            index: 0,
            path: vec![],
            declaration: LedgerFieldKind::Counter,
        }],
        circuits: vec![],
        stateful_circuits: vec![StatefulCircuit {
            source: None,
            internal: false,
            name: "read_round".into(),
            parameters: vec![],
            actions: vec![],
            result: Type::Unsigned {
                max: u64::MAX.to_string(),
            },
            return_value: StateReturn::CounterRead {
                field: "round".into(),
                index: 0,
            },
        }],
    };
    let source = render(&contract).unwrap();
    assert!(source.contains("crate::ledger_slots::round.read(context)?"));
    assert!(source.contains("crate::ledger_slots::round"));
    assert!(source.contains(".record_read(frame)?"));
    assert!(source.contains("pub struct Contract<W>"));
    assert!(source.contains("crate::ledger_contract::read_round(context"));

    contract.stateful_circuits[0].result = Type::Boolean;
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Unsigned {
                max: u64::MAX.to_string()
            },
            actual: Type::Boolean,
        })
    );
}

#[test]
fn set_actions_require_the_declared_element_type() {
    let mut contract = Contract {
        schema_version: 12,
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

#[test]
fn map_insert_and_lookup_require_key_and_value_types() {
    let mut contract = Contract {
        schema_version: 12,
        type_aliases: vec![],
        constructor: None,
        witnesses: vec![],
        ledger_fields: vec![LedgerField {
            source: None,
            id: "table".into(),
            index: 0,
            path: vec![],
            declaration: LedgerFieldKind::Map {
                key: Type::Boolean,
                value: Type::Field,
            },
        }],
        circuits: vec![],
        stateful_circuits: vec![StatefulCircuit {
            source: None,
            internal: false,
            name: "put".into(),
            parameters: vec![
                Parameter {
                    name: "key".into(),
                    ty: Type::Boolean,
                },
                Parameter {
                    name: "value".into(),
                    ty: Type::Field,
                },
            ],
            actions: vec![StateAction::MapInsert {
                field: "table".into(),
                index: 0,
                key: Expr::Parameter { name: "key".into() },
                value: Expr::Parameter {
                    name: "value".into(),
                },
            }],
            result: Type::Unit,
            return_value: StateReturn::Unit,
        }],
    };
    let source = render(&contract).unwrap();
    assert!(source.contains("MapSlot<bool, runtime::Field>"), "{source}");
    assert!(source.contains("pub struct PublicStateView<'a, D:"));
    assert!(source.contains("pub fn table("));
    assert!(source.contains("runtime::ledger::MapView<'a, bool, runtime::Field, D>"));
    assert!(source.contains("crate::ledger_slots::table.inspect(self.state)"));
    assert!(
        source.contains(".insert(context, __compact_param_0, __compact_param_1)?"),
        "{source}"
    );
    assert!(source.contains(".record_insert(frame,"), "{source}");
    contract.stateful_circuits[0].parameters[1].ty = Type::Boolean;
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Field,
            actual: Type::Boolean,
        })
    );
    contract.stateful_circuits[0].parameters[1].ty = Type::Field;
    contract.stateful_circuits[0].actions.clear();
    contract.stateful_circuits[0].result = Type::Field;
    contract.stateful_circuits[0].return_value = StateReturn::MapLookup {
        field: "table".into(),
        index: 0,
        key: Expr::Parameter { name: "key".into() },
    };
    let source = render(&contract).unwrap();
    assert!(source.contains(".lookup(context, __compact_param_0)?"));
    assert!(source.contains(".record_lookup(frame,"), "{source}");
    contract.stateful_circuits[0].result = Type::Boolean;
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Field,
            actual: Type::Boolean,
        })
    );
    contract.stateful_circuits[0].result = Type::Unit;
    contract.stateful_circuits[0].return_value = StateReturn::Unit;
    contract.stateful_circuits[0].actions = vec![StateAction::MapInsertDefault {
        field: "table".into(),
        index: 0,
        key: Expr::Parameter { name: "key".into() },
    }];
    let source = render(&contract).unwrap();
    assert!(source.contains(".insert_default(context, __compact_param_0)?"));
    assert!(source.contains(".record_insert_default(frame,"), "{source}");
    contract.stateful_circuits[0].parameters[0].ty = Type::Field;
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Boolean,
            actual: Type::Field,
        })
    );
    contract.stateful_circuits[0].parameters[0].ty = Type::Boolean;
    contract.stateful_circuits[0].actions = vec![StateAction::MapReset {
        field: "table".into(),
        index: 0,
    }];
    let source = render(&contract).unwrap();
    assert!(source.contains(".reset(context)?"));
    assert!(source.contains(".record_reset(frame)?"), "{source}");
    contract.stateful_circuits[0].actions.clear();
    contract.stateful_circuits[0].return_value = StateReturn::MapSize {
        field: "table".into(),
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
    contract.ledger_fields[0].declaration = LedgerFieldKind::Map {
        key: Type::Boolean,
        value: Type::Bytes { length: 32 },
    };
    contract.stateful_circuits[0].result = Type::Unit;
    contract.stateful_circuits[0].return_value = StateReturn::Unit;
    contract.stateful_circuits[0].actions = vec![StateAction::MapInsertDefault {
        field: "table".into(),
        index: 0,
        key: Expr::Parameter { name: "key".into() },
    }];
    assert!(!render(&contract).unwrap().contains("pub mod recorded"));

    // A nested ledger Map has a structural slot, but its value still has no
    // CellValue codec for scalar insert and lookup operations.
    contract.stateful_circuits.clear();
    contract.ledger_fields[0].declaration = LedgerFieldKind::Map {
        key: Type::Field,
        value: Type::LedgerMap {
            key: Box::new(Type::Field),
            value: Box::new(Type::Unsigned {
                max: u64::MAX.to_string(),
            }),
        },
    };
    let source = render(&contract).unwrap();
    assert!(source.contains("constructor_map()"));
    assert!(source.contains("MapSlot<"));
    assert!(source.contains("runtime::slots::MapNode<"));
    assert!(source.contains("runtime::BoundedUint<18446744073709551615>"));
    assert!(!source.contains("pub struct PublicStateView<'a, D:"));
}

#[test]
fn nested_list_queries_record_in_order_and_reject_wrong_shape() {
    let maybe_field = Type::Struct {
        name: "Maybe".into(),
        fields: vec![
            StructField {
                name: "is_some".into(),
                ty: Type::Boolean,
            },
            StructField {
                name: "value".into(),
                ty: Type::Field,
            },
        ],
    };
    let head = Expr::ListHead {
        field: "items".into(),
        index: 0,
        ty: maybe_field.clone(),
    };
    let mut contract = identity(Type::Unit, Expr::Unit);
    contract.ledger_fields = vec![LedgerField {
        source: None,
        id: "items".into(),
        index: 0,
        path: vec![],
        declaration: LedgerFieldKind::List { ty: Type::Field },
    }];
    contract.stateful_circuits = vec![StatefulCircuit {
        source: None,
        internal: false,
        name: "test".into(),
        parameters: vec![],
        actions: vec![
            StateAction::Assert {
                condition: Expr::ListIsEmpty {
                    field: "items".into(),
                    index: 0,
                },
                message: "empty".into(),
            },
            StateAction::Assert {
                condition: Expr::Equal {
                    left: Box::new(Expr::ListLength {
                        field: "items".into(),
                        index: 0,
                    }),
                    right: Box::new(Expr::UnsignedLiteral {
                        value: "0".into(),
                        max: u64::MAX.to_string(),
                    }),
                },
                message: "length".into(),
            },
            StateAction::Assert {
                condition: Expr::NotEqual {
                    left: Box::new(Expr::StructField {
                        value: Box::new(head.clone()),
                        field: "is_some".into(),
                        index: 0,
                    }),
                    right: Box::new(Expr::Boolean { value: true }),
                },
                message: "head absent".into(),
            },
            StateAction::ListPushFront {
                field: "items".into(),
                index: 0,
                value: Expr::FieldLiteral { value: "7".into() },
            },
            StateAction::Assert {
                condition: Expr::Equal {
                    left: Box::new(Expr::StructField {
                        value: Box::new(head),
                        field: "value".into(),
                        index: 1,
                    }),
                    right: Box::new(Expr::FieldLiteral { value: "7".into() }),
                },
                message: "head value".into(),
            },
        ],
        result: Type::Unit,
        return_value: StateReturn::Unit,
    }];
    let rendered = render_with_capabilities(&contract).unwrap();
    assert!(rendered.capabilities.circuits[0].recorded);
    assert!(rendered.capabilities.circuits[0].observed_call);
    let recorded = rendered.source.split("pub mod recorded").nth(1).unwrap();
    let empty = recorded.find("record_is_empty(frame)").unwrap();
    let length = recorded.find("record_length(frame)").unwrap();
    let head = recorded.find("record_head::<crate::types::Maybe").unwrap();
    let push = recorded.find("record_push_front(frame,").unwrap();
    assert!(empty < length && length < head && head < push);

    {
        let StateAction::Assert { condition, .. } = &mut contract.stateful_circuits[0].actions[1]
        else {
            unreachable!()
        };
        let Expr::Equal { left, .. } = condition else {
            unreachable!()
        };
        **left = Expr::ListLength {
            field: "items".into(),
            index: 1,
        };
    }
    assert!(matches!(
        render(&contract),
        Err(RenderError::UnknownLedgerField(_))
    ));

    let StateAction::Assert { condition, .. } = &mut contract.stateful_circuits[0].actions[1]
    else {
        unreachable!()
    };
    let Expr::Equal { left, .. } = condition else {
        unreachable!()
    };
    **left = Expr::ListLength {
        field: "items".into(),
        index: 0,
    };
    let StateAction::Assert { condition, .. } = &mut contract.stateful_circuits[0].actions[4]
    else {
        unreachable!()
    };
    let Expr::Equal { left, .. } = condition else {
        unreachable!()
    };
    let Expr::StructField { value, .. } = left.as_mut() else {
        unreachable!()
    };
    let Expr::ListHead { ty, .. } = value.as_mut() else {
        unreachable!()
    };
    *ty = Type::Boolean;
    assert!(matches!(
        render(&contract),
        Err(RenderError::TypeMismatch { .. })
    ));
}

#[test]
fn enum_list_head_value_records_only_with_matching_typed_head() {
    let names = Type::Enum {
        name: "Names".into(),
        variants: vec!["bill".into(), "sally".into()],
    };
    let maybe_names = Type::Struct {
        name: "Maybe".into(),
        fields: vec![
            StructField {
                name: "is_some".into(),
                ty: Type::Boolean,
            },
            StructField {
                name: "value".into(),
                ty: names.clone(),
            },
        ],
    };
    let mut contract = identity(Type::Unit, Expr::Unit);
    contract.ledger_fields = vec![LedgerField {
        source: None,
        id: "items".into(),
        index: 0,
        path: vec![],
        declaration: LedgerFieldKind::List { ty: names.clone() },
    }];
    contract.stateful_circuits = vec![StatefulCircuit {
        source: None,
        internal: false,
        name: "test".into(),
        parameters: vec![],
        actions: vec![StateAction::Assert {
            condition: Expr::Equal {
                left: Box::new(Expr::StructField {
                    value: Box::new(Expr::ListHead {
                        field: "items".into(),
                        index: 0,
                        ty: maybe_names.clone(),
                    }),
                    field: "value".into(),
                    index: 1,
                }),
                right: Box::new(Expr::EnumVariant {
                    ty: names.clone(),
                    variant: "bill".into(),
                }),
            },
            message: "head value".into(),
        }],
        result: Type::Unit,
        return_value: StateReturn::Unit,
    }];
    let rendered = render_with_capabilities(&contract).unwrap();
    assert!(rendered.capabilities.circuits[0].recorded);
    assert!(
        rendered
            .source
            .contains("record_head::<crate::types::Maybe")
    );

    let StateAction::Assert { condition, .. } = &mut contract.stateful_circuits[0].actions[0]
    else {
        unreachable!()
    };
    let Expr::Equal { left, .. } = condition else {
        unreachable!()
    };
    let Expr::StructField { value, .. } = left.as_mut() else {
        unreachable!()
    };
    let Expr::ListHead { ty, .. } = value.as_mut() else {
        unreachable!()
    };
    *ty = Type::Struct {
        name: "Maybe".into(),
        fields: vec![
            StructField {
                name: "is_some".into(),
                ty: Type::Boolean,
            },
            StructField {
                name: "value".into(),
                ty: Type::Field,
            },
        ],
    };
    assert!(matches!(
        render(&contract),
        Err(RenderError::TypeMismatch { .. })
    ));
}

#[test]
fn field_vector_list_head_value_records_without_admitting_other_vector_elements() {
    fn contract_for(element: Type) -> Contract {
        let vector = Type::Vector {
            element: Box::new(element),
            length: 4,
        };
        let maybe = Type::Struct {
            name: "Maybe".into(),
            fields: vec![
                StructField {
                    name: "is_some".into(),
                    ty: Type::Boolean,
                },
                StructField {
                    name: "value".into(),
                    ty: vector.clone(),
                },
            ],
        };
        let mut contract = identity(Type::Unit, Expr::Unit);
        contract.ledger_fields = vec![LedgerField {
            source: None,
            id: "items".into(),
            index: 0,
            path: vec![],
            declaration: LedgerFieldKind::List { ty: vector.clone() },
        }];
        contract.stateful_circuits = vec![StatefulCircuit {
            source: None,
            internal: false,
            name: "test".into(),
            parameters: vec![],
            actions: vec![StateAction::Assert {
                condition: Expr::Equal {
                    left: Box::new(Expr::StructField {
                        value: Box::new(Expr::ListHead {
                            field: "items".into(),
                            index: 0,
                            ty: maybe,
                        }),
                        field: "value".into(),
                        index: 1,
                    }),
                    right: Box::new(Expr::Default { ty: vector }),
                },
                message: "head vector".into(),
            }],
            result: Type::Unit,
            return_value: StateReturn::Unit,
        }];
        contract
    }

    let field = render_with_capabilities(&contract_for(Type::Field)).unwrap();
    assert!(field.capabilities.circuits[0].recorded);
    assert!(field.source.contains("record_head::<crate::types::Maybe"));

    let boolean = render_with_capabilities(&contract_for(Type::Boolean)).unwrap();
    assert!(!boolean.capabilities.circuits[0].recorded);
    assert!(!boolean.capabilities.circuits[0].observed_call);
}

#[test]
fn literal_bytes_list_head_records_only_with_exact_length_and_excludes_opaque() {
    fn contract_for(element: Type, value: Expr) -> Contract {
        let maybe = Type::Struct {
            name: "Maybe".into(),
            fields: vec![
                StructField {
                    name: "is_some".into(),
                    ty: Type::Boolean,
                },
                StructField {
                    name: "value".into(),
                    ty: element.clone(),
                },
            ],
        };
        let mut contract = identity(Type::Unit, Expr::Unit);
        contract.ledger_fields = vec![LedgerField {
            source: None,
            id: "items".into(),
            index: 0,
            path: vec![],
            declaration: LedgerFieldKind::List {
                ty: element.clone(),
            },
        }];
        contract.stateful_circuits = vec![StatefulCircuit {
            source: None,
            internal: false,
            name: "test".into(),
            parameters: vec![],
            actions: vec![StateAction::Let {
                bindings: vec![LocalBinding {
                    name: "expected".into(),
                    ty: element.clone(),
                    value,
                }],
                action: Box::new(StateAction::Assert {
                    condition: Expr::Equal {
                        left: Box::new(Expr::StructField {
                            value: Box::new(Expr::ListHead {
                                field: "items".into(),
                                index: 0,
                                ty: maybe,
                            }),
                            field: "value".into(),
                            index: 1,
                        }),
                        right: Box::new(Expr::Parameter {
                            name: "expected".into(),
                        }),
                    },
                    message: "head bytes".into(),
                }),
            }],
            result: Type::Unit,
            return_value: StateReturn::Unit,
        }];
        contract
    }

    let bytes = Type::Bytes { length: 4 };
    let literal = Expr::BytesLiteral {
        bytes: vec![1, 2, 3, 4],
    };
    let positive = render_with_capabilities(&contract_for(bytes.clone(), literal)).unwrap();
    assert!(positive.capabilities.circuits[0].recorded);
    assert!(positive.source.contains("__compact_recorded_bytes_"));
    assert!(
        positive
            .source
            .contains("record_head::<crate::types::Maybe")
    );

    let wrong_length = contract_for(
        bytes,
        Expr::BytesLiteral {
            bytes: vec![1, 2, 3],
        },
    );
    assert!(matches!(
        render(&wrong_length),
        Err(RenderError::TypeMismatch { .. })
    ));

    let opaque = Type::OpaqueString;
    let unsupported =
        render_with_capabilities(&contract_for(opaque.clone(), Expr::Default { ty: opaque }))
            .unwrap();
    assert!(!unsupported.capabilities.circuits[0].recorded);
    assert!(!unsupported.capabilities.circuits[0].observed_call);
}

#[test]
fn list_push_front_and_length_validate_declared_types() {
    let mut contract = Contract {
        schema_version: 12,
        type_aliases: vec![],
        constructor: None,
        witnesses: vec![],
        ledger_fields: vec![LedgerField {
            source: None,
            id: "items".into(),
            index: 0,
            path: vec![],
            declaration: LedgerFieldKind::List { ty: Type::Field },
        }],
        circuits: vec![],
        stateful_circuits: vec![StatefulCircuit {
            source: None,
            internal: false,
            name: "prepend".into(),
            parameters: vec![Parameter {
                name: "value".into(),
                ty: Type::Field,
            }],
            actions: vec![StateAction::ListPushFront {
                field: "items".into(),
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
    assert!(source.contains("ListSlot<runtime::Field>"), "{source}");
    assert!(
        source.contains("pub struct PublicStateView<'a, D:"),
        "{source}"
    );
    assert!(source.contains("pub fn items("), "{source}");
    assert!(
        source.contains("crate::ledger_slots::items.inspect(self.state)"),
        "{source}"
    );
    assert!(
        source.contains("runtime::ledger::ListView<'a, runtime::Field, D>"),
        "{source}"
    );
    let mut list_only = contract.clone();
    list_only.stateful_circuits.clear();
    let list_only_source = render(&list_only).unwrap();
    assert!(list_only_source.contains("pub struct PublicStateView<'a, D:"));
    assert!(list_only_source.contains("pub fn items("));
    assert!(
        source.contains(".push_front(context, __compact_param_0)?"),
        "{source}"
    );
    assert!(source.contains(".record_push_front(frame,"), "{source}");
    contract.stateful_circuits[0].parameters[0].ty = Type::Boolean;
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Field,
            actual: Type::Boolean,
        })
    );
    contract.stateful_circuits[0].actions.clear();
    contract.stateful_circuits[0].return_value = StateReturn::ListLength {
        field: "items".into(),
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
    let maybe_field = Type::Struct {
        name: "Maybe".into(),
        fields: vec![
            StructField {
                name: "is_some".into(),
                ty: Type::Boolean,
            },
            StructField {
                name: "value".into(),
                ty: Type::Field,
            },
        ],
    };
    contract.stateful_circuits[0].return_value = StateReturn::ListHead {
        field: "items".into(),
        index: 0,
    };
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: maybe_field.clone(),
            actual: Type::Unit,
        })
    );
    contract.stateful_circuits[0].result = maybe_field;
    let source = render(&contract).unwrap();
    assert!(
        source.contains(".head::<crate::types::Maybe, _, _>(context)?"),
        "{source}"
    );
    assert!(
        source.contains(".record_head::<crate::types::Maybe, _, _>(frame)?"),
        "{source}"
    );
    let mut second_instantiation = contract.stateful_circuits[0].result.clone();
    let Type::Struct { name, .. } = &mut second_instantiation else {
        unreachable!();
    };
    *name = "MaybeCompact1".into();
    contract.stateful_circuits[0].result = second_instantiation.clone();
    let source = render(&contract).unwrap();
    assert!(
        source.contains(".head::<crate::types::MaybeCompact1, _, _>(context)?"),
        "{source}"
    );
    assert!(
        source.contains(".record_head::<crate::types::MaybeCompact1, _, _>(frame)?"),
        "{source}"
    );
    for invalid in ["MaybeCompact", "MaybeCompactX", "MaybeOther"] {
        let mut actual = second_instantiation.clone();
        let Type::Struct { name, .. } = &mut actual else {
            unreachable!();
        };
        *name = invalid.into();
        contract.stateful_circuits[0].result = actual.clone();
        assert_eq!(
            render(&contract),
            Err(RenderError::TypeMismatch {
                expected: Type::Struct {
                    name: "Maybe".into(),
                    fields: match &second_instantiation {
                        Type::Struct { fields, .. } => fields.clone(),
                        _ => unreachable!(),
                    },
                },
                actual,
            })
        );
    }
    let mut wrong_value = second_instantiation.clone();
    let Type::Struct { fields, .. } = &mut wrong_value else {
        unreachable!();
    };
    fields[1].ty = Type::Boolean;
    contract.stateful_circuits[0].result = wrong_value.clone();
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: second_instantiation,
            actual: wrong_value,
        })
    );
}

#[test]
fn struct_definitions_are_shared_by_name_and_must_match() {
    let pair = Type::Struct {
        name: "Pair".into(),
        fields: vec![StructField {
            name: "amount".into(),
            ty: Type::Field,
        }],
    };
    let mut contract = Contract {
        schema_version: 12,
        type_aliases: vec![],
        constructor: None,
        witnesses: vec![],
        ledger_fields: vec![],
        circuits: vec![PureCircuit {
            source: None,
            internal: false,
            name: "identity".into(),
            parameters: vec![Parameter {
                name: "value".into(),
                ty: pair.clone(),
            }],
            result: pair.clone(),
            body: Expr::Parameter {
                name: "value".into(),
            },
        }],
        stateful_circuits: vec![],
    };
    let source = render(&contract).unwrap();
    assert_eq!(source.matches("pub struct Pair").count(), 1);

    contract.circuits[0].result = Type::Struct {
        name: "Pair".into(),
        fields: vec![StructField {
            name: "active".into(),
            ty: Type::Boolean,
        }],
    };
    assert_eq!(
        render(&contract),
        Err(RenderError::ConflictingStruct("Pair".into()))
    );
}

#[test]
fn stateful_call_checks_target_and_arguments() {
    let mut contract = Contract {
        schema_version: 12,
        type_aliases: vec![],
        constructor: None,
        ledger_fields: vec![],
        witnesses: vec![],
        circuits: vec![],
        stateful_circuits: vec![
            StatefulCircuit {
                source: None,
                internal: false,
                name: "target".into(),
                parameters: vec![Parameter {
                    name: "value".into(),
                    ty: Type::Field,
                }],
                actions: vec![],
                result: Type::Unit,
                return_value: StateReturn::Unit,
            },
            StatefulCircuit {
                source: None,
                internal: false,
                name: "caller".into(),
                parameters: vec![Parameter {
                    name: "seed".into(),
                    ty: Type::Field,
                }],
                actions: vec![StateAction::CircuitCall {
                    name: "target".into(),
                    arguments: vec![Expr::Parameter {
                        name: "seed".into(),
                    }],
                }],
                result: Type::Unit,
                return_value: StateReturn::Unit,
            },
        ],
    };
    assert!(
        render(&contract)
            .unwrap()
            .contains("let call_step = target(context, __compact_call_argument_0)?")
    );
    contract.stateful_circuits[1].actions = vec![StateAction::CircuitCall {
        name: "target".into(),
        arguments: vec![],
    }];
    assert_eq!(
        render(&contract),
        Err(RenderError::ArgumentCount {
            circuit: "target".into(),
            expected: 1,
            actual: 0
        })
    );
    contract.stateful_circuits[1].actions = vec![StateAction::CircuitCall {
        name: "missing".into(),
        arguments: vec![],
    }];
    assert_eq!(
        render(&contract),
        Err(RenderError::UnknownCircuit("missing".into()))
    );
    contract.stateful_circuits[0].actions = vec![StateAction::CircuitCall {
        name: "caller".into(),
        arguments: vec![Expr::Parameter {
            name: "value".into(),
        }],
    }];
    contract.stateful_circuits[1].actions = vec![StateAction::CircuitCall {
        name: "target".into(),
        arguments: vec![Expr::Parameter {
            name: "seed".into(),
        }],
    }];
    assert!(matches!(
        render(&contract),
        Err(RenderError::UnsupportedStatefulCall(_))
    ));
    let source = SourceLocation {
        file: "recursive.compact".into(),
        line: 4,
        column: 1,
    };
    contract.stateful_circuits[0].source = Some(source.clone());
    assert!(matches!(
        render(&contract),
        Err(RenderError::Located { location, error })
            if location == source && matches!(*error, RenderError::UnsupportedStatefulCall(_))
    ));
}

#[test]
fn pure_call_action_checks_arguments_and_discards_result() {
    let mut contract = identity(
        Type::Field,
        Expr::Parameter {
            name: "value".into(),
        },
    );
    contract.stateful_circuits = vec![StatefulCircuit {
        source: None,
        internal: false,
        name: "caller".into(),
        parameters: vec![Parameter {
            name: "seed".into(),
            ty: Type::Field,
        }],
        actions: vec![StateAction::PureCall {
            name: "identity".into(),
            arguments: vec![Expr::Parameter {
                name: "seed".into(),
            }],
        }],
        result: Type::Unit,
        return_value: StateReturn::Unit,
    }];
    assert!(
        render(&contract)
            .unwrap()
            .contains("crate::pure_circuits::identity(__compact_param_0)?;")
    );
    contract.stateful_circuits[0].actions = vec![StateAction::PureCall {
        name: "identity".into(),
        arguments: vec![],
    }];
    assert_eq!(
        render(&contract),
        Err(RenderError::ArgumentCount {
            circuit: "identity".into(),
            expected: 1,
            actual: 0
        })
    );
}

#[test]
fn recorded_pure_assert_call_rejects_extra_pure_steps() {
    let assertion = Expr::Assert {
        condition: Box::new(Expr::NotEqual {
            left: Box::new(Expr::Parameter {
                name: "value".into(),
            }),
            right: Box::new(Expr::FieldLiteral { value: "0".into() }),
        }),
        message: "positive".into(),
    };
    let mut contract = identity(
        Type::Unit,
        Expr::Sequence {
            steps: vec![assertion.clone()],
            value: Box::new(Expr::Unit),
        },
    );
    contract.circuits[0].name = "require_positive".into();
    contract.stateful_circuits = vec![StatefulCircuit {
        source: None,
        internal: false,
        name: "save".into(),
        parameters: vec![Parameter {
            name: "seed".into(),
            ty: Type::Field,
        }],
        actions: vec![StateAction::PureCall {
            name: "require_positive".into(),
            arguments: vec![Expr::Coerce {
                value: Box::new(Expr::Parameter {
                    name: "seed".into(),
                }),
                ty: Type::Field,
            }],
        }],
        result: Type::Unit,
        return_value: StateReturn::Unit,
    }];
    let rendered = render_with_capabilities(&contract).unwrap();
    assert!(rendered.capabilities.circuits[0].recorded);
    assert!(rendered.source.contains("pure_circuits::require_positive"));

    let Expr::Sequence { steps, .. } = &mut contract.circuits[0].body else {
        unreachable!()
    };
    steps.push(assertion);
    let rendered = render_with_capabilities(&contract).unwrap();
    assert!(!rendered.capabilities.circuits[0].recorded);
}

#[test]
fn recorded_struct_pure_guard_requires_closed_typed_call_and_continuation() {
    let policy = Type::Struct {
        name: "Policy".into(),
        fields: vec![StructField {
            name: "enabled".into(),
            ty: Type::Boolean,
        }],
    };
    let record = Type::Struct {
        name: "Record".into(),
        fields: vec![StructField {
            name: "value".into(),
            ty: Type::Field,
        }],
    };
    let uint64 = Type::Unsigned {
        max: "18446744073709551615".into(),
    };
    let parameters = vec![
        Parameter {
            name: "policy".into(),
            ty: policy,
        },
        Parameter {
            name: "record".into(),
            ty: record,
        },
        Parameter {
            name: "time".into(),
            ty: uint64,
        },
    ];
    let mut contract = identity(Type::Unit, Expr::Unit);
    contract.ledger_fields.push(LedgerField {
        source: None,
        id: "written".into(),
        index: 0,
        path: vec![],
        declaration: LedgerFieldKind::Cell { ty: Type::Field },
    });
    contract.circuits[0] = PureCircuit {
        source: None,
        internal: false,
        name: "guard".into(),
        parameters: parameters.clone(),
        result: Type::Unit,
        body: Expr::Sequence {
            steps: vec![
                Expr::Assert {
                    condition: Box::new(Expr::Boolean { value: true }),
                    message: "first".into(),
                },
                Expr::If {
                    condition: Box::new(Expr::Boolean { value: true }),
                    then: Box::new(Expr::Assert {
                        condition: Box::new(Expr::Boolean { value: true }),
                        message: "second".into(),
                    }),
                    otherwise: Box::new(Expr::Unit),
                },
            ],
            value: Box::new(Expr::Unit),
        },
    };
    contract.stateful_circuits = vec![
        StatefulCircuit {
            source: None,
            internal: true,
            name: "write".into(),
            parameters: vec![],
            actions: vec![StateAction::CellWrite {
                field: "written".into(),
                index: 0,
                value: Expr::FieldLiteral { value: "1".into() },
            }],
            result: Type::Unit,
            return_value: StateReturn::Unit,
        },
        StatefulCircuit {
            source: None,
            internal: false,
            name: "caller".into(),
            parameters: parameters.clone(),
            actions: vec![
                StateAction::PureCall {
                    name: "guard".into(),
                    arguments: parameters
                        .iter()
                        .map(|parameter| Expr::Coerce {
                            value: Box::new(Expr::Parameter {
                                name: parameter.name.clone(),
                            }),
                            ty: parameter.ty.clone(),
                        })
                        .collect(),
                },
                StateAction::CircuitCall {
                    name: "write".into(),
                    arguments: vec![],
                },
            ],
            result: Type::Unit,
            return_value: StateReturn::Unit,
        },
    ];
    let recorded = |contract: &Contract| {
        render_with_capabilities(contract)
            .unwrap()
            .capabilities
            .circuits
            .into_iter()
            .find(|circuit| circuit.name == "caller")
            .unwrap()
            .recorded
    };
    assert!(recorded(&contract));

    let closed = contract.clone();
    contract.stateful_circuits[1]
        .actions
        .push(StateAction::CellWrite {
            field: "written".into(),
            index: 0,
            value: Expr::FieldLiteral { value: "2".into() },
        });
    assert!(!recorded(&contract));
    contract = closed.clone();
    if let StateAction::PureCall { arguments, .. } = &mut contract.stateful_circuits[1].actions[0] {
        arguments[0] = Expr::Default {
            ty: parameters[0].ty.clone(),
        };
    }
    assert!(!recorded(&contract));
    contract = closed.clone();
    if let Expr::Sequence { steps, .. } = &mut contract.circuits[0].body {
        steps.push(Expr::Unit);
    }
    assert!(!recorded(&contract));
    contract = closed.clone();
    contract.stateful_circuits[0].parameters.push(Parameter {
        name: "extra".into(),
        ty: Type::Field,
    });
    if let StateAction::CircuitCall { arguments, .. } =
        &mut contract.stateful_circuits[1].actions[1]
    {
        arguments.push(Expr::FieldLiteral { value: "3".into() });
    }
    assert!(!recorded(&contract));

    // The same guard may precede precisely one compiler-emitted Uint<16>
    // literal-one Let/Counter increment. Different amounts and extra actions
    // cannot be admitted by this shortcut.
    contract = closed;
    contract.ledger_fields.push(LedgerField {
        source: None,
        id: "accepted".into(),
        index: 1,
        path: vec![],
        declaration: LedgerFieldKind::Counter,
    });
    contract.stateful_circuits[1].actions[1] = StateAction::Let {
        bindings: vec![LocalBinding {
            name: "one".into(),
            ty: Type::Unsigned {
                max: "65535".into(),
            },
            value: Expr::UnsignedLiteral {
                value: "1".into(),
                max: "65535".into(),
            },
        }],
        action: Box::new(StateAction::CounterIncrement {
            field: "accepted".into(),
            index: 1,
            amount: CounterAmount::Parameter { name: "one".into() },
        }),
    };
    let counter = contract.clone();
    assert!(recorded(&counter));
    if let StateAction::Let { bindings, .. } = &mut contract.stateful_circuits[1].actions[1] {
        bindings[0].value = Expr::UnsignedLiteral {
            value: "2".into(),
            max: "65535".into(),
        };
    }
    assert!(!recorded(&contract));
    contract = counter.clone();
    contract.stateful_circuits[1]
        .actions
        .push(StateAction::CounterReset {
            field: "accepted".into(),
            index: 1,
        });
    assert!(!recorded(&contract));
    contract = counter;
    if let StateAction::PureCall { arguments, .. } = &mut contract.stateful_circuits[1].actions[0] {
        arguments[1] = Expr::Default {
            ty: parameters[1].ty.clone(),
        };
    }
    assert!(!recorded(&contract));
}

#[test]
fn conditional_assertion_folds_only_closed_same_type_unsigned_equality() {
    let mut contract = identity(Type::Unit, Expr::Unit);
    contract.circuits.clear();
    contract.ledger_fields = vec![LedgerField {
        source: None,
        id: "fieldCell".into(),
        index: 0,
        path: vec![],
        declaration: LedgerFieldKind::Cell { ty: Type::Field },
    }];
    contract.stateful_circuits = vec![StatefulCircuit {
        source: None,
        internal: false,
        name: "checked_write".into(),
        parameters: vec![Parameter {
            name: "choose".into(),
            ty: Type::Boolean,
        }],
        actions: vec![
            StateAction::Assert {
                condition: Expr::If {
                    condition: Box::new(Expr::Parameter {
                        name: "choose".into(),
                    }),
                    then: Box::new(Expr::Equal {
                        left: Box::new(Expr::UnsignedLiteral {
                            value: "1".into(),
                            max: "2".into(),
                        }),
                        right: Box::new(Expr::UnsignedLiteral {
                            value: "2".into(),
                            max: "2".into(),
                        }),
                    }),
                    otherwise: Box::new(Expr::Boolean { value: true }),
                },
                message: "closed equality denied".into(),
            },
            StateAction::CellWrite {
                field: "fieldCell".into(),
                index: 0,
                value: Expr::FieldLiteral { value: "1".into() },
            },
        ],
        result: Type::Unit,
        return_value: StateReturn::Unit,
    }];
    let rendered = render_with_capabilities(&contract).unwrap();
    assert!(rendered.capabilities.circuits[0].recorded);
    assert!(
        rendered
            .source
            .contains("__compact_recorded_conditional_bool_0: bool = !(__compact_param_0)")
    );
    assert!(rendered.source.contains("closed equality denied"));
    assert!(rendered.source.contains("record_write(frame"));

    // An observation inside an arm cannot be evaluated ahead of the If.
    let StateAction::Assert { condition, .. } = &mut contract.stateful_circuits[0].actions[0]
    else {
        unreachable!()
    };
    let Expr::If { then, .. } = condition else {
        unreachable!()
    };
    **then = Expr::Equal {
        left: Box::new(Expr::CellRead {
            field: "fieldCell".into(),
            index: 0,
        }),
        right: Box::new(Expr::FieldLiteral { value: "1".into() }),
    };
    let report = render_with_capabilities(&contract).unwrap().capabilities;
    assert!(!report.circuits[0].recorded);
    let gap = report.circuits[0].recording_unavailable.as_ref().unwrap();
    assert_eq!(gap.ir_node, "StateAction::Assert");
    assert_eq!(gap.path, "actions[0]");
}
