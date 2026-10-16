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

//! Pure Rust expression shape and evaluation order.

use super::support::identity;
use compact_rust_backend::ir::{
    Expr, LocalBinding, Parameter, PureCircuit, StateAction, StateReturn, StatefulCircuit, Type,
    WitnessDeclaration,
};
use compact_rust_backend::render;

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
    let (native, recorded) = stateful.split_once("pub mod recorded {").unwrap();
    assert_eq!(native.matches(".observed(").count(), 1);
    assert_eq!(recorded.matches(".observed(").count(), 1);
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
