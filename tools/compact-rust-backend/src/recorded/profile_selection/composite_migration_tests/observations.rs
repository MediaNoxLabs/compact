// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;
fn retained() -> Contract {
    serde_json::from_str(include_str!(
        "../../../../tests/field-observation-schema20-ir.json"
    ))
    .unwrap()
}
#[test]
fn retained_field_observation_exports_and_internal_struct_helper_are_old_first() {
    let contract = retained();
    assert_eq!(
        contract
            .stateful_circuits
            .iter()
            .map(|c| c.name.as_str())
            .collect::<Vec<_>>(),
        [
            "direct",
            "snapshot",
            "field_helper",
            "snapshot_helper",
            "via_field_helper",
            "via_snapshot_helper",
            "pair",
            "selected",
            "optional"
        ]
    );
    for name in [
        "snapshot",
        "snapshot_helper",
        "via_field_helper",
        "via_snapshot_helper",
        "pair",
        "selected",
        "optional",
    ] {
        included(&contract, name, "FieldObservations retained control");
    }
}
#[test]
fn field_observations_allow_rich_formals_coerce_lets_and_unused_read_arguments() {
    let mut contract = retained();
    let root = contract.stateful_circuits[1].clone();
    let result = root.result.clone();
    contract.stateful_circuits[1].parameters = vec![
        Parameter {
            name: "saved".into(),
            ty: result.clone(),
        },
        Parameter {
            name: "selected".into(),
            ty: Type::Boolean,
        },
        Parameter {
            name: "seed".into(),
            ty: Type::Field,
        },
    ];
    let StateReturn::Expression { value } = root.return_value else {
        unreachable!()
    };
    contract.stateful_circuits[1].return_value = StateReturn::Expression {
        value: local(
            "observed",
            result.clone(),
            value,
            Expr::If {
                condition: Box::new(parameter("selected")),
                then: Box::new(Expr::Coerce {
                    value: Box::new(parameter("saved")),
                    ty: result,
                }),
                otherwise: Box::new(parameter("observed")),
            },
        ),
    };
    included(
        &contract,
        "snapshot",
        "rich Field/Boolean/Struct parameters and lexical coerce",
    );
    let mut contract = retained();
    contract.stateful_circuits[2].parameters = vec![
        Parameter {
            name: "unused_first".into(),
            ty: Type::Field,
        },
        Parameter {
            name: "unused_second".into(),
            ty: Type::Field,
        },
    ];
    let StateReturn::Expression {
        value: Expr::StructLiteral { fields, .. },
    } = &mut contract.stateful_circuits[4].return_value
    else {
        unreachable!()
    };
    fields[0] = call(
        "field_helper",
        vec![
            Expr::CellRead {
                field: "second_cell".into(),
                index: 1,
            },
            Expr::CellRead {
                field: "first_cell".into(),
                index: 0,
            },
        ],
    );
    included(
        &contract,
        "via_field_helper",
        "unused actual reads still evaluate",
    );
    let context = Context::new(&contract);
    let source = tokens(&context.algebra(&contract.stateful_circuits[4]).unwrap());
    assert_eq!(source.matches("record_read").count(), 3);
    assert!(source.find("second_cell").unwrap() < source.find("first_cell").unwrap());
}
#[test]
fn malformed_field_reference_and_helper_signature_are_refused() {
    let raw: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../tests/field-observation-schema20-ir.json"
    ))
    .unwrap();
    let cases = [
        (1, "/ledger_fields/0/index", serde_json::json!(1)),
        (
            1,
            "/ledger_fields/0/declaration/ty",
            serde_json::json!({"kind":"boolean"}),
        ),
        (
            1,
            "/stateful_circuits/1/return_value/value/fields/0",
            serde_json::json!({"kind":"parameter","name":"capture"}),
        ),
        (
            4,
            "/stateful_circuits/2/parameters",
            serde_json::json!([{"name":"missing","ty":{"kind":"field"}}]),
        ),
        (
            5,
            "/stateful_circuits/3/return_value/value",
            serde_json::json!({"kind":"call","name":"via_snapshot_helper","arguments":[]}),
        ),
    ];
    for (index, path, value) in cases {
        let mut source = raw.clone();
        *source.pointer_mut(path).unwrap() = value;
        let contract: Contract = serde_json::from_value(source).unwrap();
        let context = Context::new(&contract);
        let root = &contract.stateful_circuits[index];
        assert!(context.old(root).is_none(), "old negative {path}");
        assert!(
            matches!(
                context.algebra(root),
                Err(effect_plan::PlanError::Unsupported(_))
            ),
            "new negative {path}"
        );
    }
}
