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
use crate::ir::{
    Expr, Parameter, PureCircuit, SourceLocation, StateAction, StateReturn, StatefulCircuit, Type,
};
use crate::source_diagnostics::{Owner, Slot};
use std::cell::RefCell;

#[derive(Default)]
struct Trace(RefCell<Vec<(Option<OwnerKey>, Slot)>>);
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OwnerKey {
    Expr(usize),
    Pure(usize),
    Action(usize),
    Other,
}
fn owner_key(owner: Option<Owner<'_>>) -> Option<OwnerKey> {
    owner.map(|owner| match owner {
        Owner::Expr(value) => OwnerKey::Expr(std::ptr::from_ref(value) as usize),
        Owner::Pure(value) => OwnerKey::Pure(std::ptr::from_ref(value) as usize),
        Owner::Action(value) => OwnerKey::Action(std::ptr::from_ref(value) as usize),
        _ => OwnerKey::Other,
    })
}
impl RefusalObserver for Trace {
    fn refused(&self, site: RefusalSite<'_>) {
        self.0.borrow_mut().push((owner_key(site.owner), site.slot));
    }
}
fn one(trace: &Trace, expr: &Expr) {
    assert_eq!(
        trace.0.borrow().as_slice(),
        &[(owner_key(Some(Owner::Expr(expr))), Slot::Node)]
    );
}
fn field() -> Expr {
    Expr::FieldLiteral { value: "1".into() }
}
fn call(name: &str) -> Expr {
    Expr::Call {
        name: name.into(),
        arguments: vec![],
    }
}
fn pure(name: &str, body: Expr) -> PureCircuit {
    PureCircuit {
        source: None,
        name: name.into(),
        internal: false,
        parameters: vec![],
        result: Type::Field,
        body,
    }
}
fn contract(circuits: Vec<PureCircuit>) -> Contract {
    Contract {
        schema_version: 20,
        type_aliases: vec![],
        ledger_fields: vec![],
        constructor: None,
        witnesses: vec![],
        circuits,
        stateful_circuits: vec![],
    }
}
fn render(c: &Contract, trace: Option<&Trace>) -> Result<String, crate::RenderError> {
    crate::render_with_native_refusal(c, None, trace.map(|value| value as &dyn RefusalObserver))
        .map(|value| value.source)
}

#[test]
fn self_cycle_site_is_deferred_until_finish_and_keeps_old_located_target() {
    let mut c = contract(vec![pure("again", call("again"))]);
    let point = SourceLocation {
        file: "cycle.compact".into(),
        line: 3,
        column: 8,
    };
    c.circuits[0].source = Some(point.clone());
    let trace = Trace::default();
    let checked = validate_with_observer(&c, Some(&trace)).unwrap();
    assert!(trace.0.borrow().is_empty());
    assert_eq!(
        checked.finish(&c),
        Err(crate::RenderError::Located {
            location: point.clone(),
            error: Box::new(crate::RenderError::RecursivePureCall("again".into()))
        })
    );
    one(&trace, &c.circuits[0].body);
    let trace = Trace::default();
    let observed = render(&c, Some(&trace));
    assert_eq!(observed, render(&c, None));
    assert_eq!(
        observed,
        Err(crate::RenderError::Located {
            location: point,
            error: Box::new(crate::RenderError::RecursivePureCall("again".into()))
        })
    );
    one(&trace, &c.circuits[0].body);
}

#[test]
fn mutual_cycle_keeps_first_gray_edge_not_equal_other_occurrence() {
    let c = contract(vec![
        pure("first", call("second")),
        pure(
            "second",
            Expr::Add {
                left: Box::new(call("first")),
                right: Box::new(call("first")),
            },
        ),
    ]);
    // Original LIFO source scan registers Add.right before Add.left; the
    // unchanged DFS starts declaration zero and closes on that first edge.
    let Expr::Add { left, right } = &c.circuits[1].body else {
        panic!("fixture")
    };
    assert_eq!(left, right);
    assert!(!std::ptr::eq(left.as_ref(), right.as_ref()));
    let trace = Trace::default();
    let observed = render(&c, Some(&trace));
    assert_eq!(observed, render(&c, None));
    assert_eq!(
        observed,
        Err(crate::RenderError::RecursivePureCall("first".into()))
    );
    one(&trace, right);
    assert_ne!(
        owner_key(Some(Owner::Expr(left))),
        owner_key(Some(Owner::Expr(right)))
    );
}

#[test]
fn fold_callback_retains_original_whole_fold_node() {
    let body = Expr::VectorFoldCall {
        name: "fold".into(),
        initial: Box::new(field()),
        source: Box::new(Expr::Vector {
            element: Type::Field,
            elements: vec![field()],
        }),
        accumulator: Type::Field,
        element: Type::Field,
        length: 1,
    };
    let mut p = pure("fold", body);
    p.parameters = ["acc", "item"]
        .map(|name| Parameter {
            name: name.into(),
            ty: Type::Field,
        })
        .to_vec();
    let c = contract(vec![p]);
    let trace = Trace::default();
    let observed = render(&c, Some(&trace));
    assert_eq!(observed, render(&c, None));
    assert_eq!(
        observed,
        Err(crate::RenderError::RecursivePureCall("fold".into()))
    );
    one(&trace, &c.circuits[0].body);
}

#[test]
fn existing_semantic_winner_suppresses_deferred_cycle_callback() {
    let mut c = contract(vec![
        pure("again", call("again")),
        pure(
            "other",
            Expr::FieldLiteral {
                value: "bad".into(),
            },
        ),
    ]);
    let trace = Trace::default();
    let checked = validate_with_observer(&c, Some(&trace)).unwrap();
    assert!(checked.pure_cycle.is_some());
    assert!(trace.0.borrow().is_empty());
    drop(checked);
    let trace = Trace::default();
    let observed = render(&c, Some(&trace));
    assert_eq!(observed, render(&c, None));
    assert_eq!(
        observed,
        Err(crate::RenderError::InvalidFieldLiteral("bad".into()))
    );
    one(&trace, &c.circuits[1].body);
    c.circuits.truncate(1);
    c.circuits[0].parameters.push(Parameter {
        name: "value".into(),
        ty: Type::Field,
    });
    let trace = Trace::default();
    let observed = render(&c, Some(&trace));
    assert_eq!(observed, render(&c, None));
    assert_eq!(
        observed,
        Err(crate::RenderError::ArgumentCount {
            circuit: "again".into(),
            expected: 1,
            actual: 0
        })
    );
    one(&trace, &c.circuits[0].body);
}

#[test]
fn duplicate_unknown_and_mixed_graphs_never_issue_a_preflight_cycle_site() {
    let mut c = contract(vec![pure("again", call("again")), pure("again", field())]);
    let trace = Trace::default();
    validate_with_observer(&c, Some(&trace))
        .unwrap()
        .finish(&c)
        .unwrap();
    assert!(trace.0.borrow().is_empty());
    assert_eq!(
        measure(&c, Limits::DEFAULT).unwrap().graph_status,
        GraphStatus::DuplicateName
    );
    let trace = Trace::default();
    let observed = render(&c, Some(&trace));
    assert_eq!(observed, render(&c, None));
    assert_eq!(
        observed,
        Err(crate::RenderError::DuplicateCircuit("again".into()))
    );
    assert_eq!(
        trace.0.borrow().as_slice(),
        &[(
            owner_key(Some(Owner::Pure(&c.circuits[1]))),
            Slot::Field("name")
        )]
    );
    c.circuits[1] = pure("other", call("missing"));
    let trace = Trace::default();
    validate_with_observer(&c, Some(&trace))
        .unwrap()
        .finish(&c)
        .unwrap();
    assert!(trace.0.borrow().is_empty());
    assert_eq!(
        measure(&c, Limits::DEFAULT).unwrap().graph_status,
        GraphStatus::UnknownCallee
    );
    let trace = Trace::default();
    let observed = render(&c, Some(&trace));
    assert_eq!(observed, render(&c, None));
    assert_eq!(
        observed,
        Err(crate::RenderError::UnknownCircuit("missing".into()))
    );
    one(&trace, &c.circuits[1].body);
    c.circuits = vec![pure("again", call("state"))];
    c.stateful_circuits = vec![StatefulCircuit {
        source: None,
        name: "state".into(),
        internal: false,
        parameters: vec![],
        actions: vec![StateAction::PureCall {
            name: "again".into(),
            arguments: vec![],
        }],
        result: Type::Unit,
        return_value: StateReturn::Unit,
    }];
    let trace = Trace::default();
    validate_with_observer(&c, Some(&trace))
        .unwrap()
        .finish(&c)
        .unwrap();
    assert!(trace.0.borrow().is_empty());
    assert_eq!(
        measure(&c, Limits::DEFAULT).unwrap().graph_status,
        GraphStatus::Cycle
    );
    let trace = Trace::default();
    let observed = render(&c, Some(&trace));
    assert_eq!(observed, render(&c, None));
    assert_eq!(
        observed,
        Err(crate::RenderError::UnknownCircuit("state".into()))
    );
    one(&trace, &c.circuits[0].body);
}

#[test]
fn stateful_cycles_remain_deferred_to_original_semantic_owner() {
    let mut c = contract(vec![]);
    c.stateful_circuits = (0..2)
        .map(|i| StatefulCircuit {
            source: None,
            name: format!("state{i}"),
            internal: false,
            parameters: vec![],
            actions: vec![StateAction::CircuitCall {
                name: format!("state{}", 1 - i),
                arguments: vec![],
            }],
            result: Type::Unit,
            return_value: StateReturn::Unit,
        })
        .collect();
    let trace = Trace::default();
    validate_with_observer(&c, Some(&trace))
        .unwrap()
        .finish(&c)
        .unwrap();
    assert!(trace.0.borrow().is_empty());
    assert_eq!(
        measure(&c, Limits::DEFAULT).unwrap().graph_status,
        GraphStatus::Cycle
    );
    let trace = Trace::default();
    let observed = render(&c, Some(&trace));
    assert_eq!(observed, render(&c, None));
    assert_eq!(
        observed,
        Err(crate::RenderError::UnsupportedStatefulCall("state0".into()))
    );
    assert_eq!(
        trace.0.borrow().as_slice(),
        &[(
            owner_key(Some(Owner::Action(&c.stateful_circuits[1].actions[0]))),
            Slot::Field("name")
        )]
    );
}

#[test]
fn distinct_equal_contract_finish_cannot_borrow_first_contract_edge() {
    let first = contract(vec![pure("again", call("again"))]);
    let other = first.clone();
    assert_eq!(first, other);
    assert!(!std::ptr::eq(&first, &other));
    let trace = Trace::default();
    let checked = validate_with_observer(&first, Some(&trace)).unwrap();
    assert!(trace.0.borrow().is_empty());
    assert_eq!(
        checked.finish(&other),
        Err(crate::RenderError::RecursivePureCall("again".into()))
    );
    assert_eq!(trace.0.borrow().as_slice(), &[(None, Slot::Node)]);
    let trace = Trace::default();
    let checked = validate_with_observer(&other, Some(&trace)).unwrap();
    assert_eq!(
        checked.finish(&other),
        Err(crate::RenderError::RecursivePureCall("again".into()))
    );
    one(&trace, &other.circuits[0].body);
}

#[test]
fn successful_shared_dag_preserves_complete_output_without_events() {
    let c = contract(vec![
        pure(
            "entry",
            Expr::Add {
                left: Box::new(call("leaf")),
                right: Box::new(call("leaf")),
            },
        ),
        pure("leaf", field()),
    ]);
    let trace = Trace::default();
    assert_eq!(render(&c, Some(&trace)), render(&c, None));
    assert!(render(&c, None).is_ok());
    assert!(trace.0.borrow().is_empty());
    assert_eq!(
        measure(&c, Limits::DEFAULT).unwrap().graph_status,
        GraphStatus::Acyclic
    );
}

#[test]
fn full_render_declaration_cycle_and_one_over_refusal_on_two_mib_worker() {
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(|| {
            let count = Limits::DEFAULT.declarations;
            let mut c = contract(
                (0..count)
                    .map(|i| pure(&format!("c{i}"), call(&format!("c{}", (i + 1) % count))))
                    .collect(),
            );
            let trace = Trace::default();
            let observed = render(&c, Some(&trace));
            assert_eq!(observed, render(&c, None));
            assert_eq!(
                observed,
                Err(crate::RenderError::RecursivePureCall("c0".into()))
            );
            one(&trace, &c.circuits[count - 1].body);
            c.circuits.push(pure("extra", field()));
            let trace = Trace::default();
            let observed = render(&c, Some(&trace));
            assert_eq!(observed, render(&c, None));
            assert_eq!(
                observed,
                Err(crate::RenderError::ResourceLimit {
                    resource: "declarations",
                    observed: count + 1,
                    limit: count
                })
            );
            assert!(trace.0.borrow().is_empty());
            c.circuits.pop();
            c.circuits[0].body = Expr::Add {
                left: Box::new(call("c1")),
                right: Box::new(Expr::FieldLiteral {
                    value: "bad".into(),
                }),
            };
            let Expr::Add { right, .. } = &c.circuits[0].body else {
                panic!("fixture")
            };
            let trace = Trace::default();
            let observed = render(&c, Some(&trace));
            assert_eq!(observed, render(&c, None));
            assert_eq!(
                observed,
                Err(crate::RenderError::InvalidFieldLiteral("bad".into()))
            );
            assert_eq!(
                trace.0.borrow().as_slice(),
                &[
                    (owner_key(Some(Owner::Expr(right))), Slot::Node),
                    (
                        owner_key(Some(Owner::Expr(&c.circuits[0].body))),
                        Slot::Node
                    ),
                ]
            );
        })
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn global_dag_resource_failure_has_no_unique_closing_edge_or_observer_site() {
    let c = contract(vec![pure("entry", call("leaf")), pure("leaf", field())]);
    let mut limits = Limits::DEFAULT;
    limits.call_depth = 1;
    let trace = Trace::default();
    let mut site = None;
    let error = measure_with_cycle_site(&c, limits, Some(&trace), &mut site)
        .expect_err("unchanged global graph bound");
    assert_eq!(error.resource, Kind::CallDepth);
    assert_eq!(error.observed, 2);
    assert_eq!(error.limit, 1);
    assert!(site.is_none());
    assert!(trace.0.borrow().is_empty());
}
