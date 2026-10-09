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

//! Closed struct constructors, field guards, and vector-map shapes.

use super::support::identity;
use compact_rust_backend::ir::{
    Contract, CounterAmount, Expr, LedgerField, LedgerFieldKind, LocalBinding, Parameter,
    PureCircuit, SCHEMA_VERSION, StateAction, StateReturn, StatefulCircuit, StructField, Type,
};
use compact_rust_backend::{RenderError, render_with_capabilities};

#[test]
fn recorded_distinct_struct_constructor_calls_preserve_identity_and_typed_composition() {
    let fixture = || -> Contract {
        let mut contract: Contract = serde_json::from_str(include_str!(
            "../../fixtures/recorded-struct-constructor-cells.json"
        ))
        .unwrap();
        contract.schema_version = SCHEMA_VERSION;
        contract
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
    // Writing the same-typed root Field directly is valid even though the
    // legacy constructor recipe required a projection.
    let direct = render_with_capabilities(&wrong_write).unwrap();
    assert!(direct.capabilities.circuits[0].recorded);
    let body = super::lifecycle_recording::generated_body(
        &direct.source,
        "recorded",
        &wrong_write.stateful_circuits[0].name,
    );
    assert!(body.contains("record_write"));
    assert!(body.find("pure_circuits :: makeAlpha").unwrap() < body.find("record_write").unwrap());

    let mut effectful_helper = fixture();
    let previous = effectful_helper.circuits[0].body.clone();
    effectful_helper.circuits[0].body = Expr::Sequence {
        steps: vec![Expr::Assert {
            condition: Box::new(Expr::Boolean { value: true }),
            message: "side condition".into(),
        }],
        value: Box::new(previous),
    };
    // A pure assertion is a checked native pure operation, not a ledger effect.
    let asserted = render_with_capabilities(&effectful_helper).unwrap();
    assert!(asserted.capabilities.circuits[0].recorded);
    let pure = super::lifecycle_recording::generated_body(
        &asserted.source,
        "pure_circuits",
        &effectful_helper.circuits[0].name,
    );
    assert!(pure.contains("side condition"));
    let recorded = super::lifecycle_recording::generated_body(
        &asserted.source,
        "recorded",
        &effectful_helper.stateful_circuits[0].name,
    );
    assert!(
        recorded.find("pure_circuits :: makeAlpha").unwrap()
            < recorded.find("record_write").unwrap()
    );
    let StateAction::Let { action, .. } = &mut wrong_write.stateful_circuits[0].actions[0] else {
        unreachable!()
    };
    let StateAction::Let { action, .. } = action.as_mut() else {
        unreachable!()
    };
    let StateAction::CellWrite { value, .. } = action.as_mut() else {
        unreachable!()
    };
    *value = Expr::Boolean { value: true };
    assert!(render_with_capabilities(&wrong_write).is_err());
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
fn identity_vector_map_recording_requires_closed_literals_and_exact_shape() {
    let element = Type::Unsigned {
        max: u64::MAX.to_string(),
    };
    let vector = Type::Vector {
        element: Box::new(element.clone()),
        length: 3,
    };
    let literal = || Expr::UnsignedLiteral {
        value: "0".into(),
        max: u64::MAX.to_string(),
    };
    let mut contract = identity(Type::Unit, Expr::Unit);
    contract.ledger_fields = vec![LedgerField {
        source: None,
        id: "values".into(),
        index: 0,
        path: vec![],
        declaration: LedgerFieldKind::Cell { ty: vector.clone() },
    }];
    contract.stateful_circuits = vec![StatefulCircuit {
        source: None,
        internal: false,
        name: "write_map".into(),
        parameters: vec![],
        actions: vec![StateAction::Let {
            bindings: vec![LocalBinding {
                name: "mapped".into(),
                ty: vector,
                value: Expr::VectorMap {
                    parameter: Parameter {
                        name: "item".into(),
                        ty: element.clone(),
                    },
                    source: Box::new(Expr::Tuple {
                        elements: vec![literal(), literal(), literal()],
                    }),
                    body: Box::new(Expr::Parameter {
                        name: "item".into(),
                    }),
                    result: element,
                    length: 3,
                },
            }],
            action: Box::new(StateAction::CellWrite {
                field: "values".into(),
                index: 0,
                value: Expr::Parameter {
                    name: "mapped".into(),
                },
            }),
        }],
        result: Type::Unit,
        return_value: StateReturn::Unit,
    }];
    let rendered = render_with_capabilities(&contract).unwrap();
    assert!(rendered.capabilities.circuits[0].recorded);
    assert!(rendered.capabilities.circuits[0].observed_call);
    let mut nonidentity = contract.clone();
    let StateAction::Let { bindings, .. } = &mut nonidentity.stateful_circuits[0].actions[0] else {
        unreachable!()
    };
    let Expr::VectorMap { body, .. } = &mut bindings[0].value else {
        unreachable!()
    };
    **body = literal();
    assert!(
        !render_with_capabilities(&nonidentity)
            .unwrap()
            .capabilities
            .circuits[0]
            .recorded
    );
    let StateAction::Let { bindings, .. } = &mut contract.stateful_circuits[0].actions[0] else {
        unreachable!()
    };
    let Expr::VectorMap { length, .. } = &mut bindings[0].value else {
        unreachable!()
    };
    *length = 2;
    assert!(render_with_capabilities(&contract).is_err());
}
