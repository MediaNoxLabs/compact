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

//! Stateful action targets, parameter types, calls, and counter results.

use super::support::identity;
use compact_rust_backend::ir::{
    Contract, CounterAmount, Expr, LedgerField, LedgerFieldKind, Parameter, PureCircuit,
    SCHEMA_VERSION, SourceLocation, StateAction, StateReturn, StatefulCircuit, Type,
};
use compact_rust_backend::{RenderError, render, render_with_capabilities};

#[test]
fn state_action_must_reference_the_declared_ledger_field_and_index() {
    let mut contract = Contract {
        schema_version: SCHEMA_VERSION,
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
    let compact: String = source.chars().filter(|c| !c.is_whitespace()).collect();
    assert!(compact.contains("crate::ledger_slots::round.record_increment(frame,"));
    assert!(source.contains(".value() as u16"));
    assert!(source.contains("Ok(frame.finish("));
    assert!(source.contains("pub fn increment_call<'observed, Private>("));
    assert!(source.contains("runtime::transaction::RecordedCall::new("));
    assert!(source.contains("\"increment\","));

    for name in ["recording", "r#recording"] {
        let mut recording_circuit = contract.clone();
        recording_circuit.stateful_circuits[0].name = name.into();
        let source = render(&recording_circuit).unwrap();
        assert!(source.contains(&format!("pub fn {name}<Private>(")));
        assert!(!source.contains("pub fn recording(&self) -> &recorded::Contract"));
    }

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
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect::<String>()
            .contains("crate::ledger_slots::round.record_decrement(frame,")
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
fn stateful_parameters_are_checked_before_cell_writes() {
    let mut contract = Contract {
        schema_version: SCHEMA_VERSION,
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
        schema_version: SCHEMA_VERSION,
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

    // A Rust raw spelling is accepted only through the typed IR.
    for collision_name in [
        "increment_by_call",
        "increment_by$call",
        "r#increment_by_call",
    ] {
        let mut collision = two_parameters.clone();
        let mut exported = collision.stateful_circuits[0].clone();
        exported.name = collision_name.into();
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
            [("increment_by", true, false), (collision_name, true, true)]
        );
        let gap = rendered.capabilities.circuits[0]
            .observed_call_unavailable
            .as_ref()
            .unwrap();
        assert_eq!(gap.code.as_str(), "name_collision");
        assert_eq!(gap.path, "name");
    }

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
        schema_version: SCHEMA_VERSION,
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
        schema_version: SCHEMA_VERSION,
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
fn stateful_call_checks_target_and_arguments() {
    let mut contract = Contract {
        schema_version: SCHEMA_VERSION,
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
fn counter_less_than_requires_typed_counter_slots_and_thresholds() {
    let mut contract: Contract =
        serde_json::from_str(include_str!("../counter-less-than-schema15-ir.json")).unwrap();
    contract.schema_version = SCHEMA_VERSION;
    let rendered = render_with_capabilities(&contract).unwrap();
    assert_eq!(rendered.capabilities.circuits.len(), 4);
    assert!(
        rendered
            .capabilities
            .circuits
            .iter()
            .all(|circuit| circuit.recorded && circuit.observed_call)
    );
    assert!(rendered.source.contains(".record_less_than("));
    let first = contract
        .stateful_circuits
        .iter()
        .position(|circuit| circuit.name == "compare")
        .unwrap();
    let mutate = |value: Expr| {
        let mut wrong = contract.clone();
        wrong.stateful_circuits[first].return_value = StateReturn::Expression { value };
        assert!(render_with_capabilities(&wrong).is_err());
    };
    mutate(Expr::CounterLessThan {
        field: "low".into(),
        index: 0,
        threshold: Box::new(Expr::Boolean { value: false }),
    });
    mutate(Expr::CounterLessThan {
        field: "low".into(),
        index: 1,
        threshold: Box::new(Expr::Parameter {
            name: "threshold".into(),
        }),
    });
    mutate(Expr::CounterLessThan {
        field: "missing".into(),
        index: 0,
        threshold: Box::new(Expr::Parameter {
            name: "threshold".into(),
        }),
    });
    let mut wrong = contract.clone();
    wrong.ledger_fields[0].declaration = LedgerFieldKind::Cell { ty: Type::Field };
    assert!(render_with_capabilities(&wrong).is_err());
    let mut old_schema = contract.clone();
    old_schema.schema_version = 14;
    assert!(render_with_capabilities(&old_schema).is_err());
    let mut pure = contract.clone();
    pure.circuits.push(PureCircuit {
        name: "invalid_pure".into(),
        source: None,
        internal: false,
        parameters: vec![],
        result: Type::Boolean,
        body: Expr::CounterLessThan {
            field: "low".into(),
            index: 0,
            threshold: Box::new(Expr::UnsignedLiteral {
                value: "4".into(),
                max: u64::MAX.to_string(),
            }),
        },
    });
    assert!(render_with_capabilities(&pure).is_err());
}
