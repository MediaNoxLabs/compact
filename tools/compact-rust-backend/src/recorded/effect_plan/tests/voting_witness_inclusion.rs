// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Finite whole-voting inclusion probes for result-only witness domain expansion.
//! Old closure acceptance is mandatory before every checked-plan success.
use super::*;

fn witness() -> Expr {
    Expr::WitnessCall {
        name: "local_state".into(),
        arguments: vec![],
    }
}
fn set_result(contract: &mut Contract, ty: Type) {
    let declaration = contract
        .witnesses
        .iter_mut()
        .find(|w| w.name == "local_state")
        .unwrap();
    assert!(declaration.parameters.is_empty());
    declaration.result = ty;
}
fn phase_value(contract: &mut Contract, expression: Expr) {
    let Expr::If { then, .. } = phase_assertion(contract) else {
        panic!("retained phase conditional")
    };
    **then = expression;
}
fn wide() -> Type {
    Type::Unsigned {
        max: "340282366920938463463374607431768211456".into(),
    }
}
fn tuple(length: usize) -> Type {
    Type::Tuple {
        elements: vec![Type::Boolean; length],
    }
}
fn vector(element: Type, length: usize) -> Type {
    Type::Vector {
        element: Box::new(element),
        length,
    }
}
fn product(payload: Type) -> Type {
    Type::Struct {
        name: "VotingWitnessProduct".into(),
        fields: vec![
            StructField {
                name: "allowed".into(),
                ty: Type::Boolean,
            },
            StructField {
                name: "payload".into(),
                ty: payload,
            },
        ],
    }
}
fn included(contract: &Contract, label: &str) {
    let env = Environment::new(contract);
    env.old_first(label);
    let checked = env
        .checked("vote_commit")
        .unwrap_or_else(|gap| panic!("{label}: whole old-accepted voting root refused: {gap:?}"));
    render(&checked).unwrap_or_else(|gap| {
        panic!("{label}: complete checked voting root failed rendering: {gap:?}")
    });
}

#[test]
fn whole_old_voting_struct_witness_results_cover_legacy_tuple_vector_and_wide_members() {
    let payloads = [
        tuple(1),
        tuple(8),
        vector(Type::Boolean, 0),
        vector(Type::Field, 3),
        vector(tuple(8), 2),
        wide(),
        Type::Struct {
            name: "NestedWitnessPayload".into(),
            fields: vec![
                StructField {
                    name: "items".into(),
                    ty: vector(wide(), 2),
                },
                StructField {
                    name: "point".into(),
                    ty: Type::JubjubPoint,
                },
            ],
        },
    ];
    for payload in payloads {
        let ty = product(payload);
        let mut contract = source();
        set_result(&mut contract, ty.clone());
        phase_value(
            &mut contract,
            Expr::StructField {
                value: Box::new(witness()),
                field: "allowed".into(),
                index: 0,
            },
        );
        included(&contract, &format!("projected witness {ty:?}"));
    }
}

// Two phase witnesses replace the single phase + secret pair. This retains the
// old exact total4 without introducing an unsupported tuple literal observation.
fn replace_secret_with_literal(contract: &mut Contract) {
    fn replace(value: &mut Json) -> usize {
        match value {
            Json::Object(fields)
                if fields.get("kind").and_then(Json::as_str) == Some("witness_call")
                    && fields.get("name").and_then(Json::as_str) == Some("local_secret_key") =>
            {
                *value = serde_json::to_value(bytes(23)).unwrap();
                1
            }
            Json::Object(fields) => fields.values_mut().map(replace).sum(),
            Json::Array(values) => values.iter_mut().map(replace).sum(),
            _ => 0,
        }
    }
    let root = contract
        .stateful_circuits
        .iter_mut()
        .find(|r| r.name == "vote_commit")
        .unwrap();
    let mut value = serde_json::to_value(&*root).unwrap();
    assert_eq!(replace(&mut value), 1);
    *root = serde_json::from_value(value).unwrap();
}

#[test]
fn whole_old_voting_direct_product_witness_results_preserve_complete_effect_counts() {
    for ty in [
        tuple(1),
        tuple(2),
        tuple(8),
        vector(Type::Boolean, 0),
        vector(Type::Field, 3),
        vector(tuple(2), 2),
    ] {
        let mut contract = source();
        set_result(&mut contract, ty.clone());
        replace_secret_with_literal(&mut contract);
        phase_value(
            &mut contract,
            Expr::Equal {
                left: Box::new(witness()),
                right: Box::new(witness()),
            },
        );
        included(&contract, &format!("direct product witness {ty:?}"));
    }
}

#[test]
fn whole_old_voting_direct_wide_witness_result_compares_with_valid_native_literal() {
    let mut contract = source();
    let ty = wide();
    set_result(&mut contract, ty.clone());
    let Type::Unsigned { max } = ty else {
        unreachable!()
    };
    phase_value(
        &mut contract,
        Expr::Equal {
            left: Box::new(witness()),
            right: Box::new(Expr::UnsignedLiteral {
                value: "1".into(),
                max,
            }),
        },
    );
    included(&contract, "direct wide unsigned witness");
}
