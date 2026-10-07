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

use super::*;
use crate::ir::{LocalBinding, Parameter, StructField};

fn field(value: &str) -> Expr {
    Expr::FieldLiteral {
        value: value.into(),
    }
}
fn parameter(name: &str) -> Expr {
    Expr::Parameter { name: name.into() }
}
fn call(name: &str, arguments: Vec<Expr>) -> Expr {
    Expr::Call {
        name: name.into(),
        arguments,
    }
}
fn circuit(name: &str, parameters: Vec<Parameter>, result: Type, body: Expr) -> PureCircuit {
    PureCircuit {
        source: None,
        name: name.into(),
        internal: false,
        parameters,
        result,
        body,
    }
}
fn declaration(name: &str, ty: Type) -> Parameter {
    Parameter {
        name: name.into(),
        ty,
    }
}
fn pair() -> Type {
    Type::Vector {
        element: Box::new(Type::Field),
        length: 2,
    }
}
fn pair_value() -> Expr {
    Expr::Vector {
        element: Type::Field,
        elements: vec![field("1"), field("2")],
    }
}
fn forbidden() -> Vec<Expr> {
    vec![
        Expr::CellRead {
            field: "secret".into(),
            index: 0,
        },
        Expr::WitnessCall {
            name: "answer".into(),
            arguments: vec![],
        },
        Expr::If {
            condition: Box::new(Expr::Boolean { value: true }),
            then: Box::new(field("1")),
            otherwise: Box::new(Expr::CellRead {
                field: "hidden".into(),
                index: 0,
            }),
        },
    ]
}

#[test]
fn field_policy_checks_transitive_arguments_scope_and_recursion_without_poisoning_siblings() {
    let leaf = circuit(
        "leaf",
        vec![declaration("x", Type::Field)],
        Type::Field,
        parameter("x"),
    );
    let root = circuit(
        "root",
        vec![],
        Type::Field,
        Expr::Add {
            left: Box::new(call("leaf", vec![field("1")])),
            right: Box::new(call("leaf", vec![field("2")])),
        },
    );
    let circuits = HashMap::from([("leaf", &leaf), ("root", &root)]);
    let mut visiting = HashSet::new();
    assert!(closed_pure_field_call("root", &circuits, &mut visiting));
    assert!(visiting.is_empty());
    assert!(!closed_pure_field_call("absent", &circuits, &mut visiting));
    for body in [
        call("leaf", vec![]),
        parameter("escaped"),
        call("root", vec![]),
    ]
    .into_iter()
    .chain(forbidden())
    {
        let bad = PureCircuit {
            body,
            ..root.clone()
        };
        let circuits = HashMap::from([("leaf", &leaf), ("root", &bad)]);
        assert!(!closed_pure_field_call("root", &circuits, &mut visiting));
        assert!(visiting.is_empty());
    }
    let wrong = PureCircuit {
        parameters: vec![declaration("x", Type::Boolean)],
        ..leaf.clone()
    };
    assert!(!closed_pure_field_call(
        "root",
        &HashMap::from([("leaf", &wrong), ("root", &root)]),
        &mut visiting
    ));
}

#[test]
fn unsigned_policy_refuses_hidden_effects_and_undeclared_arguments() {
    let ty = Type::Unsigned {
        max: "65535".into(),
    };
    let base = circuit(
        "sum",
        vec![declaration("x", ty.clone())],
        ty.clone(),
        Expr::UnsignedAdd {
            max: "65535".into(),
            left: Box::new(parameter("x")),
            right: Box::new(Expr::UnsignedLiteral {
                max: "65535".into(),
                value: "1".into(),
            }),
        },
    );
    assert!(closed_pure_unsigned_call(
        "sum",
        &HashMap::from([("sum", &base)]),
        &mut HashSet::new()
    ));
    for body in forbidden()
        .into_iter()
        .chain([parameter("unknown"), call("absent", vec![])])
    {
        let bad = PureCircuit {
            body,
            ..base.clone()
        };
        assert!(!closed_pure_unsigned_call(
            "sum",
            &HashMap::from([("sum", &bad)]),
            &mut HashSet::new()
        ));
    }
    for bad in [
        PureCircuit {
            result: Type::Field,
            ..base.clone()
        },
        PureCircuit {
            parameters: vec![declaration("x", Type::Field)],
            ..base
        },
    ] {
        assert!(!closed_pure_unsigned_call(
            "sum",
            &HashMap::from([("sum", &bad)]),
            &mut HashSet::new()
        ));
    }
}

#[test]
fn field_coercions_preserve_transitive_subtraction_scope_and_effect_checks() {
    let wrap = |value| Expr::Coerce {
        value: Box::new(value),
        ty: Type::Field,
    };
    let subtract = |left, right| Expr::Subtract {
        left: Box::new(left),
        right: Box::new(right),
    };
    let leaf = circuit(
        "leaf",
        vec![declaration("x", Type::Field)],
        Type::Field,
        wrap(subtract(parameter("x"), field("1"))),
    );
    // Repeated transitive calls must not leave the first sibling marked active.
    let root = circuit(
        "root",
        vec![declaration("root_only", Type::Field)],
        Type::Field,
        wrap(subtract(
            call("leaf", vec![parameter("root_only")]),
            call("leaf", vec![field("2")]),
        )),
    );
    let mut visiting = HashSet::new();
    let accepted = HashMap::from([("leaf", &leaf), ("root", &root)]);
    assert!(closed_pure_field_call("root", &accepted, &mut visiting));
    assert!(visiting.is_empty());
    for forbidden in forbidden().into_iter().chain([
        // A caller's formal is not in scope inside its callee.
        parameter("root_only"),
        call("root", vec![parameter("x")]),
        call("absent", vec![parameter("x")]),
        Expr::Coerce {
            value: Box::new(parameter("x")),
            ty: Type::Unsigned { max: "255".into() },
        },
    ]) {
        for invalid_on_left in [true, false] {
            let (left, right) = if invalid_on_left {
                (forbidden.clone(), parameter("x"))
            } else {
                (parameter("x"), forbidden.clone())
            };
            let rejected = PureCircuit {
                body: wrap(subtract(left, right)),
                ..leaf.clone()
            };
            assert!(
                !closed_pure_field_call(
                    "root",
                    &HashMap::from([("leaf", &rejected), ("root", &root)]),
                    &mut visiting,
                ),
                "wrapper concealed {forbidden:?}; invalid_on_left={invalid_on_left}"
            );
            assert!(visiting.is_empty());
            assert!(closed_pure_field_call("root", &accepted, &mut visiting));
            assert!(visiting.is_empty());
        }
    }
}

#[test]
fn unsigned_conversions_preserve_transitive_subtraction_scope_and_effect_checks() {
    let ty = Type::Unsigned {
        max: "65535".into(),
    };
    let literal = || Expr::UnsignedLiteral {
        max: "65535".into(),
        value: "1".into(),
    };
    let subtract = |left, right| Expr::UnsignedSubtract {
        max: "65535".into(),
        left: Box::new(left),
        right: Box::new(right),
    };
    let wrappers: [fn(Expr) -> Expr; 2] = [
        |value| Expr::Coerce {
            value: Box::new(value),
            ty: Type::Unsigned {
                max: "65535".into(),
            },
        },
        |value| Expr::UnsignedCast {
            max: "65535".into(),
            value: Box::new(value),
        },
    ];
    for (wrapper_index, wrap) in wrappers.into_iter().enumerate() {
        let leaf = circuit(
            "leaf",
            vec![declaration("x", ty.clone())],
            ty.clone(),
            wrap(subtract(parameter("x"), literal())),
        );
        let root = circuit(
            "root",
            vec![declaration("root_only", ty.clone())],
            ty.clone(),
            wrap(subtract(
                call("leaf", vec![parameter("root_only")]),
                call("leaf", vec![literal()]),
            )),
        );
        let accepted = HashMap::from([("leaf", &leaf), ("root", &root)]);
        let mut visiting = HashSet::new();
        assert!(closed_pure_unsigned_call("root", &accepted, &mut visiting));
        assert!(visiting.is_empty());
        for forbidden in forbidden().into_iter().chain([
            parameter("root_only"),
            call("root", vec![parameter("x")]),
            call("absent", vec![parameter("x")]),
            Expr::Coerce {
                value: Box::new(parameter("x")),
                ty: Type::Field,
            },
        ]) {
            for invalid_on_left in [true, false] {
                let (left, right) = if invalid_on_left {
                    (forbidden.clone(), parameter("x"))
                } else {
                    (parameter("x"), forbidden.clone())
                };
                let rejected = PureCircuit {
                    body: wrap(subtract(left, right)),
                    ..leaf.clone()
                };
                assert!(
                    !closed_pure_unsigned_call(
                        "root",
                        &HashMap::from([("leaf", &rejected), ("root", &root)]),
                        &mut visiting,
                    ),
                    "wrapper {wrapper_index} concealed {forbidden:?}; invalid_on_left={invalid_on_left}"
                );
                assert!(visiting.is_empty());
                assert!(closed_pure_unsigned_call("root", &accepted, &mut visiting));
                assert!(visiting.is_empty());
            }
        }
    }
}

#[test]
fn assertion_policy_preserves_exact_parameter_zero_and_statement_shapes() {
    let nonzero = circuit(
        "guard",
        vec![declaration("x", Type::Field)],
        Type::Unit,
        Expr::Sequence {
            steps: vec![Expr::Assert {
                condition: Box::new(Expr::NotEqual {
                    left: Box::new(parameter("x")),
                    right: Box::new(field("0")),
                }),
                message: "source message".into(),
            }],
            value: Box::new(Expr::Unit),
        },
    );
    assert!(closed_pure_assert_call(&nonzero));
    let boolean = circuit(
        "guard",
        vec![declaration("ok", Type::Boolean)],
        Type::Boolean,
        Expr::Sequence {
            steps: vec![Expr::Assert {
                condition: Box::new(parameter("ok")),
                message: "boolean source".into(),
            }],
            value: Box::new(parameter("ok")),
        },
    );
    assert!(closed_pure_assert_call(&boolean));
    for (left, right) in [
        (parameter("other"), field("0")),
        (parameter("x"), field("1")),
        (field("0"), parameter("x")),
    ] {
        let bad = PureCircuit {
            body: Expr::Sequence {
                steps: vec![Expr::Assert {
                    condition: Box::new(Expr::NotEqual {
                        left: Box::new(left),
                        right: Box::new(right),
                    }),
                    message: "x".into(),
                }],
                value: Box::new(Expr::Unit),
            },
            ..nonzero.clone()
        };
        assert!(!closed_pure_assert_call(&bad));
    }
    for extra in forbidden() {
        let mut bad = nonzero.clone();
        let Expr::Sequence { steps, .. } = &mut bad.body else {
            unreachable!()
        };
        steps.push(extra);
        assert!(!closed_pure_assert_call(&bad));
    }
    assert!(!closed_pure_assert_call(&PureCircuit {
        parameters: vec![],
        ..nonzero
    }));
}

#[test]
fn paired_hash_policy_checks_tuple_vector_coercion_and_transitive_call_result() {
    let leaf = circuit(
        "hash",
        vec![declaration("p", pair())],
        Type::Field,
        Expr::TransientHash {
            value: Box::new(parameter("p")),
        },
    );
    let root = circuit(
        "root",
        vec![],
        Type::Field,
        call(
            "hash",
            vec![Expr::Coerce {
                value: Box::new(Expr::Tuple {
                    elements: vec![field("1"), field("2")],
                }),
                ty: pair(),
            }],
        ),
    );
    assert!(closed_pure_field_pair_hash_call(
        "root",
        &HashMap::from([("root", &root), ("hash", &leaf)])
    ));
    for body in [
        call("hash", vec![]),
        call("root", vec![]),
        call("hash", vec![field("1")]),
        Expr::Coerce {
            value: Box::new(pair_value()),
            ty: Type::Field,
        },
    ] {
        let bad = PureCircuit {
            body,
            ..root.clone()
        };
        assert!(!closed_pure_field_pair_hash_call(
            "root",
            &HashMap::from([("root", &bad), ("hash", &leaf)])
        ));
    }
    let bad_leaf = PureCircuit {
        body: field("1"),
        result: pair(),
        ..leaf
    };
    assert!(!closed_pure_field_pair_hash_call(
        "root",
        &HashMap::from([("root", &root), ("hash", &bad_leaf)])
    ));
}

#[test]
fn paired_hash_policy_audits_unused_bindings_and_requires_an_actual_hash() {
    let good = circuit(
        "hash",
        vec![],
        Type::Field,
        Expr::Let {
            bindings: vec![LocalBinding {
                name: "p".into(),
                ty: pair(),
                value: pair_value(),
            }],
            body: Box::new(Expr::TransientHash {
                value: Box::new(parameter("p")),
            }),
        },
    );
    assert!(closed_pure_field_pair_hash_call(
        "hash",
        &HashMap::from([("hash", &good)])
    ));
    for value in forbidden() {
        let mut bad = good.clone();
        let Expr::Let { bindings, .. } = &mut bad.body else {
            unreachable!()
        };
        bindings.insert(
            0,
            LocalBinding {
                name: "unused".into(),
                ty: Type::Field,
                value,
            },
        );
        assert!(!closed_pure_field_pair_hash_call(
            "hash",
            &HashMap::from([("hash", &bad)])
        ));
    }
    for body in [
        field("1"),
        Expr::TransientHash {
            value: Box::new(Expr::Vector {
                element: Type::Field,
                elements: vec![field("1")],
            }),
        },
        Expr::TransientHash {
            value: Box::new(Expr::Tuple {
                elements: vec![field("1"), Expr::Boolean { value: true }],
            }),
        },
    ] {
        let bad = PureCircuit {
            body,
            ..good.clone()
        };
        assert!(!closed_pure_field_pair_hash_call(
            "hash",
            &HashMap::from([("hash", &bad)])
        ));
    }
}

#[test]
fn literal_vector_policy_preserves_exact_declared_length_element_and_arity() {
    let good = circuit("pair", vec![], pair(), pair_value());
    assert!(closed_literal_field_vector_call(
        "pair",
        &pair(),
        &HashMap::from([("pair", &good)])
    ));
    assert!(!closed_literal_field_vector_call(
        "absent",
        &pair(),
        &HashMap::from([("pair", &good)])
    ));
    for bad in [
        PureCircuit {
            parameters: vec![declaration("x", Type::Field)],
            ..good.clone()
        },
        PureCircuit {
            body: Expr::Vector {
                element: Type::Field,
                elements: vec![field("1")],
            },
            ..good.clone()
        },
        PureCircuit {
            body: Expr::Vector {
                element: Type::Boolean,
                elements: vec![field("1"), field("2")],
            },
            ..good.clone()
        },
        PureCircuit {
            body: Expr::Vector {
                element: Type::Field,
                elements: vec![parameter("x"), field("2")],
            },
            ..good.clone()
        },
        PureCircuit {
            result: Type::Field,
            ..good.clone()
        },
    ] {
        assert!(!closed_literal_field_vector_call(
            "pair",
            &pair(),
            &HashMap::from([("pair", &bad)])
        ));
    }
    assert!(!closed_literal_field_vector_call(
        "pair",
        &Type::Field,
        &HashMap::from([("pair", &good)])
    ));
}

#[test]
fn struct_hash_policy_refuses_effects_and_unsupported_cell_representations() {
    let ty = Type::Struct {
        name: "Record".into(),
        fields: vec![StructField {
            name: "tag".into(),
            ty: Type::Bytes { length: 4 },
        }],
    };
    let good = Expr::PersistentHash {
        value: Box::new(Expr::StructLiteral {
            ty: ty.clone(),
            fields: vec![parameter("tag")],
        }),
    };
    assert!(closed_struct_hash_value(&good));
    for expr in forbidden().into_iter().chain([call("helper", vec![])]) {
        assert!(!closed_struct_hash_value(&Expr::PersistentHash {
            value: Box::new(expr)
        }));
    }
    assert!(!closed_struct_hash_value(&Expr::Coerce {
        value: Box::new(parameter("opaque")),
        ty: Type::OpaqueString
    }));
    assert!(identity_struct_call_argument(&Expr::Coerce {
        value: Box::new(parameter("record")),
        ty: ty.clone()
    }));
    assert!(!identity_struct_call_argument(&Expr::Coerce {
        value: Box::new(Expr::Coerce {
            value: Box::new(parameter("record")),
            ty: Type::Field
        }),
        ty
    }));
}

#[test]
fn paired_hash_locals_keep_annotations_and_lexical_shadowing() {
    let good = circuit(
        "hash",
        vec![],
        Type::Field,
        Expr::Let {
            bindings: vec![LocalBinding {
                name: "p".into(),
                ty: pair(),
                value: pair_value(),
            }],
            body: Box::new(Expr::TransientHash {
                value: Box::new(parameter("p")),
            }),
        },
    );
    assert!(closed_pure_field_pair_hash_call(
        "hash",
        &HashMap::from([("hash", &good)])
    ));
    let mut wrong = good.clone();
    let Expr::Let { bindings, .. } = &mut wrong.body else {
        unreachable!()
    };
    bindings[0].ty = Type::Field;
    assert!(!closed_pure_field_pair_hash_call(
        "hash",
        &HashMap::from([("hash", &wrong)])
    ));

    let shadowed = PureCircuit {
        body: Expr::Let {
            bindings: vec![LocalBinding {
                name: "p".into(),
                ty: Type::Field,
                value: field("1"),
            }],
            body: Box::new(good.body.clone()),
        },
        ..good.clone()
    };
    assert!(closed_pure_field_pair_hash_call(
        "hash",
        &HashMap::from([("hash", &shadowed)])
    ));
    let escaped = PureCircuit {
        body: Expr::TransientHash {
            value: Box::new(Expr::Tuple {
                elements: vec![
                    Expr::Let {
                        bindings: vec![LocalBinding {
                            name: "local".into(),
                            ty: Type::Field,
                            value: field("1"),
                        }],
                        body: Box::new(parameter("local")),
                    },
                    parameter("local"),
                ],
            }),
        },
        ..good
    };
    assert!(!closed_pure_field_pair_hash_call(
        "hash",
        &HashMap::from([("hash", &escaped)])
    ));
}

#[test]
fn paired_hash_helper_resolution_checks_root_and_nested_names() {
    let leaf = circuit(
        "leaf",
        vec![declaration("p", pair())],
        Type::Field,
        Expr::TransientHash {
            value: Box::new(parameter("p")),
        },
    );
    let root = circuit(
        "root",
        vec![],
        Type::Field,
        call("leaf", vec![pair_value()]),
    );
    let declarations = HashMap::from([("root", &root), ("leaf", &leaf)]);
    assert!(closed_pure_field_pair_hash_call("root", &declarations));
    assert!(!closed_pure_field_pair_hash_call("missing", &declarations));
    assert!(!closed_pure_field_pair_hash_call(
        "root",
        &HashMap::from([("root", &root)])
    ));
}
