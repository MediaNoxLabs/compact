// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;

fn chained(count: usize) -> Contract {
    let mut c = source();
    let mut inner = root(&c).clone();
    inner.name = "chain_0".into();
    inner.internal = true;
    let arguments = inner
        .parameters
        .iter()
        .map(|p| parameter(&p.name))
        .collect::<Vec<_>>();
    c.stateful_circuits.push(inner.clone());
    for i in 1..count {
        let mut next = inner.clone();
        next.name = format!("chain_{i}");
        next.actions = vec![StateAction::CircuitCall {
            name: format!("chain_{}", i - 1),
            arguments: arguments.clone(),
        }];
        c.stateful_circuits.push(next);
    }
    circuit_mut(&mut c, "flow").actions = vec![StateAction::CircuitCall {
        name: format!("chain_{}", count - 1),
        arguments,
    }];
    c
}

#[test]
fn old_grammar_stays_subject_to_existing_bounded_call_resources() {
    included(&chained(8), "flow", "eight nested same-frame helpers");
    let c = chained(20);
    let env = Environment::new(&c);
    assert!(
        env.old(root(&c)).is_some(),
        "old grammar itself has no depth budget"
    );
    assert!(
        matches!(env.checked(root(&c)), Err(PlanError::Unsupported(gap)) if gap.detail.contains("call depth"))
    );
    // This is the existing resource boundary, not a claim of universal grammar inclusion.
    assert!(crate::render_with_capabilities(&c).is_err());
}

#[test]
fn malformed_root_and_helper_scopes_types_arity_and_cycles_fail_closed() {
    for mode in 0..7 {
        let mut c = source();
        match mode {
            0 => {
                let r = circuit_mut(&mut c, "flow");
                r.parameters[1].name = r.parameters[0].name.clone();
            }
            1 => replace_uses(&mut c, "flow", "input", parameter("missing_input")),
            2 => circuit_mut(&mut c, "flow").parameters[0].ty = Type::Field,
            3 => {
                circuit_mut(&mut c, "pair").parameters.pop();
            }
            4 => {
                circuit_mut(&mut c, "pair")
                    .actions
                    .push(StateAction::CircuitCall {
                        name: "pair".into(),
                        arguments: vec![parameter("coin"), parameter("recipient")],
                    });
            }
            5 => {
                circuit_mut(&mut c, "pair")
                    .actions
                    .push(expression(Expr::CreateZswapInput {
                        coin: Box::new(parameter("input")),
                    }));
            }
            6 => {
                let p = circuit_mut(&mut c, "pair").parameters[0].clone();
                circuit_mut(&mut c, "pair").parameters[1].name = p.name;
            }
            _ => unreachable!(),
        }
        let env = Environment::new(&c);
        assert!(
            env.old(root(&c)).is_none(),
            "source error {mode}: old refusal"
        );
        assert!(
            matches!(env.checked(root(&c)), Err(PlanError::Unsupported(_))),
            "source error {mode}: checked refusal"
        );
    }
}

#[test]
fn literal_conditions_and_missing_evidence_counts_are_expansions() {
    let original = source();
    for name in ["produce", "consume", "witness_order"] {
        let env = Environment::new(&original);
        let r = env.stateful[name];
        assert!(env.old(r).is_none(), "{name}: lacks old query requirement");
        render(&env.checked(r).unwrap()).unwrap();
    }
    crate::render_with_capabilities(&original).unwrap();
    for selected in [false, true] {
        let mut c = source();
        replace_uses(
            &mut c,
            "flow",
            "selected",
            Expr::Boolean { value: selected },
        );
        let env = Environment::new(&c);
        assert!(
            env.old(root(&c)).is_none(),
            "literal condition is outside old value grammar"
        );
        render(&env.checked(root(&c)).unwrap()).unwrap();
        crate::render_with_capabilities(&c).unwrap();
    }
}
