// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Old-family value controls qualify checked local ownership and equality.
use crate::ir::{Contract, Expr, LocalBinding, PureCircuit, StateAction, Type};
use crate::recorded::{effect_plan, typed_plan};
use quote::quote;

fn source() -> Contract {
    serde_json::from_str(include_str!(
        "../../../../tests/shielded-receive-schema20-ir.json"
    ))
    .unwrap()
}
fn qualify(contract: &Contract, label: &str) {
    let root = contract
        .stateful_circuits
        .iter()
        .find(|c| c.name == "accept")
        .unwrap();
    let ledger = contract
        .ledger_fields
        .iter()
        .map(|f| (f.id.as_str(), f))
        .collect();
    let witnesses = contract
        .witnesses
        .iter()
        .map(|w| (w.name.as_str(), w))
        .collect();
    let pure = contract
        .circuits
        .iter()
        .map(|p| (p.name.as_str(), p))
        .collect();
    let circuits = contract
        .stateful_circuits
        .iter()
        .map(|c| (c.name.as_str(), c))
        .collect();
    let old = typed_plan::lower_shielded_receive(root, &ledger, &witnesses, &pure, &circuits)
        .unwrap_or_else(|| panic!("old-FIRST failed: {label}"));
    let steps = old.steps;
    let tokens = quote!(#(#steps)*).to_string();
    assert_eq!(tokens.matches("kernel_self").count(), 1, "{label}");
    assert_eq!(tokens.matches("create_zswap_output").count(), 1, "{label}");
    assert_eq!(tokens.matches("CoinReceive").count(), 1, "{label}");
    let checked = effect_plan::lower_with_calls(root, &ledger, &witnesses, &pure, &circuits);
    checked.unwrap_or_else(|error| panic!("checked inclusion {label}: {error:?}"));
    // The current public pipeline may still use the old family. Native rendering
    // and that legacy recording must succeed, preventing malformed-IR false gaps.
    let public = crate::render_with_capabilities(contract).unwrap();
    let entry = public
        .capabilities
        .circuits
        .iter()
        .find(|c| c.name == "accept")
        .unwrap();
    assert!(entry.recorded && entry.observed_call, "{label}");
}
fn prepend(contract: &mut Contract, ty: Type, value: Expr) {
    let helper = contract
        .stateful_circuits
        .iter_mut()
        .find(|c| c.name == "receiveShielded")
        .unwrap();
    let StateAction::Let { bindings, .. } = &mut helper.actions[0] else {
        panic!("retained helper lexical structure changed")
    };
    bindings.insert(
        0,
        LocalBinding {
            name: "unused_owned_domain_probe".into(),
            ty,
            value,
        },
    );
}
#[test]
fn unchanged_receive_is_accepted_by_both_engines() {
    qualify(&source(), "unchanged original");
}
#[test]
fn old_receive_accepts_opaque_string_from_a_pure_interface() {
    let mut contract = source();
    contract.circuits.push(PureCircuit {
        source: None,
        name: "owned_empty_string".into(),
        internal: true,
        parameters: vec![],
        result: Type::OpaqueString,
        body: Expr::Default {
            ty: Type::OpaqueString,
        },
    });
    prepend(
        &mut contract,
        Type::OpaqueString,
        Expr::Call {
            name: "owned_empty_string".into(),
            arguments: vec![],
        },
    );
    qualify(&contract, "OpaqueString pure result to unused local");
}
#[test]
fn old_receive_accepts_a_tuple_coerced_to_a_fixed_vector_local() {
    let mut contract = source();
    let ty = Type::Vector {
        element: Box::new(Type::Field),
        length: 2,
    };
    prepend(
        &mut contract,
        ty.clone(),
        Expr::Coerce {
            value: Box::new(Expr::Tuple {
                elements: vec![
                    Expr::FieldLiteral { value: "7".into() },
                    Expr::FieldLiteral { value: "11".into() },
                ],
            }),
            ty,
        },
    );
    qualify(&contract, "Tuple<Field,Field> to Vector<Field,2> local");
}
#[test]
fn old_receive_accepts_default_struct_with_owned_and_vector_children() {
    let mut contract = source();
    let ty = Type::Struct {
        name: "OwnedProbe".into(),
        fields: vec![
            crate::ir::StructField {
                name: "text".into(),
                ty: Type::OpaqueString,
            },
            crate::ir::StructField {
                name: "bytes".into(),
                ty: Type::OpaqueBytes,
            },
            crate::ir::StructField {
                name: "values".into(),
                ty: Type::Vector {
                    element: Box::new(Type::Field),
                    length: 2,
                },
            },
        ],
    };
    prepend(&mut contract, ty.clone(), Expr::Default { ty });
    qualify(
        &contract,
        "Default<Struct<OpaqueString,OpaqueBytes,Vector<Field,2>>>",
    );
}

#[test]
fn receive_point_equality_is_accepted_by_both_engines() {
    let point_box = Type::Struct {
        name: "ReceivePointBox".into(),
        fields: vec![crate::ir::StructField {
            name: "point".into(),
            ty: Type::JubjubPoint,
        }],
    };
    for projected in [false, true] {
        let mut contract = source();
        let operand = if projected {
            Expr::StructField {
                value: Box::new(Expr::Default {
                    ty: point_box.clone(),
                }),
                field: "point".into(),
                index: 0,
            }
        } else {
            Expr::Default {
                ty: point_box.clone(),
            }
        };
        prepend(
            &mut contract,
            Type::Boolean,
            Expr::Equal {
                left: Box::new(operand.clone()),
                right: Box::new(operand),
            },
        );
        qualify(
            &contract,
            if projected {
                "projected point equality"
            } else {
                "point-containing struct equality"
            },
        );
    }
}

#[test]
fn receive_empty_named_struct_is_accepted_by_both_engines() {
    let empty = Type::Struct {
        name: "ReceiveEmpty".into(),
        fields: vec![],
    };
    for value in [
        Expr::Default { ty: empty.clone() },
        Expr::StructLiteral {
            ty: empty.clone(),
            fields: vec![],
        },
    ] {
        let mut contract = source();
        prepend(&mut contract, empty.clone(), value);
        qualify(&contract, "empty named struct");
    }
}

#[test]
fn receive_direct_enum_and_identity_coerce_have_checked_support() {
    let mut contract = source();
    let ty = Type::Enum {
        name: "ReceiveState".into(),
        variants: vec!["Initial".into(), "Ready".into()],
    };
    prepend(
        &mut contract,
        ty.clone(),
        Expr::Coerce {
            value: Box::new(Expr::EnumVariant {
                ty: ty.clone(),
                variant: "Ready".into(),
            }),
            ty,
        },
    );
    qualify(
        &contract,
        "direct enum variant through exact identity coercion",
    );
}
