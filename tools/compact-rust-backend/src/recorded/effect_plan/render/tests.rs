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

//! Structural controls for payload-free values; operations must still execute.
mod collections;
mod control_frames;

use super::*;
use crate::ir::{Parameter, Type, WitnessDeclaration};

fn unit(id: usize) -> Value {
    Value { id, ty: Type::Unit }
}

fn output(operation: Operation) -> Binding {
    Binding {
        value: unit(7),
        operation,
        path: "unit".into(),
    }
}

#[test]
fn unit_references_are_literals_but_nonunit_references_keep_their_value() {
    assert!(matches!(value(&unit(7)), syn::Expr::Tuple(t) if t.elems.is_empty()));
    let field = value(&Value {
        id: 7,
        ty: Type::Field,
    });
    let syn::Expr::Path(path) = field else {
        panic!("Field payload was lost")
    };
    assert!(path.path.is_ident("__compact_effect_7"));
}

#[test]
fn literal_unit_has_no_synthetic_binding() {
    let mut steps = Vec::new();
    leaf::render(
        &output(Operation::Literal(Literal::Unit)),
        &Declarations::default(),
        &mut steps,
    )
    .unwrap();
    assert!(steps.is_empty());
}

#[test]
fn nonliteral_unit_initializer_still_executes() {
    let mut steps = Vec::new();
    leaf::render(
        &output(Operation::Default),
        &Declarations::default(),
        &mut steps,
    )
    .unwrap();
    assert_eq!(steps.len(), 1);
    let syn::Stmt::Local(local) = &steps[0] else {
        panic!("initializer dropped")
    };
    assert!(matches!(
        local.init.as_ref().unwrap().expr.as_ref(),
        syn::Expr::Call(_)
    ));
}

#[test]
fn unit_assertion_retains_its_condition_and_early_failure() {
    let mut steps = Vec::new();
    leaf::render(
        &output(Operation::Assert {
            condition: Value {
                id: 3,
                ty: Type::Boolean,
            },
            message: "stop before next effect".into(),
        }),
        &Declarations::default(),
        &mut steps,
    )
    .unwrap();
    assert_eq!(steps.len(), 1);
    let syn::Stmt::Expr(syn::Expr::If(assertion), None) = &steps[0] else {
        panic!("assertion was removed or reordered")
    };
    let syn::Expr::Unary(condition) = assertion.cond.as_ref() else {
        panic!("assertion condition changed")
    };
    assert!(matches!(condition.op, syn::UnOp::Not(_)));
    let syn::Expr::Path(condition) = condition.expr.as_ref() else {
        panic!("condition payload lost")
    };
    assert!(condition.path.is_ident("__compact_effect_3"));
    let [syn::Stmt::Expr(syn::Expr::Return(failure), Some(_))] =
        assertion.then_branch.stmts.as_slice()
    else {
        panic!("early failure changed")
    };
    let syn::Expr::Call(error) = failure.expr.as_deref().unwrap() else {
        panic!("error constructor lost")
    };
    assert_eq!(error.args.len(), 1);
    assert!(assertion.else_branch.is_none());
}

#[test]
fn unit_witness_argument_and_result_do_not_erase_the_fallible_callback() {
    let witness = WitnessDeclaration {
        source: None,
        name: "notify".into(),
        parameters: vec![Parameter {
            name: "input".into(),
            ty: Type::Unit,
        }],
        result: Type::Unit,
    };
    let declarations = Declarations::new(
        &Default::default(),
        &std::collections::HashMap::from([("notify", &witness)]),
    );
    let mut steps = Vec::new();
    leaf::render(
        &output(Operation::Witness {
            declaration: WitnessId(0),
            arguments: vec![unit(1)],
        }),
        &declarations,
        &mut steps,
    )
    .unwrap();
    assert_eq!(steps.len(), 1);
    let syn::Stmt::Local(local) = &steps[0] else {
        panic!("witness call dropped")
    };
    let syn::Expr::Try(fallible) = local.init.as_ref().unwrap().expr.as_ref() else {
        panic!("witness failure dropped")
    };
    let syn::Expr::MethodCall(call) = fallible.expr.as_ref() else {
        panic!("witness frame lost")
    };
    let syn::Expr::Closure(callback) = call.args.first().unwrap() else {
        panic!("callback lost")
    };
    let syn::Expr::Block(body) = callback.body.as_ref() else {
        panic!("callback body lost")
    };
    let [syn::Stmt::Expr(syn::Expr::MethodCall(call), None)] = body.block.stmts.as_slice() else {
        panic!("witness execution changed")
    };
    assert_eq!(call.method, "notify");
    assert_eq!(call.args.len(), 2);
    assert!(matches!(call.args.last().unwrap(), syn::Expr::Tuple(t) if t.elems.is_empty()));
}
