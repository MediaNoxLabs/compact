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

//! Typed branch composition and remaining unsupported conditional operands.

use super::support::identity;
use compact_rust_backend::ir::{
    Contract, Expr, LedgerField, LedgerFieldKind, LocalBinding, Parameter, SCHEMA_VERSION,
    StateAction, StateReturn, StatefulCircuit, StructField, Type, WitnessDeclaration,
};
use compact_rust_backend::render_with_capabilities;

// prettyplease may wrap generic arguments and add a trailing comma.
fn compact_generic_syntax(source: &str) -> String {
    source
        .split_whitespace()
        .collect::<String>()
        .replace(",>", ">")
}

#[test]
fn recorded_nested_unsigned_projection_composes_widths_and_effects() {
    let mut contract: Contract = serde_json::from_str(include_str!(
        "../../fixtures/recorded-nested-uint64-cell-counter.json"
    ))
    .unwrap();
    contract.schema_version = SCHEMA_VERSION;
    let rendered = render_with_capabilities(&contract).unwrap();
    assert!(rendered.capabilities.circuits[0].recorded);
    let recorded = rendered.source.split_once("pub mod recorded").unwrap().1;
    assert!(compact_generic_syntax(recorded).contains("cast_unsigned::<4,18446744073709551615>"));
    assert!(recorded.find("record_write").unwrap() < recorded.find("record_increment").unwrap());

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
        render_with_capabilities(&contract)
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
        render_with_capabilities(&contract)
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
        render_with_capabilities(&contract)
            .unwrap()
            .capabilities
            .circuits[0]
            .recorded
    );
}

#[test]
fn recorded_conditional_field_pair_preserves_native_predicate_and_arm_validation() {
    let mut contract: Contract = serde_json::from_str(include_str!(
        "../../fixtures/recorded-closed-conditional-field-pair.json"
    ))
    .unwrap();
    contract.schema_version = SCHEMA_VERSION;
    let rendered = render_with_capabilities(&contract).unwrap();
    assert!(rendered.capabilities.circuits[0].recorded);
    // Check the emitted conditional, independently of a retired temporary name.
    let body = super::lifecycle_recording::generated_body(
        &rendered.source,
        "recorded",
        &contract.stateful_circuits[0].name,
    );
    assert!(body.contains("if "), "{body}");

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
fn recorded_annotated_uint8_composes_recorded_predicates_and_arms() {
    let mut contract: Contract = serde_json::from_str(include_str!(
        "../../fixtures/recorded-annotated-uint8-conditional.json"
    ))
    .unwrap();
    contract.schema_version = SCHEMA_VERSION;
    let rendered = render_with_capabilities(&contract).unwrap();
    assert!(rendered.capabilities.circuits[0].recorded);
    let recorded = rendered.source.split_once("pub mod recorded").unwrap().1;
    assert!(compact_generic_syntax(recorded).contains("cast_unsigned::<2,255>"));
    assert!(recorded.contains("record_write"));
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
        render_with_capabilities(&contract)
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
        render_with_capabilities(&contract)
            .unwrap()
            .capabilities
            .circuits[0]
            .recorded
    );
}

#[test]
fn recorded_curve_argument_composes_effectful_predicates_and_lazy_witness_arms() {
    let mut contract: Contract = serde_json::from_str(include_str!(
        "../../fixtures/recorded-closed-curve-argument.json"
    ))
    .unwrap();
    contract.schema_version = SCHEMA_VERSION;
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
    let rendered = render_with_capabilities(&contract).unwrap();
    assert!(rendered.capabilities.circuits[0].recorded);
    assert!(rendered.capabilities.circuits[0].observed_call);
    let body = super::lifecycle_recording::generated_body(
        &rendered.source,
        "recorded",
        &contract.stateful_circuits[0].name,
    );
    assert_eq!(body.matches("record_read").count(), 1);
    assert!(body.find("record_read").unwrap() < body.find("hash_to_curve").unwrap());

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
    let rendered = render_with_capabilities(&contract).unwrap();
    assert!(rendered.capabilities.circuits[0].recorded);
    assert!(rendered.capabilities.circuits[0].observed_call);
    use syn::visit_mut::VisitMut;
    #[derive(Default)]
    struct WitnessBranch {
        depth: usize,
        calls: usize,
    }
    impl VisitMut for WitnessBranch {
        fn visit_expr_if_mut(&mut self, node: &mut syn::ExprIf) {
            self.visit_expr_mut(&mut node.cond);
            self.depth += 1;
            self.visit_block_mut(&mut node.then_branch);
            if let Some((_, otherwise)) = &mut node.else_branch {
                self.visit_expr_mut(otherwise);
            }
            self.depth -= 1;
        }
        fn visit_expr_method_call_mut(&mut self, node: &mut syn::ExprMethodCall) {
            if node.method == "dynamicArm" {
                assert!(self.depth > 0);
                self.calls += 1;
            }
            syn::visit_mut::visit_expr_method_call_mut(self, node);
        }
    }
    let body = super::lifecycle_recording::generated_body(
        &rendered.source,
        "recorded",
        &contract.stateful_circuits[0].name,
    );
    let mut visitor = WitnessBranch::default();
    visitor.visit_block_mut(&mut syn::parse_str::<syn::Block>(&body).unwrap());
    assert_eq!(
        visitor.calls, 1,
        "the witness remains once inside the selected branch"
    );
    contract.witnesses[0].result = Type::Boolean;
    assert!(
        render_with_capabilities(&contract).is_err(),
        "witness result types remain authoritative"
    );
}

#[test]
fn recorded_nested_uint4_composes_typed_arithmetic_and_witness_arms() {
    let mut contract: Contract =
        serde_json::from_str(include_str!("../../fixtures/recorded-nested-uint4.json")).unwrap();
    contract.schema_version = SCHEMA_VERSION;
    let rendered = render_with_capabilities(&contract).unwrap();
    assert!(rendered.capabilities.circuits[0].recorded);
    assert!(rendered.source.contains("BoundedUint::<4>::new"));
    let recorded = rendered.source.split_once("pub mod recorded").unwrap().1;
    assert!(recorded.contains("if "));
    assert!(recorded.contains("record_write"));

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
        render_with_capabilities(&contract)
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
        render_with_capabilities(&contract)
            .unwrap()
            .capabilities
            .circuits[0]
            .recorded
    );
    // Small unsigned multiplication now composes inside the selected arm.
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
    **then = Expr::UnsignedMultiply {
        max: "2".into(),
        left: Box::new(Expr::UnsignedLiteral {
            value: "1".into(),
            max: "2".into(),
        }),
        right: Box::new(Expr::UnsignedLiteral {
            value: "1".into(),
            max: "2".into(),
        }),
    };
    let multiplied = render_with_capabilities(&contract).unwrap();
    assert!(multiplied.capabilities.circuits[0].recorded);
    let body = super::lifecycle_recording::generated_body(
        &multiplied.source,
        "recorded",
        &contract.stateful_circuits[0].name,
    );
    let multiply = body.find("multiply_unsigned").unwrap();
    assert!(body[..multiply].matches("if ").count() >= 2, "{body}");
    assert!(multiply < body.find("record_write").unwrap());
    assert!(compact_generic_syntax(&body).contains("multiply_unsigned::<2,2,2>"));

    // An unselected branch is still type checked: expanding the arithmetic
    // vocabulary does not permit a Boolean where this unsigned arm is required.
    let StateAction::Let { bindings, .. } = &mut contract.stateful_circuits[0].actions[0] else {
        unreachable!()
    };
    let Expr::If {
        condition, then, ..
    } = &mut bindings[0].value
    else {
        unreachable!()
    };
    **condition = Expr::Boolean { value: false };
    let Expr::Coerce { value, .. } = then.as_mut() else {
        unreachable!()
    };
    let Expr::UnsignedCast { value, .. } = value.as_mut() else {
        unreachable!()
    };
    let Expr::If { then, .. } = value.as_mut() else {
        unreachable!()
    };
    **then = Expr::UnsignedMultiply {
        max: "2".into(),
        left: Box::new(Expr::Boolean { value: true }),
        right: Box::new(Expr::UnsignedLiteral {
            value: "1".into(),
            max: "2".into(),
        }),
    };
    assert!(render_with_capabilities(&contract).is_err());
}

#[test]
fn unsigned_ternary_comparison_records_typed_literal_and_parameter_arms() {
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
    assert!(rendered.capabilities.circuits[0].recorded);
    // An unselected branch must still have a valid parameter binding.
    contract.stateful_circuits[0]
        .parameters
        .retain(|p| p.name != "y");
    assert!(matches!(
        render_with_capabilities(&contract),
        Err(compact_rust_backend::RenderError::UnknownParameter(name)) if name == "y"
    ));
}

#[test]
fn ternary_struct_member_records_typed_literal_and_parameter_values() {
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
    let body = super::lifecycle_recording::generated_body(&rendered.source, "recorded", "test");
    assert!(body.contains("record_write") && body.contains("if "));
    let literal_body = body;

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
    assert!(
        rendered.capabilities.circuits[0].recorded
            && rendered.capabilities.circuits[0].observed_call
    );
    let body = super::lifecycle_recording::generated_body(&rendered.source, "recorded", "test");
    // Generated parameters use positional Rust identifiers, not source names.
    // Only the branch input changed: require the emitted body to reflect it.
    assert_ne!(body, literal_body);
    assert!(body.contains("if ") && body.contains("record_write"));
}
