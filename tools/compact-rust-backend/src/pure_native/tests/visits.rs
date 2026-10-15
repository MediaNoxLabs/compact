// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;

#[test]
fn sequence_children_and_equal_arm_occurrences_are_checked_once() {
    let source = Expr::Sequence {
        steps: vec![local("saved", Type::Unit, call("mark"), reference("saved"))],
        value: Box::new(branch(boolean(true), call("mark"), call("mark"))),
    };
    let mark = pure("mark", Type::Unit);
    let calls = HashMap::from([("mark", &mark)]);
    let (result, visits) = visitation::observe(|| observe(&source, &HashMap::new(), &calls));
    result.unwrap();
    let Expr::Sequence { steps, value } = &source else {
        unreachable!()
    };
    let Expr::Let { bindings, body } = &steps[0] else {
        unreachable!()
    };
    let Expr::If {
        condition,
        then,
        otherwise,
    } = value.as_ref()
    else {
        unreachable!()
    };
    let nodes = [
        &source,
        &steps[0],
        &bindings[0].value,
        body.as_ref(),
        value.as_ref(),
        condition.as_ref(),
        then.as_ref(),
        otherwise.as_ref(),
    ];
    assert_eq!(visits.len(), nodes.len());
    for node in nodes {
        assert_eq!(visits[&(std::ptr::from_ref(node) as usize)], 1);
    }
    assert_ne!(
        std::ptr::from_ref(then.as_ref()),
        std::ptr::from_ref(otherwise.as_ref())
    );
}

#[test]
fn complete_pure_item_uses_one_body_visit_instead_of_retyping_unit_statements() {
    let contract = crate::ir::Contract {
        schema_version: crate::ir::SCHEMA_VERSION,
        type_aliases: Vec::new(),
        ledger_fields: Vec::new(),
        constructor: None,
        witnesses: Vec::new(),
        stateful_circuits: Vec::new(),
        circuits: vec![PureCircuit {
            source: None,
            internal: false,
            name: "only_unit".into(),
            parameters: Vec::new(),
            result: Type::Unit,
            body: Expr::Sequence {
                steps: vec![assertion(boolean(true))],
                value: Box::new(Expr::Unit),
            },
        }],
    };
    let (_, visits) = visitation::observe(|| crate::render(&contract).unwrap());
    assert_eq!(visits.len(), 4);
    assert!(visits.values().all(|count| *count == 1));
}
