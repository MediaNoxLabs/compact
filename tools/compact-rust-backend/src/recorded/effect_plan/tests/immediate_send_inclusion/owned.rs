// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;

fn sample(ty: Type) -> (Type, Expr) {
    let carrier = Type::Struct {
        name: "ImmediateOwnedCarrier".into(),
        fields: vec![StructField {
            name: "item".into(),
            ty: ty.clone(),
        }],
    };
    (
        ty,
        Expr::StructField {
            value: Box::new(Expr::Default { ty: carrier }),
            field: "item".into(),
            index: 0,
        },
    )
}
fn prepend(c: &mut Contract, ty: Type, value: Expr) {
    let body = returned_mut(c, "sendShielded");
    let original = std::mem::replace(body, Expr::Unit);
    *body = Expr::Let {
        bindings: vec![LocalBinding {
            name: "immediate_owned".into(),
            ty,
            value,
        }],
        body: Box::new(original),
    };
}
#[test]
fn whole_old_immediate_send_owned_unused_values_compose() {
    for ty in [
        Type::OpaqueString,
        Type::OpaqueBytes,
        Type::Tuple {
            elements: vec![Type::Field, Type::Boolean],
        },
        Type::Vector {
            element: Box::new(Type::Field),
            length: 2,
        },
    ] {
        let mut c = source();
        let label = format!("owned local {ty:?}");
        let (ty, value) = sample(ty);
        prepend(&mut c, ty, value);
        included(&c, "receive_then_send", &label);
    }
}
#[test]
fn whole_old_immediate_send_owned_helper_and_unselected_call_compose() {
    for unselected in [false, true] {
        let mut c = source();
        let (ty, value) = sample(Type::OpaqueString);
        c.stateful_circuits.push(StatefulCircuit {
            source: None,
            name: "owned_identity".into(),
            internal: true,
            parameters: vec![Parameter {
                name: "input".into(),
                ty: ty.clone(),
            }],
            result: ty.clone(),
            actions: vec![],
            return_value: StateReturn::Expression {
                value: parameter("input"),
            },
        });
        let call = Expr::Call {
            name: "owned_identity".into(),
            arguments: vec![value.clone()],
        };
        prepend(
            &mut c,
            ty,
            if unselected {
                Expr::If {
                    condition: Box::new(Expr::Boolean { value: true }),
                    then: Box::new(value),
                    otherwise: Box::new(call),
                }
            } else {
                call
            },
        );
        included(
            &c,
            "receive_then_send",
            &format!("owned identity/unselected={unselected}"),
        );
    }
}
