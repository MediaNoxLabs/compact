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

//! Declared product types, names, ownership, and collection coercions.

use super::support::identity;
use compact_rust_backend::ir::{
    Contract, Expr, LedgerField, LedgerFieldKind, Parameter, PureCircuit, SCHEMA_VERSION,
    StateAction, StateReturn, StatefulCircuit, StructField, Type, TypeAlias,
};
use compact_rust_backend::{RenderError, render};

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
    assert!(source.contains("::midnight_compact_runtime::CompactCellValue"));
    assert!(source.contains("::midnight_compact_runtime::CompactEnum"));
    assert!(source.contains("pub enum Choice"));
    assert!(source.contains("RUST_RUNTIME_ABI == 51"));
    assert!(!source.contains("impl FieldRepr for Choice"));
    assert!(!source.contains("impl BinaryHashRepr for Choice"));
    assert!(!source.contains("impl FromFieldRepr for Choice"));
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
    assert!(source.contains("pub type Tag = ::midnight_compact_runtime::FixedBytes<8>;"));
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
fn struct_definitions_are_shared_by_name_and_must_match() {
    let pair = Type::Struct {
        name: "Pair".into(),
        fields: vec![StructField {
            name: "amount".into(),
            ty: Type::Field,
        }],
    };
    let mut contract = Contract {
        schema_version: SCHEMA_VERSION,
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
