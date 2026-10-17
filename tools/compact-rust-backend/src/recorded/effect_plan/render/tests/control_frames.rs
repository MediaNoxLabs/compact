// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Checked control frames distinguish IR Unit from other Rust () representations.
use super::*;
use crate::ir::{Expr, LedgerField, LedgerFieldKind, StateReturn, StatefulCircuit};

fn typed(id: usize, ty: Type) -> Value {
    Value { id, ty }
}
fn binding(value: Value, operation: Operation) -> Binding {
    Binding {
        path: format!("control[{}]", value.id),
        value,
        operation,
    }
}
fn body(bindings: Vec<Binding>, result: Value) -> Block {
    Block { bindings, result }
}
fn reset(id: usize) -> Binding {
    binding(
        unit(id),
        Operation::CounterReset {
            slot: CounterSlot {
                declaration: LedgerId(0),
                index: 0,
            },
        },
    )
}
fn call(id: usize, argument: Value, parameter: Value, body: Block) -> Binding {
    binding(
        typed(id, body.result.ty.clone()),
        Operation::Call {
            declaration: CallId(0),
            arguments: vec![argument],
            parameters: vec![parameter],
            body,
        },
    )
}
fn branch(id: usize, condition: Value, then: Block, otherwise: Block) -> Binding {
    binding(
        typed(id, then.result.ty.clone()),
        Operation::If {
            condition,
            then,
            otherwise,
        },
    )
}
fn checked(parameters: Vec<Value>, block: Block, helper_type: Type) -> TypedPlan {
    let field = LedgerField {
        id: "counter".into(),
        index: 0,
        path: vec![],
        source: None,
        declaration: LedgerFieldKind::Counter,
    };
    let witness = WitnessDeclaration {
        source: None,
        name: "notify".into(),
        parameters: vec![Parameter {
            name: "input".into(),
            ty: Type::Unit,
        }],
        result: Type::Unit,
    };
    // Signature authority is independent of the hand-built candidate body;
    // these controls exercise checked rendering, not source equivalence.
    let helper = StatefulCircuit {
        source: None,
        name: "helper".into(),
        internal: true,
        parameters: vec![Parameter {
            name: "input".into(),
            ty: helper_type.clone(),
        }],
        result: helper_type,
        actions: vec![],
        return_value: StateReturn::Expression {
            value: Expr::Parameter {
                name: "input".into(),
            },
        },
    };
    let declarations = Declarations::with_calls(
        &std::collections::HashMap::from([("counter", &field)]),
        &std::collections::HashMap::from([("notify", &witness)]),
        &Default::default(),
        &std::collections::HashMap::from([("helper", &helper)]),
    );
    let plan = Plan {
        parameters,
        result_type: block.result.ty.clone(),
        body: block,
    };
    let checked = CheckedPlan::validate(plan, declarations).expect("valid scoped control plan");
    render(&checked).unwrap()
}
fn initializer(statement: &syn::Stmt) -> &syn::Expr {
    let syn::Stmt::Local(local) = statement else {
        panic!("frame binding lost")
    };
    local.init.as_ref().unwrap().expr.as_ref()
}
fn frame_initializer(statement: &syn::Stmt) -> &syn::Expr {
    let syn::Stmt::Local(local) = statement else {
        panic!("frame binding lost")
    };
    let syn::Pat::Ident(target) = &local.pat else {
        panic!("Unit payload carrier retained")
    };
    assert_eq!(target.ident, "frame");
    initializer(statement)
}
fn tail(block: &syn::Block) -> &syn::Expr {
    let Some(syn::Stmt::Expr(expression, None)) = block.stmts.last() else {
        panic!("block result lost")
    };
    expression
}
fn frame_tail(block: &syn::Block) {
    assert!(matches!(tail(block), syn::Expr::Path(p) if p.path.is_ident("frame")));
}
fn check_reset(statement: &syn::Stmt) {
    let syn::Expr::Try(fallible) = frame_initializer(statement) else {
        panic!("reset failure lost")
    };
    let syn::Expr::MethodCall(call) = fallible.expr.as_ref() else {
        panic!("reset lost")
    };
    assert_eq!(call.method, "record_reset");
    assert_eq!(call.args.len(), 1);
    assert!(matches!(call.args.first().unwrap(), syn::Expr::Path(p) if p.path.is_ident("frame")));
}

#[test]
fn unit_calls_return_only_the_frame_without_payload_formals() {
    for effectful in [false, true] {
        let call_body = body(if effectful { vec![reset(2)] } else { vec![] }, unit(1));
        let result = checked(
            vec![],
            body(
                vec![
                    binding(unit(0), Operation::Literal(Literal::Unit)),
                    call(3, unit(0), unit(1), call_body),
                ],
                unit(3),
            ),
            Type::Unit,
        );
        assert_eq!(result.steps.len(), 1);
        let syn::Expr::Block(call) = frame_initializer(&result.steps[0]) else {
            panic!("lexical call frame lost")
        };
        assert_eq!(call.block.stmts.len(), if effectful { 2 } else { 1 });
        if effectful {
            check_reset(&call.block.stmts[0]);
        }
        frame_tail(&call.block);
        assert!(matches!(result.result, syn::Expr::Tuple(t) if t.elems.is_empty()));
    }
}

#[test]
fn unit_conditionals_keep_both_orientations_and_empty_arm() {
    for effectful_then in [None, Some(false), Some(true)] {
        let condition = typed(0, Type::Boolean);
        let then = if effectful_then == Some(true) {
            body(vec![reset(2)], unit(2))
        } else {
            body(vec![], unit(1))
        };
        let otherwise = if effectful_then == Some(false) {
            body(vec![reset(3)], unit(3))
        } else {
            body(vec![], unit(1))
        };
        let result = checked(
            vec![condition.clone()],
            body(
                vec![
                    binding(unit(1), Operation::Literal(Literal::Unit)),
                    branch(4, condition, then, otherwise),
                ],
                unit(4),
            ),
            Type::Unit,
        );
        let syn::Expr::If(choice) = frame_initializer(&result.steps[1]) else {
            panic!("conditional frame lost")
        };
        assert!(
            matches!(choice.cond.as_ref(), syn::Expr::Path(p) if p.path.is_ident("__compact_effect_0"))
        );
        let syn::Expr::Block(otherwise) = choice.else_branch.as_ref().unwrap().1.as_ref() else {
            panic!("else arm lost")
        };
        for (block, effectful) in [
            (&choice.then_branch, effectful_then == Some(true)),
            (&otherwise.block, effectful_then == Some(false)),
        ] {
            assert_eq!(block.stmts.len(), if effectful { 2 } else { 1 });
            if effectful {
                check_reset(&block.stmts[0]);
            }
            frame_tail(block);
        }
    }
}

#[test]
fn unit_call_preserves_nonliteral_and_fallible_argument_producers_in_order() {
    let result = checked(
        vec![],
        body(
            vec![
                binding(unit(0), Operation::Default),
                binding(
                    unit(1),
                    Operation::Witness {
                        declaration: WitnessId(0),
                        arguments: vec![unit(0)],
                    },
                ),
                call(
                    4,
                    unit(1),
                    unit(2),
                    body(vec![binding(unit(3), Operation::Default)], unit(3)),
                ),
            ],
            unit(4),
        ),
        Type::Unit,
    );
    assert_eq!(result.steps.len(), 3);
    assert!(matches!(initializer(&result.steps[0]), syn::Expr::Call(_)));
    let syn::Expr::Try(witness) = initializer(&result.steps[1]) else {
        panic!("argument witness failure lost")
    };
    let syn::Expr::MethodCall(witness) = witness.expr.as_ref() else {
        panic!("argument witness lost")
    };
    let syn::Expr::Closure(callback) = witness.args.first().unwrap() else {
        panic!("witness callback lost")
    };
    let syn::Expr::Block(callback) = callback.body.as_ref() else {
        panic!("callback body lost")
    };
    let syn::Stmt::Expr(syn::Expr::MethodCall(notify), None) = &callback.block.stmts[0] else {
        panic!("callback execution lost")
    };
    assert_eq!(notify.method, "notify");
    assert!(matches!(notify.args.last().unwrap(), syn::Expr::Tuple(t) if t.elems.is_empty()));
    let syn::Expr::Block(call) = frame_initializer(&result.steps[2]) else {
        panic!("call lost")
    };
    assert_eq!(call.block.stmts.len(), 2);
    let syn::Expr::Call(default) = initializer(&call.block.stmts[0]) else {
        panic!("nonliteral default lost")
    };
    let syn::Expr::Path(default) = default.func.as_ref() else {
        panic!("Default operation changed")
    };
    assert_eq!(default.path.segments.last().unwrap().ident, "default");
    frame_tail(&call.block);
}

#[test]
fn conditional_inside_call_keeps_following_frame_operations_in_scope_and_order() {
    let condition = typed(2, Type::Boolean);
    let result = checked(
        vec![],
        body(
            vec![
                binding(unit(0), Operation::Literal(Literal::Unit)),
                call(
                    7,
                    unit(0),
                    unit(1),
                    body(
                        vec![
                            binding(
                                condition.clone(),
                                Operation::Literal(Literal::Boolean(true)),
                            ),
                            branch(
                                5,
                                condition,
                                body(vec![reset(3)], unit(3)),
                                body(vec![], unit(1)),
                            ),
                            reset(6),
                        ],
                        unit(6),
                    ),
                ),
                reset(8),
            ],
            unit(8),
        ),
        Type::Unit,
    );
    assert_eq!(result.steps.len(), 2);
    let syn::Expr::Block(call) = frame_initializer(&result.steps[0]) else {
        panic!("outer call lost")
    };
    assert_eq!(call.block.stmts.len(), 4);
    let syn::Expr::If(choice) = frame_initializer(&call.block.stmts[1]) else {
        panic!("nested conditional lost")
    };
    check_reset(&choice.then_branch.stmts[0]);
    frame_tail(&choice.then_branch);
    check_reset(&call.block.stmts[2]);
    frame_tail(&call.block);
    check_reset(&result.steps[1]);
}

#[test]
fn field_and_empty_tuple_calls_and_branches_keep_typed_payload_carriers() {
    for ty in [Type::Field, Type::Tuple { elements: vec![] }] {
        let input = typed(0, ty.clone());
        let formal = typed(1, ty.clone());
        let returned = typed(2, ty.clone());
        let condition = typed(3, Type::Boolean);
        let result = checked(
            vec![input.clone(), condition.clone()],
            body(
                vec![
                    call(2, input.clone(), formal.clone(), body(vec![], formal)),
                    branch(4, condition, body(vec![], returned), body(vec![], input)),
                ],
                typed(4, ty.clone()),
            ),
            ty.clone(),
        );
        for statement in &result.steps[2..] {
            let syn::Stmt::Local(local) = statement else {
                panic!("typed payload lost")
            };
            let syn::Pat::Type(pattern) = &local.pat else {
                panic!("non-Unit payload erased")
            };
            assert!(matches!(pattern.pat.as_ref(), syn::Pat::Tuple(t) if t.elems.len() == 2));
            let syn::Type::Tuple(types) = pattern.ty.as_ref() else {
                panic!("result type lost")
            };
            assert_eq!(types.elems.len(), 2);
            if matches!(ty, Type::Tuple { .. }) {
                assert!(matches!(&types.elems[1], syn::Type::Tuple(t) if t.elems.is_empty()));
            }
        }
        let syn::Expr::Block(call) = initializer(&result.steps[2]) else {
            panic!("call lost")
        };
        assert_eq!(call.block.stmts.len(), 2); // Formal assignment must remain, including Tuple[].
        assert!(matches!(call.block.stmts[0], syn::Stmt::Local(_)));
        assert!(
            matches!(tail(&call.block), syn::Expr::Tuple(t) if t.elems.len() == 2
            && matches!(&t.elems[1], syn::Expr::Path(p) if p.path.is_ident("__compact_effect_1")))
        );
        let syn::Expr::If(choice) = initializer(&result.steps[3]) else {
            panic!("branch lost")
        };
        assert!(
            matches!(tail(&choice.then_branch), syn::Expr::Tuple(t) if t.elems.len() == 2
            && matches!(&t.elems[1], syn::Expr::Path(p) if p.path.is_ident("__compact_effect_2")))
        );
        let syn::Expr::Block(otherwise) = choice.else_branch.as_ref().unwrap().1.as_ref() else {
            panic!("else lost")
        };
        assert!(
            matches!(tail(&otherwise.block), syn::Expr::Tuple(t) if t.elems.len() == 2
            && matches!(&t.elems[1], syn::Expr::Path(p) if p.path.is_ident("__compact_effect_0")))
        );
    }
}

#[test]
fn unit_root_alias_is_omitted_without_renumbering_other_source_parameters() {
    for ty in [Type::Field, Type::Tuple { elements: vec![] }] {
        let input = typed(1, ty.clone());
        let result = checked(
            vec![unit(0), input.clone(), unit(2)],
            body(vec![reset(3)], input),
            ty,
        );
        assert_eq!(result.steps.len(), 2);
        let syn::Expr::Path(source) = initializer(&result.steps[0]) else {
            panic!("non-Unit source parameter alias lost")
        };
        assert!(source.path.is_ident("__compact_param_1"));
        let syn::Stmt::Local(local) = &result.steps[0] else {
            panic!("non-Unit alias lost")
        };
        let syn::Pat::Type(alias) = &local.pat else {
            panic!("non-Unit alias type lost")
        };
        assert!(
            matches!(alias.pat.as_ref(), syn::Pat::Ident(p) if p.ident == "__compact_effect_1")
        );
        check_reset(&result.steps[1]);
    }
    let result = checked(vec![unit(0)], body(vec![], unit(0)), Type::Unit);
    assert!(result.steps.is_empty());
    assert!(matches!(result.result, syn::Expr::Tuple(t) if t.elems.is_empty()));
}

#[test]
fn unit_assertion_argument_keeps_its_refusal_before_the_call_body() {
    let condition = typed(0, Type::Boolean);
    let result = checked(
        vec![condition.clone()],
        body(
            vec![
                binding(
                    unit(1),
                    Operation::Assert {
                        condition,
                        message: "must hold before reset".into(),
                    },
                ),
                call(4, unit(1), unit(2), body(vec![reset(3)], unit(3))),
            ],
            unit(4),
        ),
        Type::Unit,
    );
    assert_eq!(result.steps.len(), 3);
    let syn::Stmt::Expr(syn::Expr::If(assertion), _) = &result.steps[1] else {
        panic!("assertion refusal lost or reordered")
    };
    assert!(matches!(assertion.cond.as_ref(), syn::Expr::Unary(_)));
    assert!(matches!(
        &assertion.then_branch.stmts[0],
        syn::Stmt::Expr(syn::Expr::Return(_), _)
    ));
    let syn::Expr::Block(call) = frame_initializer(&result.steps[2]) else {
        panic!("call body lost")
    };
    assert_eq!(call.block.stmts.len(), 2);
    check_reset(&call.block.stmts[0]);
    frame_tail(&call.block);
}
