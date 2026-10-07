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
