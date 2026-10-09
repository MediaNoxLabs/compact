// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Three finite candidate inclusion gaps; every case must pass the old evaluator first.
use super::*;
use crate::ir::{Contract, Expr, LocalBinding, StateAction, Type};
use quote::quote;

fn source() -> Contract {
    serde_json::from_str(include_str!(
        "../../../tests/shielded-receive-schema20-ir.json"
    ))
    .unwrap()
}
fn qualify(contract: &Contract, expect_checked: bool, label: &str) {
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
    if expect_checked {
        checked.unwrap_or_else(|error| panic!("unchanged checked control: {error:?}"));
    } else {
        let Err(effect_plan::PlanError::Unsupported(gap)) = checked else {
            panic!("expected a source domain gap, not an invariant failure: {label}")
        };
        assert!(
            gap.path.contains("callee[receiveShielded]"),
            "{label}: {gap:?}"
        );
        assert!(gap.path.contains("bindings[0]"), "{label}: {gap:?}");
        eprintln!("owned-domain witness {label}: {gap:?}");
    }
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
    qualify(&source(), true, "unchanged original");
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
    qualify(&contract, false, "OpaqueString pure result to unused local");
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
    qualify(
        &contract,
        false,
        "Tuple<Field,Field> to Vector<Field,2> local",
    );
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
        false,
        "Default<Struct<OpaqueString,OpaqueBytes,Vector<Field,2>>>",
    );
}
