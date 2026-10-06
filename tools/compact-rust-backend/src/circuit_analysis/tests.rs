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
use crate::ir::{LocalBinding, NativeWitnessBuiltin, Type};

fn circuit(name: &str, actions: Vec<StateAction>) -> StatefulCircuit {
    StatefulCircuit {
        name: name.into(),
        source: None,
        internal: false,
        parameters: vec![],
        result: Type::Unit,
        return_value: StateReturn::Unit,
        actions,
    }
}
fn witness() -> Expr {
    Expr::WitnessCall {
        name: "user".into(),
        arguments: vec![],
    }
}
fn native() -> Expr {
    Expr::NativeWitnessCall {
        builtin: NativeWitnessBuiltin::OwnPublicKey,
    }
}
fn expression(value: Expr) -> StateAction {
    StateAction::Expression { value }
}
fn call(name: &str) -> StateAction {
    StateAction::CircuitCall {
        name: name.into(),
        arguments: vec![],
    }
}
fn facts(root: &StatefulCircuit, all: &[StatefulCircuit]) -> (bool, bool) {
    let map = all.iter().map(|c| (c.name.as_str(), c)).collect();
    (
        circuit_uses_witness(root, &map, &mut HashSet::new()).unwrap(),
        circuit_emits_native_private_output(root, &map, &mut HashSet::new()).unwrap(),
    )
}
#[test]
fn user_witness_and_native_private_output_are_distinct_facts() {
    for (actions, expected) in [
        (vec![], (false, false)),
        (vec![expression(witness())], (true, false)),
        (vec![expression(native())], (false, true)),
        (
            vec![expression(witness()), expression(native())],
            (true, true),
        ),
    ] {
        let c = circuit("root", actions);
        assert_eq!(facts(&c, std::slice::from_ref(&c)), expected);
    }
}
#[test]
fn nested_arguments_unused_bindings_and_commitment_openings_are_visited() {
    for (effect, expected) in [(witness(), (true, false)), (native(), (false, true))] {
        let values = [
            Expr::Call {
                name: "not_declared_here".into(),
                arguments: vec![effect.clone()],
            },
            Expr::Let {
                bindings: vec![LocalBinding {
                    name: "unused".into(),
                    ty: Type::Field,
                    value: effect.clone(),
                }],
                body: Box::new(Expr::Unit),
            },
            Expr::TransientCommit {
                value: Box::new(Expr::FieldLiteral { value: "1".into() }),
                opening: Box::new(effect.clone()),
            },
            Expr::PersistentCommit {
                value: Box::new(Expr::FieldLiteral { value: "1".into() }),
                opening: Box::new(effect),
            },
        ];
        for value in values {
            let c = circuit("root", vec![expression(value)]);
            assert_eq!(facts(&c, std::slice::from_ref(&c)), expected);
        }
    }
}
#[test]
fn both_conditional_arms_are_analyzed_without_constant_folding() {
    for swap in [false, true] {
        for (effect, expected) in [(witness(), (true, false)), (native(), (false, true))] {
            let (then, otherwise) = if swap {
                (Expr::Unit, effect)
            } else {
                (effect, Expr::Unit)
            };
            let c = circuit(
                "root",
                vec![expression(Expr::If {
                    condition: Box::new(Expr::Boolean { value: true }),
                    then: Box::new(then),
                    otherwise: Box::new(otherwise),
                })],
            );
            assert_eq!(facts(&c, std::slice::from_ref(&c)), expected);
        }
    }
}
#[test]
fn effectful_return_bindings_actions_and_results_are_analyzed() {
    for (effect, expected) in [(witness(), (true, false)), (native(), (false, true))] {
        let plans = [
            ReturnPlan::Let {
                bindings: vec![LocalBinding {
                    name: "unused".into(),
                    ty: Type::Field,
                    value: effect.clone(),
                }],
                result: Box::new(ReturnPlan::Value { value: Expr::Unit }),
            },
            ReturnPlan::Sequence {
                actions: vec![expression(effect.clone())],
                result: Box::new(ReturnPlan::Value { value: Expr::Unit }),
            },
            ReturnPlan::Conditional {
                condition: Expr::Boolean { value: true },
                then: Box::new(ReturnPlan::Value { value: Expr::Unit }),
                otherwise: Box::new(ReturnPlan::Value { value: effect }),
            },
        ];
        for body in plans {
            let mut c = circuit("root", vec![]);
            c.return_value = StateReturn::Effectful { body };
            assert_eq!(facts(&c, std::slice::from_ref(&c)), expected);
        }
    }
}
#[test]
fn transitive_diamond_and_repeated_calls_do_not_capture_unrelated_declarations() {
    let leaf = circuit("leaf", vec![expression(witness()), expression(native())]);
    let left = circuit("left", vec![call("leaf")]);
    let right = circuit("right", vec![call("leaf")]);
    let root = circuit("root", vec![call("left"), call("right"), call("left")]);
    let plain = circuit("plain", vec![]);
    let all = [root.clone(), left, right, leaf, plain.clone()];
    assert_eq!(facts(&root, &all), (true, true));
    assert_eq!(facts(&plain, &all), (false, false));
}
#[test]
fn direct_and_mutual_recursion_are_errors_for_both_analyses() {
    for all in [
        vec![circuit("a", vec![call("a")])],
        vec![circuit("a", vec![call("b")]), circuit("b", vec![call("a")])],
    ] {
        let map = all.iter().map(|c| (c.name.as_str(), c)).collect();
        assert!(
            matches!(circuit_uses_witness(&all[0],&map,&mut HashSet::new()),Err(RenderError::UnsupportedStatefulCall(name)) if name=="a")
        );
        assert!(
            matches!(circuit_emits_native_private_output(&all[0],&map,&mut HashSet::new()),Err(RenderError::UnsupportedStatefulCall(name)) if name=="a")
        );
    }
}
#[test]
fn expression_queries_follow_declared_calls_and_leave_unknown_validation_to_renderer() {
    let c = circuit("helper", vec![expression(witness())]);
    let map = HashMap::from([("helper", &c)]);
    let known = Expr::Call {
        name: "helper".into(),
        arguments: vec![],
    };
    assert!(expression_requires_witness(&known, &map).unwrap());
    assert!(expression_contains_stateful_call(&known, &map));
    let unknown = Expr::Call {
        name: "unknown".into(),
        arguments: vec![],
    };
    assert!(!expression_requires_witness(&unknown, &map).unwrap());
    assert!(!expression_contains_stateful_call(&unknown, &map));
}

#[test]
fn action_scopes_and_native_statement_preserve_dependency_kinds() {
    for (effect, expected) in [(witness(), (true, false)), (native(), (false, true))] {
        for action in [
            StateAction::Let {
                bindings: vec![LocalBinding {
                    name: "unused".into(),
                    ty: Type::Field,
                    value: effect.clone(),
                }],
                action: Box::new(StateAction::Sequence { actions: vec![] }),
            },
            StateAction::If {
                condition: Expr::Boolean { value: true },
                then: Box::new(expression(effect.clone())),
                otherwise: Box::new(StateAction::Sequence { actions: vec![] }),
            },
            StateAction::If {
                condition: Expr::Boolean { value: true },
                then: Box::new(StateAction::Sequence { actions: vec![] }),
                otherwise: Box::new(expression(effect)),
            },
        ] {
            let c = circuit("root", vec![action]);
            assert_eq!(facts(&c, std::slice::from_ref(&c)), expected);
        }
    }
    let c = circuit(
        "root",
        vec![StateAction::NativeWitnessCall {
            builtin: NativeWitnessBuiltin::OwnPublicKey,
        }],
    );
    assert_eq!(facts(&c, std::slice::from_ref(&c)), (false, true));
}
