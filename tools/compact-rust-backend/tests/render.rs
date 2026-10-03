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
    LocalBinding, Parameter, PureCircuit, SourceLocation, StateAction, StateReturn,
    StatefulCircuit, StructField, Type, TypeAlias, WitnessDeclaration,
};
use compact_rust_backend::{RenderError, render};

fn identity(result: Type, body: Expr) -> Contract {
    Contract {
        schema_version: 8,
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
    assert!(source.contains("RUST_RUNTIME_ABI == 18"));
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
    assert!(source.contains("crate::ledger_slots::plain.insert(context, __compact_param_0)?"));
    assert!(source.contains("crate::ledger_slots::plain.is_full(context)?"));
    assert!(source.contains("crate::ledger_slots::historic.reset_history(context)?"));
    assert!(source.contains("crate::ledger_slots::historic.is_full(context)?"));
    assert!(!source.contains("context.merkle_insert(0,"));
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
    assert!(render(&contract).unwrap().contains("amount.clone()"));
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
    assert!(source.contains("__compact_constructor_value_0.clone()"));
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
    assert!(source.contains("context.insert_set(0, (true).clone())?"));
    assert!(source.contains("context.remove_set(0, (false).clone())?"));
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
        *item = Box::new(Expr::Boolean { value: true });
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
            .contains("crate::pure_circuits::target(value)?")
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
    *condition = Box::new(Expr::Boolean { value: true });
    *otherwise = Box::new(Expr::Boolean { value: false });
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
            schema_version: 8,
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
        schema_version: 8,
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
        schema_version: 8,
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
    assert!(!render(&contract).unwrap().contains("pub mod recorded"));

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
        schema_version: 8,
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
    assert!(!render(&contract).unwrap().contains("pub mod recorded"));

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
fn unsupported_nested_call_does_not_expose_an_incomplete_trace() {
    let mut contract = Contract {
        schema_version: 8,
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
    assert!(!render(&contract).unwrap().contains("pub mod recorded"));

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
fn unsupported_field_expression_does_not_expose_a_recorded_call() {
    let mut contract = Contract {
        schema_version: 8,
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
        circuits: vec![],
        stateful_circuits: vec![StatefulCircuit {
            source: None,
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
                value: Expr::Multiply {
                    left: Box::new(Expr::Parameter {
                        name: "left".into(),
                    }),
                    right: Box::new(Expr::Parameter {
                        name: "right".into(),
                    }),
                },
            }],
        }],
    };
    assert!(!render(&contract).unwrap().contains("pub mod recorded"));

    let StateAction::CellWrite { value, .. } = &mut contract.stateful_circuits[0].actions[0] else {
        unreachable!()
    };
    let Expr::Multiply { left, right } = value.clone() else {
        unreachable!()
    };
    *value = Expr::Add { left, right };
    assert!(render(&contract).unwrap().contains("pub mod recorded"));
}

#[test]
fn stateful_parameters_are_checked_before_cell_writes() {
    let mut contract = Contract {
        schema_version: 8,
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
        schema_version: 8,
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
        schema_version: 8,
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
        schema_version: 8,
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
        schema_version: 8,
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
        schema_version: 8,
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
}

#[test]
fn list_push_front_and_length_validate_declared_types() {
    let mut contract = Contract {
        schema_version: 8,
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
        schema_version: 8,
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
        schema_version: 8,
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
