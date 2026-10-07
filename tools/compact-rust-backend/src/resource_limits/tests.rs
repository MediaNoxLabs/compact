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
use crate::ir::Contract;
use serde_json::json;
fn unit() -> serde_json::Value {
    json!({"kind":"unit"})
}
fn fixture(n: usize, repeats: usize) -> Contract {
    let circuits=(0..n).map(|i|json!({"name":format!("c{i}"),"parameters":[],"internal":i>0,"result":unit(),"return_value":unit(),"actions":if i+1<n {(0..repeats).map(|_|json!({"kind":"circuit_call","name":format!("c{}",i+1),"arguments":[]})).collect::<Vec<_>>()}else{vec![json!({"kind":"assert","condition":{"kind":"boolean","value":true},"message":"ok"})]}})).collect::<Vec<_>>();
    serde_json::from_value(json!({"schema_version":20,"ledger_fields":[],"witnesses":[],"circuits":[],"stateful_circuits":circuits})).unwrap()
}
#[test]
fn exact_nodes_and_one_over() {
    let c = fixture(3, 1);
    let m = measure(&c, Limits::CENSUS).unwrap();
    let mut l = Limits::CENSUS;
    l.nodes = m.nodes;
    assert!(measure(&c, l).is_ok());
    l.nodes -= 1;
    assert_eq!(measure(&c, l).unwrap_err().resource, Kind::Nodes);
}
#[test]
fn child_batch_checked_before_push() {
    let c = fixture(128, 1);
    let mut l = Limits::CENSUS;
    l.pending = 4;
    let e = measure(&c, l).unwrap_err();
    assert_eq!(
        (e.resource, e.observed, e.limit),
        (Kind::PendingNodes, 128, 4)
    );
}
#[test]
fn exact_string_bytes_and_one_over() {
    let c = fixture(3, 1);
    let m = measure(&c, Limits::CENSUS).unwrap();
    let mut l = Limits::CENSUS;
    l.strings = m.string_bytes;
    assert!(measure(&c, l).is_ok());
    l.strings -= 1;
    assert_eq!(measure(&c, l).unwrap_err().resource, Kind::StringBytes);
}
#[test]
fn declarations_count_before_allocation() {
    let c = fixture(3, 1);
    let mut l = Limits::CENSUS;
    l.declarations = 2;
    assert_eq!(measure(&c, l).unwrap_err().resource, Kind::Declarations);
}
#[test]
fn repeated_edges_count_expansion_not_only_unique_callees() {
    let once = measure(&fixture(8, 1), Limits::CENSUS).unwrap();
    let twice = measure(&fixture(8, 2), Limits::CENSUS).unwrap();
    assert!(twice.expanded_work > once.expanded_work * 4);
    let mut l = Limits::CENSUS;
    l.expanded_work = once.expanded_work;
    assert_eq!(
        measure(&fixture(8, 2), l).unwrap_err().resource,
        Kind::ExpandedWork
    );
}
#[test]
fn caller_boundaries_add_depth() {
    let one = measure(&fixture(1, 1), Limits::CENSUS).unwrap();
    let chain = measure(&fixture(8, 1), Limits::CENSUS).unwrap();
    assert_eq!(chain.call_depth, 8);
    assert!(chain.expanded_depth > one.syntax_depth);
    let mut l = Limits::CENSUS;
    l.call_depth = 7;
    assert_eq!(
        measure(&fixture(8, 1), l).unwrap_err().resource,
        Kind::CallDepth
    );
    l = Limits::CENSUS;
    l.expanded_depth = chain.expanded_depth - 1;
    assert_eq!(
        measure(&fixture(8, 1), l).unwrap_err().resource,
        Kind::ExpandedDepth
    );
}
#[test]
fn graph_metrics_are_declaration_order_independent() {
    let c = fixture(8, 2);
    let a = measure(&c, Limits::CENSUS).unwrap();
    let mut c = c;
    c.stateful_circuits.reverse();
    let b = measure(&c, Limits::CENSUS).unwrap();
    assert_eq!(a.nodes, b.nodes);
    assert_eq!(a.expanded_work, b.expanded_work);
    assert_eq!(a.call_depth, b.call_depth);
    assert_eq!(a.expanded_depth, b.expanded_depth);
}
#[test]
fn cycle_and_unknown_are_returned_to_semantic_owner() {
    use crate::ir::StateAction;
    let mut c = fixture(2, 1);
    c.stateful_circuits[1].actions = vec![StateAction::CircuitCall {
        name: "c0".into(),
        arguments: vec![],
    }];
    assert_eq!(
        measure(&c, Limits::CENSUS).unwrap().graph_status,
        GraphStatus::Cycle
    );
    c.stateful_circuits[1].actions = vec![StateAction::CircuitCall {
        name: "absent".into(),
        arguments: vec![],
    }];
    assert_eq!(
        measure(&c, Limits::CENSUS).unwrap().graph_status,
        GraphStatus::UnknownCallee
    );
}
#[test]
fn bounded_deterministic_graph_family() {
    for n in 1..=12 {
        for r in 1..=3 {
            let c = fixture(n, r);
            let m = measure(&c, Limits::CENSUS).unwrap();
            assert_eq!(m.call_depth, n);
            assert_eq!(m.call_edges, (n - 1) * r);
            assert!(m.expanded_work >= m.nodes - 1);
        }
    }
}

#[test]
fn exact_family_path_and_one_path_unit_over() {
    let c = fixture(12, 1);
    let m = measure(&c, Limits::CENSUS).unwrap();
    let cost = *m.render_path_units.iter().max().unwrap();
    assert!(cost > 0);
    let mut l = Limits::CENSUS;
    l.path_units = cost;
    assert!(measure(&c, l).is_ok());
    l.path_units -= 1;
    let e = measure(&c, l).unwrap_err();
    assert_eq!(e.resource, Kind::TypedRecordedPath);
    assert_eq!((e.observed, e.limit), (cost, cost - 1));
}
#[test]
fn exact_syntax_and_expanded_depth_then_one_over() {
    let c = fixture(12, 1);
    let m = measure(&c, Limits::CENSUS).unwrap();
    let mut l = Limits::CENSUS;
    l.syntax_depth = m.syntax_depth;
    l.expanded_depth = m.expanded_depth;
    assert!(measure(&c, l).is_ok());
    l.syntax_depth -= 1;
    assert_eq!(measure(&c, l).unwrap_err().resource, Kind::SyntaxDepth);
    l.syntax_depth = m.syntax_depth;
    l.expanded_depth -= 1;
    assert_eq!(measure(&c, l).unwrap_err().resource, Kind::ExpandedDepth);
}

// Direct typed API controls run on the standard Rust test worker. Parsing,
// lowering, syn construction and pretty-printing all occur inside that worker.
fn nested_expression(family: &str, n: usize, native: bool) -> Contract {
    use crate::ir::{Expr, Parameter, PureCircuit, StateReturn, StatefulCircuit, Type};
    let mut c: Contract = serde_json::from_value(
        json!({"schema_version":20,"ledger_fields":[],"circuits":[],"stateful_circuits":[]}),
    )
    .unwrap();
    let mut body = Expr::Parameter { name: "x".into() };
    for _ in 0..n {
        body = match family {
            "hash" => Expr::TransientHash {
                value: Box::new(body),
            },
            "add" => Expr::Add {
                left: Box::new(body),
                right: Box::new(Expr::FieldLiteral { value: "1".into() }),
            },
            "if" => Expr::If {
                condition: Box::new(Expr::Parameter {
                    name: "selected".into(),
                }),
                then: Box::new(body),
                otherwise: Box::new(Expr::FieldLiteral { value: "1".into() }),
            },
            _ => unreachable!(),
        };
    }
    let parameters = vec![
        Parameter {
            name: "x".into(),
            ty: Type::Field,
        },
        Parameter {
            name: "selected".into(),
            ty: Type::Boolean,
        },
    ];
    if native {
        c.stateful_circuits.push(StatefulCircuit {
            source: None,
            name: "entry".into(),
            internal: false,
            parameters,
            result: Type::Field,
            actions: vec![],
            return_value: StateReturn::Expression { value: body },
        });
    } else {
        c.circuits.push(PureCircuit {
            source: None,
            name: "entry".into(),
            internal: false,
            parameters,
            result: Type::Field,
            body,
        });
    }
    c
}
#[test]
fn ordinary_worker_expression_boundaries_and_refusal() {
    for (family, n, native) in [
        ("hash", 42, false),
        ("hash", 41, true),
        ("if", 45, false),
        ("add", 45, false),
    ] {
        let c = nested_expression(family, n, native);
        crate::render_with_capabilities(&c).expect("bounded ordinary-worker expression");
    }
    for (family, n, native, kind) in [
        ("hash", 43, false, Kind::PurePath),
        ("hash", 42, true, Kind::NativePath),
        ("if", 46, false, Kind::SyntaxDepth),
    ] {
        let c = nested_expression(family, n, native);
        let error = measure(&c, Limits::DEFAULT).unwrap_err();
        assert_eq!(error.resource, kind);
        assert!(matches!(
            crate::render(&c),
            Err(crate::RenderError::ResourceLimit { .. })
        ));
    }
}
#[test]
fn schema_precedes_resource_and_local_frame_precedes_graph_delegation() {
    let mut c = nested_expression("hash", 43, false);
    c.schema_version = 0;
    assert_eq!(crate::render(&c), Err(crate::RenderError::SchemaVersion(0)));
    c.schema_version = 20;
    c.stateful_circuits = fixture(2, 1).stateful_circuits;
    c.stateful_circuits[1].actions = vec![crate::ir::StateAction::CircuitCall {
        name: "missing".into(),
        arguments: vec![],
    }];
    assert_eq!(
        measure(&c, Limits::DEFAULT).unwrap_err().resource,
        Kind::PurePath
    );
    c.stateful_circuits[1].actions = vec![crate::ir::StateAction::CircuitCall {
        name: "c0".into(),
        arguments: vec![],
    }];
    assert_eq!(
        measure(&c, Limits::DEFAULT).unwrap_err().resource,
        Kind::PurePath
    );
}
#[test]
fn semantic_unknown_cycle_and_malformed_shapes_are_unchanged() {
    use crate::ir::StateAction;
    let mut c = fixture(2, 1);
    c.stateful_circuits[1].actions = vec![StateAction::CircuitCall {
        name: "missing".into(),
        arguments: vec![],
    }];
    assert_eq!(
        crate::render(&c),
        Err(crate::RenderError::UnknownCircuit("missing".into()))
    );
    c.stateful_circuits[1].actions = vec![StateAction::CircuitCall {
        name: "c0".into(),
        arguments: vec![],
    }];
    assert_eq!(
        crate::render(&c),
        Err(crate::RenderError::UnsupportedStatefulCall("c0".into()))
    );
    let mut c = nested_expression("add", 1, false);
    c.circuits[0].body = crate::ir::Expr::Parameter {
        name: "missing".into(),
    };
    assert_eq!(
        crate::render(&c),
        Err(crate::RenderError::UnknownParameter("missing".into()))
    );
}
#[test]
fn nested_types_and_aliases_are_bounded_on_ordinary_worker() {
    use crate::ir::{Type, TypeAlias};
    for (depth, accepted) in [(44, true), (47, false)] {
        let mut ty = Type::Field;
        for _ in 0..depth {
            ty = Type::Tuple { elements: vec![ty] };
        }
        let mut c = nested_expression("add", 0, false);
        c.circuits.clear();
        c.type_aliases.push(TypeAlias {
            source: None,
            name: "Nested".into(),
            ty,
        });
        if accepted {
            crate::render(&c).expect("bounded nested alias");
        } else {
            assert!(matches!(
                crate::render(&c),
                Err(crate::RenderError::ResourceLimit {
                    resource: "syntax_depth",
                    ..
                })
            ));
        }
    }
}
#[test]
fn reviewed_raw_and_normalized_names_preserve_semantic_ownership() {
    let mut c = nested_expression("add", 0, false);
    c.circuits[0].name = "a$b".into();
    c.stateful_circuits = fixture(1, 1).stateful_circuits;
    c.stateful_circuits[0].name = "a_b".into();
    assert!(crate::render(&c).is_ok());
    c.stateful_circuits[0].name = "a$b".into();
    assert_eq!(
        crate::render(&c),
        Err(crate::RenderError::DuplicateCircuit("a$b".into()))
    );
}

#[test]
fn calibrated_mixed_owner_paths_render_on_ordinary_worker() {
    // Small immutable calibration inputs retain their exact combined shapes.
    for (source, recorded) in [
        (include_str!("test_inputs/binding_arithmetic.json"), true),
        (include_str!("test_inputs/branch_arithmetic.json"), true),
        (include_str!("test_inputs/constructor_crypto.json"), false),
        (include_str!("test_inputs/bound_chain-14.json"), true),
        (include_str!("test_inputs/mixed_chain-11.json"), true),
        (include_str!("test_inputs/native_map-32.json"), false),
    ] {
        let contract: Contract = serde_json::from_str(source).unwrap();
        let rendered =
            crate::render_with_capabilities(&contract).expect("bounded mixed owner path");
        assert_eq!(
            rendered.capabilities.circuits.iter().any(|c| c.recorded),
            recorded
        );
    }
}
#[test]
fn literal_buffer_exact_and_one_over() {
    let mut c = nested_expression("hash", 0, false);
    c.circuits[0].body = crate::ir::Expr::BytesLiteral { bytes: vec![0; 32] };
    let mut limits = Limits::CENSUS;
    limits.literal_bytes = 32;
    assert!(measure(&c, limits).is_ok());
    limits.literal_bytes = 31;
    assert_eq!(
        measure(&c, limits).unwrap_err().resource,
        Kind::LiteralBytes
    );
}

fn constructor_assertion(message: &str) -> Contract {
    use crate::ir::{Constructor, ConstructorStep, Expr};
    let mut contract = fixture(0, 0);
    contract.constructor = Some(Constructor {
        source: None,
        parameters: vec![],
        steps: vec![ConstructorStep::Assert {
            condition: Expr::Boolean { value: true },
            message: message.into(),
        }],
    });
    contract
}

#[test]
fn constructor_assertion_counts_utf8_bytes_at_the_exact_string_boundary() {
    // No identifiers or other text occur in this constructor. These three
    // Unicode scalars occupy 2 + 4 + 3 bytes, independently of the visitor.
    let exact = constructor_assertion("é🌙界");
    crate::render(&exact).expect("supported constructor assertion");
    let limits = Limits {
        strings: 9,
        ..Limits::CENSUS
    };
    assert_eq!(measure(&exact, limits).unwrap().string_bytes, 9);
    let excess = constructor_assertion("é🌙界!");
    let error = measure(&excess, limits).unwrap_err();
    assert_eq!(
        (error.resource, error.limit, error.observed),
        (Kind::StringBytes, 9, 10)
    );
}

#[test]
fn commitment_accounts_for_literal_bytes_in_each_operand() {
    use crate::ir::{Expr, Type};
    let make = |value_len, opening_len| {
        let mut contract = nested_expression("hash", 0, false);
        let circuit = &mut contract.circuits[0];
        circuit.parameters.clear();
        circuit.result = Type::Bytes { length: 32 };
        circuit.body = Expr::PersistentCommit {
            value: Box::new(Expr::BytesLiteral {
                bytes: vec![0; value_len],
            }),
            opening: Box::new(Expr::BytesLiteral {
                bytes: vec![0; opening_len],
            }),
        };
        contract
    };
    let exact = make(3, 32);
    crate::render(&exact).expect("supported commitment with a 32-byte opening");
    let limits = Limits {
        literal_bytes: 35,
        ..Limits::CENSUS
    };
    assert_eq!(measure(&exact, limits).unwrap().literal_bytes, 35);
    // Both additions must count. The 33-byte opening is a scanner-only input:
    // public semantic lowering still owns its separate opening-type refusal.
    for excess in [make(4, 32), make(3, 33)] {
        let error = measure(&excess, limits).unwrap_err();
        assert_eq!(
            (error.resource, error.limit, error.observed),
            (Kind::LiteralBytes, 35, 36)
        );
    }
}

#[test]
fn nested_commitment_and_ec_operands_cannot_escape_depth_accounting() {
    use crate::ir::{Expr, Type};
    let field = || Expr::FieldLiteral { value: "7".into() };
    let wrap = |mut value: Expr, point: bool, count| {
        for _ in 0..count {
            value = if point {
                Expr::EcNeg {
                    value: Box::new(value),
                }
            } else {
                Expr::TransientHash {
                    value: Box::new(value),
                }
            };
        }
        value
    };
    for (ec, second) in [(false, false), (false, true), (true, false), (true, true)] {
        let make = |nested| {
            let mut contract = nested_expression("hash", 0, false);
            let circuit = &mut contract.circuits[0];
            circuit.parameters.clear();
            circuit.result = if ec { Type::JubjubPoint } else { Type::Field };
            let mut first = if ec {
                Expr::EcMulGenerator {
                    scalar: Box::new(field()),
                }
            } else {
                field()
            };
            let mut last = field();
            if nested {
                if second {
                    // The EC point starts one level deeper than the scalar.
                    last = wrap(last, false, if ec { 4 } else { 3 });
                } else {
                    first = wrap(first, ec, 3);
                }
            }
            circuit.body = if ec {
                Expr::EcMul {
                    point: Box::new(first),
                    scalar: Box::new(last),
                }
            } else {
                Expr::TransientCommit {
                    value: Box::new(first),
                    opening: Box::new(last),
                }
            };
            contract
        };
        let shallow = make(false);
        let nested = make(true);
        crate::render(&shallow).expect("supported shallow crypto expression");
        crate::render(&nested).expect("supported nested crypto expression");
        let depth = measure(&shallow, Limits::CENSUS).unwrap().syntax_depth;
        let exact = Limits {
            syntax_depth: depth + 3,
            ..Limits::CENSUS
        };
        assert_eq!(measure(&nested, exact).unwrap().syntax_depth, depth + 3);
        let short = Limits {
            syntax_depth: depth + 2,
            ..exact
        };
        assert!(measure(&shallow, short).is_ok());
        let error = measure(&nested, short).unwrap_err();
        assert_eq!(
            (error.resource, error.limit, error.observed),
            (Kind::SyntaxDepth, depth + 2, depth + 3),
            "ec={ec}, second_operand={second}"
        );
    }
}

#[test]
fn collection_returns_and_list_actions_count_slot_and_expression_payloads() {
    use crate::ir::{Expr, LedgerField, LedgerFieldKind, StateAction, StateReturn, Type};
    // These are the same typed shapes used by existing Set-member, Map-lookup
    // and List-push renderer tests. No new recorded profile is being admitted.
    for carrier in ["set", "map", "list"] {
        let make = |field: &str, literal: &str| {
            let mut contract = fixture(1, 0);
            let value = Expr::FieldLiteral {
                value: literal.into(),
            };
            contract.ledger_fields.push(LedgerField {
                source: None,
                id: field.into(),
                index: 0,
                path: vec![],
                declaration: match carrier {
                    "set" => LedgerFieldKind::Set { ty: Type::Field },
                    "map" => LedgerFieldKind::Map {
                        key: Type::Field,
                        value: Type::Field,
                    },
                    _ => LedgerFieldKind::List { ty: Type::Field },
                },
            });
            let circuit = &mut contract.stateful_circuits[0];
            circuit.name = "read".into();
            circuit.actions.clear();
            match carrier {
                "set" => {
                    circuit.result = Type::Boolean;
                    circuit.return_value = StateReturn::SetMember {
                        field: field.into(),
                        index: 0,
                        value,
                    };
                }
                "map" => {
                    circuit.result = Type::Field;
                    circuit.return_value = StateReturn::MapLookup {
                        field: field.into(),
                        index: 0,
                        key: value,
                    };
                }
                _ => circuit.actions.push(StateAction::ListPushFront {
                    field: field.into(),
                    index: 0,
                    value,
                }),
            }
            contract
        };
        // "read" (4), declared "store" (5), referenced "store" (5), "7" (1).
        let exact = make("store", "7");
        crate::render(&exact).expect("supported collection carrier");
        let limits = Limits {
            strings: 15,
            ..Limits::CENSUS
        };
        assert_eq!(measure(&exact, limits).unwrap().string_bytes, 15);
        let error = measure(&make("store", "17"), limits).unwrap_err();
        assert_eq!(
            (error.resource, error.limit, error.observed),
            (Kind::StringBytes, 15, 16),
            "{carrier} expression text"
        );
        // A renamed slot must account for both its declaration and its use.
        let renamed = make("stores", "7");
        crate::render(&renamed).expect("consistent slot rename");
        let exact_rename = Limits {
            strings: 17,
            ..limits
        };
        assert_eq!(measure(&renamed, exact_rename).unwrap().string_bytes, 17);
        let error = measure(
            &renamed,
            Limits {
                strings: 16,
                ..limits
            },
        )
        .unwrap_err();
        assert_eq!(
            (error.resource, error.limit, error.observed),
            (Kind::StringBytes, 16, 17),
            "{carrier} declared and referenced slot"
        );
    }
}

#[test]
fn all_public_render_entries_preserve_resource_error_fields_and_display() {
    let metadata = json!({"circuits": []});
    let small = constructor_assertion("bounded");
    crate::render(&small).unwrap();
    crate::render_with_capabilities(&small).unwrap();
    crate::render_with_proof_capabilities(&small, &metadata).unwrap();

    let limit = Limits::DEFAULT.strings;
    let large = constructor_assertion(&"x".repeat(limit + 1));
    let errors = [
        crate::render(&large).unwrap_err(),
        crate::render_with_capabilities(&large)
            .err()
            .expect("resource failure"),
        crate::render_with_proof_capabilities(&large, &metadata)
            .err()
            .expect("resource failure"),
    ];
    for error in errors {
        assert_eq!(
            error,
            crate::RenderError::ResourceLimit {
                resource: "string_bytes",
                limit,
                observed: limit + 1,
            }
        );
        assert_eq!(
            error.to_string(),
            format!(
                "compiler resource string_bytes exceeds {limit} (observed {})",
                limit + 1
            )
        );
    }
}
