// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Result role separation preserves input authority and independent validation.
use super::*;

fn contract() -> Contract {
    let mut c: Contract =
        serde_json::from_str(include_str!("../../../../tests/tuple12-result-ir.json")).unwrap();
    c.schema_version = SCHEMA_VERSION;
    c
}
fn checked(c: &Contract) -> Result<CheckedPlan, PlanError> {
    check_with_calls(
        &c.stateful_circuits[0],
        &c.ledger_fields.iter().map(|v| (v.id.as_str(), v)).collect(),
        &HashMap::new(),
        &c.circuits.iter().map(|v| (v.name.as_str(), v)).collect(),
        &c.stateful_circuits
            .iter()
            .map(|v| (v.name.as_str(), v))
            .collect(),
    )
}
#[test]
fn tuple_result_admission_does_not_grant_public_input_encoding() {
    let mut c = contract();
    let ty = c.stateful_circuits[0].result.clone();
    c.stateful_circuits[0].parameters.push(Parameter {
        name: "input".into(),
        ty,
    });
    let Err(PlanError::Unsupported(gap)) = checked(&c) else {
        panic!("public Tuple12 input must remain unsupported");
    };
    assert_eq!(gap.path, "parameters[0]");
    assert_eq!(gap.detail, "unsupported parameter type");
}
#[test]
fn independent_validator_rejects_forged_result_and_live_value_annotations() {
    let checked = checked(&contract()).unwrap();
    assert!(validate::validate(checked.plan(), checked.declarations()).is_ok());
    let mut wrong_result = checked.plan().clone();
    wrong_result.result_type = Type::Field;
    assert!(validate::validate(&wrong_result, checked.declarations()).is_err());
    let mut wrong_value = checked.plan().clone();
    wrong_value.body.result.ty = Type::Field;
    assert!(validate::validate(&wrong_value, checked.declarations()).is_err());
}
