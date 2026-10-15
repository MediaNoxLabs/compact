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

//! List shape validation and typed recorded head values.

use super::support::identity;
use compact_rust_backend::ir::{
    Contract, Expr, LedgerField, LedgerFieldKind, LocalBinding, Parameter, SCHEMA_VERSION,
    StateAction, StateReturn, StatefulCircuit, StructField, Type,
};
use compact_rust_backend::{RenderError, render, render_with_capabilities};

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
fn field_and_boolean_vector_heads_record_with_declared_slot_authority() {
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
    assert!(
        boolean.capabilities.circuits[0].recorded && boolean.capabilities.circuits[0].observed_call
    );
    assert!(
        boolean
            .source
            .split("pub mod recorded")
            .nth(1)
            .unwrap()
            .contains("record_head::<crate::types::Maybe")
    );
    let mut wrong_slot = contract_for(Type::Boolean);
    let StateAction::Assert {
        condition: Expr::Equal { left, .. },
        ..
    } = &mut wrong_slot.stateful_circuits[0].actions[0]
    else {
        unreachable!()
    };
    let Expr::StructField { value, .. } = left.as_mut() else {
        unreachable!()
    };
    let Expr::ListHead { index, .. } = value.as_mut() else {
        unreachable!()
    };
    *index = 99;
    assert!(render(&wrong_slot).is_err());
}

#[test]
fn bytes_and_opaque_list_heads_record_with_exact_type_and_length() {
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
    let body = super::lifecycle_recording::generated_body(&positive.source, "recorded", "test");
    // generated_body is parsed Rust printed as tokens, not rustfmt source.
    assert!(body.contains(&quote::quote!(runtime::FixedBytes<4>).to_string()));
    let literal = body
        .find(&quote::quote!(runtime::FixedBytes::new).to_string())
        .unwrap();
    let head = body
        .find(&quote::quote!(record_head::<crate::types::Maybe, _, _>).to_string())
        .unwrap();
    assert!(literal < head);
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
    let supported = render_with_capabilities(&contract_for(
        opaque.clone(),
        Expr::Default { ty: opaque.clone() },
    ))
    .unwrap();
    assert!(
        supported.capabilities.circuits[0].recorded
            && supported.capabilities.circuits[0].observed_call
    );
    assert!(
        supported
            .source
            .split("pub mod recorded")
            .nth(1)
            .unwrap()
            .contains("record_head::<crate::types::Maybe")
    );
    let wrong_type = contract_for(opaque, Expr::Boolean { value: true });
    assert!(matches!(
        render(&wrong_type),
        Err(RenderError::TypeMismatch { .. })
    ));
}

#[test]
fn list_push_front_and_length_validate_declared_types() {
    let mut contract = Contract {
        schema_version: SCHEMA_VERSION,
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
