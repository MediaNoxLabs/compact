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

//! Pure call signatures, local scope, and conditional typing.

use super::support::identity;
use compact_rust_backend::ir::{Expr, LocalBinding, Parameter, PureCircuit, Type};
use compact_rust_backend::{RenderError, render};

#[test]
fn pure_call_checks_target_arity_and_argument_types() {
    let mut contract = identity(
        Type::Field,
        Expr::Call {
            name: "target".into(),
            arguments: vec![Expr::Parameter {
                name: "value".into(),
            }],
        },
    );
    assert_eq!(
        render(&contract),
        Err(RenderError::UnknownCircuit("target".into()))
    );
    contract.circuits.push(PureCircuit {
        source: None,
        internal: false,
        name: "target".into(),
        parameters: vec![Parameter {
            name: "flag".into(),
            ty: Type::Boolean,
        }],
        result: Type::Field,
        body: Expr::Parameter {
            name: "flag".into(),
        },
    });
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Boolean,
            actual: Type::Field,
        })
    );
    contract.circuits[1].parameters[0].ty = Type::Field;
    if let Expr::Call { arguments, .. } = &mut contract.circuits[0].body {
        arguments.clear();
    }
    assert_eq!(
        render(&contract),
        Err(RenderError::ArgumentCount {
            circuit: "target".into(),
            expected: 1,
            actual: 0,
        })
    );
    if let Expr::Call { arguments, .. } = &mut contract.circuits[0].body {
        arguments.push(Expr::Parameter {
            name: "value".into(),
        });
    }
    assert!(
        render(&contract)
            .unwrap()
            .contains("crate::pure_circuits::target(value)")
    );
}

#[test]
fn local_binding_checks_declared_type_and_scope() {
    let mut contract = identity(
        Type::Field,
        Expr::Let {
            bindings: vec![LocalBinding {
                name: "saved".into(),
                ty: Type::Boolean,
                value: Expr::Parameter {
                    name: "value".into(),
                },
            }],
            body: Box::new(Expr::Parameter {
                name: "saved".into(),
            }),
        },
    );
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Boolean,
            actual: Type::Field,
        })
    );
    let Expr::Let { bindings, .. } = &mut contract.circuits[0].body else {
        unreachable!()
    };
    bindings[0].ty = Type::Field;
    let source = render(&contract).unwrap();
    assert!(source.contains("let __compact_local_saved: runtime::Field = value;"));
    assert!(source.contains("Ok({"));
}

#[test]
fn conditional_requires_boolean_condition_and_equal_branch_types() {
    let mut contract = identity(
        Type::Field,
        Expr::If {
            condition: Box::new(Expr::Parameter {
                name: "value".into(),
            }),
            then: Box::new(Expr::Parameter {
                name: "value".into(),
            }),
            otherwise: Box::new(Expr::Parameter {
                name: "value".into(),
            }),
        },
    );
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Boolean,
            actual: Type::Field,
        })
    );
    let Expr::If {
        condition,
        otherwise,
        ..
    } = &mut contract.circuits[0].body
    else {
        unreachable!()
    };
    **condition = Expr::Boolean { value: true };
    **otherwise = Expr::Boolean { value: false };
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Field,
            actual: Type::Boolean,
        })
    );
}
