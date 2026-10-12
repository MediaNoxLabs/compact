// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Unit routing delegates to the same qualified public ABI as checked plans.
use super::*;
use crate::ir::StructField;
fn tuple(n: usize) -> Type {
    Type::Tuple {
        elements: vec![Type::Field; n],
    }
}
fn vector(ty: Type, n: usize) -> Type {
    Type::Vector {
        element: Box::new(ty),
        length: n,
    }
}
fn named(ty: Type) -> Type {
    Type::Struct {
        name: "Envelope".into(),
        fields: vec![StructField {
            name: "payload".into(),
            ty,
        }],
    }
}
fn unsigned(max: &str) -> Type {
    Type::Unsigned { max: max.into() }
}
fn with_input(ty: Type) -> Contract {
    let mut c = subject();
    c.stateful_circuits[0].parameters = vec![Parameter {
        name: "input".into(),
        ty,
    }];
    c
}
fn assert_actual(c: &Contract) {
    let (output, rows) = trace::capture_json(|| crate::render_with_capabilities(c));
    let output = output.unwrap();
    assert!(
        output.capabilities.circuits[0].recorded && output.capabilities.circuits[0].observed_call
    );
    let root = rows
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["circuit"] == "root" && r["phase"] == "public_emission" && r["helper"] == false)
        .unwrap();
    assert_eq!(root["body_owner"], "algebra.ordinary");
    let ProfileAttempt::Admitted(plan) = attempt_contract(c).unwrap() else {
        panic!("expected checked Unit")
    };
    let steps = &plan.steps;
    assert!(
        quote::quote!(#(#steps)*)
            .to_string()
            .contains("record_increment")
    );
}
#[test]
fn newly_reachable_unit_interfaces_use_official_public_representations() {
    for ty in [
        Type::Unit,
        Type::OpaqueBytes,
        tuple(0),
        tuple(9),
        tuple(10),
        tuple(11),
        unsigned("18446744073709551616"),
        unsigned("340282366920938463463374607431768211455"),
        unsigned("340282366920938463463374607431768211456"),
        unsigned("452312848583266388373324160190187140051835877600158453279131187530910662655"),
        vector(Type::OpaqueBytes, 0),
        vector(Type::OpaqueBytes, 2),
        vector(tuple(11), 2),
        named(Type::OpaqueBytes),
        named(Type::Unit),
        named(tuple(8)),
        Type::Struct {
            name: "Empty".into(),
            fields: vec![],
        },
    ] {
        let c = with_input(ty);
        assert_actual(&c);
    }
}
#[test]
fn unencodable_or_noncanonical_public_interfaces_get_checked_refusals() {
    for ty in [
        tuple(12),
        vector(tuple(12), 0),
        named(tuple(9)),
        named(vector(tuple(9), 0)),
        Type::LedgerMap {
            key: Box::new(Type::Field),
            value: Box::new(Type::Field),
        },
        unsigned("01"),
        unsigned("-1"),
        unsigned("abc"),
        unsigned("452312848583266388373324160190187140051835877600158453279131187530910662656"),
    ] {
        let c = with_input(ty);
        assert!(
            matches!(attempt_contract(&c).unwrap(), ProfileAttempt::Rejected(_)),
            "invalid interface does not bypass checked policy as NotApplicable"
        );
    }
    let mut c = with_input(named(Type::Field));
    c.stateful_circuits[0].parameters.push(Parameter {
        name: "conflicting".into(),
        ty: named(Type::Boolean),
    });
    assert!(
        matches!(attempt_contract(&c).unwrap(), ProfileAttempt::Rejected(_)),
        "canonical named definitions cannot disagree"
    );
    let mut c = with_input(Type::OpaqueBytes);
    let duplicate = c.stateful_circuits[0].parameters[0].clone();
    c.stateful_circuits[0].parameters.push(duplicate);
    assert!(
        matches!(attempt_contract(&c).unwrap(), ProfileAttempt::Rejected(_)),
        "duplicate declared parameters are rejected"
    );
}
#[test]
fn wider_interfaces_do_not_supply_public_effect_evidence_or_change_return_forms() {
    for ty in [Type::OpaqueBytes, tuple(11), named(Type::Unit)] {
        let mut c = with_input(ty);
        c.stateful_circuits[0].actions = vec![StateAction::Assert {
            condition: Expr::Boolean { value: true },
            message: "private-only".into(),
        }];
        assert!(!effect_plan::has_public_program(&checked(&c)));
        assert_supported(&c); // Execution availability is independent of public evidence.
        c.stateful_circuits[0].actions = vec![public_action()];
        c.stateful_circuits[0].return_value = StateReturn::Expression { value: Expr::Unit };
        assert!(matches!(
            attempt_contract(&c).unwrap(),
            ProfileAttempt::NotApplicable
        ));
        c.stateful_circuits[0].return_value = StateReturn::Unit;
        c.stateful_circuits[0].result = Type::Boolean;
        assert!(matches!(
            attempt_contract(&c).unwrap(),
            ProfileAttempt::NotApplicable
        ));
    }
}
