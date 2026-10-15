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

//! Native private outputs and witness-driven execution frames.

use super::support::identity;
use compact_rust_backend::ir::{
    Contract, Expr, LedgerField, LedgerFieldKind, LocalBinding, NativeWitnessBuiltin, Parameter,
    SCHEMA_VERSION, StateAction, StateReturn, StatefulCircuit, Type, WitnessDeclaration,
};
use compact_rust_backend::{RenderError, render, render_with_capabilities};

#[test]
fn native_own_public_key_is_a_private_effect_without_a_user_witness() {
    let contract = Contract {
        schema_version: SCHEMA_VERSION,
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
        schema_version: SCHEMA_VERSION,
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
        schema_version: SCHEMA_VERSION,
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
fn witness_calls_require_a_declared_witness_and_matching_signature() {
    let mut contract = Contract {
        schema_version: SCHEMA_VERSION,
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
    assert!(!source.contains("::core::convert::From<&'a super::Contract<W>>"));
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

    // `$` is valid Compact syntax; `r#` cases exercise the typed IR only.
    for name in ["recording", "r#recording"] {
        let mut collision = contract.clone();
        collision.stateful_circuits[1].name = name.into();
        let rendered = render(&collision).unwrap();
        assert!(!rendered.contains("pub fn recording(&self)"), "{name}");
        assert!(rendered.contains(&format!("pub fn {name}<Private>(")));
        assert!(rendered.contains("pub recording: recorded::Contract"));
        assert!(rendered.contains("::core::convert::From<&'a super::Contract<W>>"));
        assert!(rendered.contains("witnesses: &contract.witnesses"));

        // A private-IR circuit named `from` (a Compact keyword) cannot prevent
        // fully qualified trait construction of the witnessed handle.
        let mut from = collision.stateful_circuits[1].clone();
        from.name = "from".into();
        collision.stateful_circuits.push(from);
        let rendered = render(&collision).unwrap();
        syn::parse_file(&rendered).unwrap();
        assert!(rendered.contains("pub fn from<Private>("));
        assert!(rendered.contains("::core::convert::From<&'a super::Contract<W>>"));
    }
    for name in ["outer_call", "outer$call", "r#outer_call"] {
        let mut collision = contract.clone();
        let mut exported = collision.stateful_circuits[1].clone();
        exported.name = name.into();
        collision.stateful_circuits.push(exported);
        let rendered = render_with_capabilities(&collision).unwrap();
        assert!(!rendered.source.contains("pub fn outer_call<'observed"));
        assert!(!rendered.capabilities.circuits[0].observed_call);
        assert_eq!(
            rendered.capabilities.circuits[0]
                .observed_call_unavailable
                .as_ref()
                .unwrap()
                .code
                .as_str(),
            "name_collision"
        );
        assert!(rendered.capabilities.circuits[1].observed_call);
    }

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
        0
    );
    // Scalar Unit roots now inline the checked helper body. Verify each real
    // caller retains its witnessed write, independently of the shared symbol.
    for caller in ["outer", "outer_again"] {
        let body = super::lifecycle_recording::generated_body(&shared, "recorded", caller);
        assert_eq!(body.matches("try_witness_metered").count(), 1, "{body}");
        assert_eq!(body.matches("record_write").count(), 1, "{body}");
        assert!(body.find("try_witness_metered").unwrap() < body.find("record_write").unwrap());
    }

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
