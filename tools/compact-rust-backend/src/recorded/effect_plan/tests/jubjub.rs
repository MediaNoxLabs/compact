// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Point-domain admission, operand order and the deliberately unchanged scalar surface.
use super::*;
#[path = "jubjub/validation.rs"]
mod validation;

fn contract() -> Contract {
    serde_json::from_str(include_str!(
        "../../../../tests/jubjub-scalar-cell-schema20-ir.json"
    ))
    .unwrap()
}
fn checked(contract: &Contract, circuit: &StatefulCircuit) -> Result<CheckedPlan, PlanError> {
    check(
        circuit,
        &contract
            .ledger_fields
            .iter()
            .map(|f| (f.id.as_str(), f))
            .collect(),
        &contract
            .witnesses
            .iter()
            .map(|w| (w.name.as_str(), w))
            .collect(),
    )
}
fn candidate(body: Expr, result: Type) -> StatefulCircuit {
    StatefulCircuit {
        source: None,
        name: "point_candidate".into(),
        internal: false,
        parameters: vec![
            Parameter {
                name: "point".into(),
                ty: Type::JubjubPoint,
            },
            Parameter {
                name: "scalar".into(),
                ty: Type::Field,
            },
        ],
        actions: vec![],
        result,
        return_value: StateReturn::Expression { value: body },
    }
}
fn parameter(name: &str) -> Expr {
    Expr::Parameter { name: name.into() }
}
fn generate(scalar: Expr) -> Expr {
    Expr::EcMulGenerator {
        scalar: Box::new(scalar),
    }
}
fn reduce(value: Expr) -> Expr {
    Expr::JubjubScalarFromNative {
        value: Box::new(value),
    }
}
fn multiply(point: Expr, scalar: Expr) -> Expr {
    Expr::EcMul {
        point: Box::new(point),
        scalar: Box::new(scalar),
    }
}
fn add(left: Expr, right: Expr) -> Expr {
    Expr::EcAdd {
        left: Box::new(left),
        right: Box::new(right),
    }
}
fn read() -> Expr {
    Expr::CellRead {
        field: "result".into(),
        index: 0,
    }
}
fn refusal(expression: Expr, result: Type) -> RecordingGap {
    match checked(&contract(), &candidate(expression, result)).unwrap_err() {
        PlanError::Unsupported(gap) => gap,
        other => panic!("expected lowering refusal: {other:?}"),
    }
}

#[test]
fn retained_point_sources_use_the_checked_value_domain() {
    let contract = contract();
    for circuit in &contract.stateful_circuits {
        let checked = checked(&contract, circuit).unwrap();
        assert_eq!(checked.plan().result_type, Type::JubjubPoint);
        let write = checked
            .plan()
            .body
            .bindings
            .iter()
            .find_map(|b| match &b.operation {
                Operation::CellWrite { value, .. } => Some(value),
                _ => None,
            })
            .unwrap();
        assert_eq!(write, &checked.plan().body.result);
        assert!(render(&checked).is_ok());
    }
}

#[test]
fn point_operands_are_evaluated_once_in_left_to_right_order() {
    let mut contract = contract();
    for name in ["left_scalar", "right_scalar"] {
        contract.witnesses.push(WitnessDeclaration {
            source: None,
            name: name.into(),
            parameters: vec![],
            result: Type::Field,
        });
    }
    let witness = |name: &str| Expr::WitnessCall {
        name: name.into(),
        arguments: vec![],
    };
    let circuit = candidate(
        add(
            generate(reduce(witness("left_scalar"))),
            multiply(read(), witness("right_scalar")),
        ),
        Type::JubjubPoint,
    );
    let checked = checked(&contract, &circuit).unwrap();
    let bindings = &checked.plan().body.bindings;
    assert_eq!(bindings.len(), 7);
    assert!(matches!(bindings[0].operation, Operation::Witness { .. }));
    assert!(matches!(
        bindings[1].operation,
        Operation::JubjubScalarFromNative(_)
    ));
    assert!(matches!(
        bindings[2].operation,
        Operation::EcMulGenerator(_)
    ));
    assert!(matches!(bindings[3].operation, Operation::CellRead(_)));
    assert!(matches!(bindings[4].operation, Operation::Witness { .. }));
    assert!(matches!(bindings[5].operation, Operation::EcMul { .. }));
    assert!(matches!(bindings[6].operation, Operation::EcAdd { .. }));
    let steps = render(&checked).unwrap().steps;
    let syntax = quote::quote!(#(#steps)*).to_string();
    for method in [
        "left_scalar",
        "right_scalar",
        "jubjub_scalar_from_native",
        "ec_mul_generator",
        "ec_mul (",
        "ec_add",
        "record_read",
    ] {
        assert_eq!(syntax.matches(method).count(), 1, "{method}: {syntax}");
    }
    // Shared adapters preserve fallible multiplication and infallible reduction/addition.
    assert!(syntax.contains(&format!(
        "ec_mul_generator (__compact_effect_{}) ?",
        bindings[1].value.id
    )));
    assert!(syntax.contains(&format!(
        "ec_mul (__compact_effect_{} , __compact_effect_{}) ?",
        bindings[3].value.id, bindings[4].value.id
    )));
}

#[test]
fn invalid_jubjub_operands_report_the_first_exact_operand_path() {
    for (expression, result, path) in [
        (reduce(parameter("point")), Type::Field, "value"),
        (generate(parameter("point")), Type::JubjubPoint, "scalar"),
        (
            multiply(parameter("scalar"), parameter("point")),
            Type::JubjubPoint,
            "point",
        ),
        (
            multiply(parameter("point"), parameter("point")),
            Type::JubjubPoint,
            "scalar",
        ),
        (
            add(parameter("scalar"), parameter("scalar")),
            Type::JubjubPoint,
            "left",
        ),
        (
            add(parameter("point"), parameter("scalar")),
            Type::JubjubPoint,
            "right",
        ),
    ] {
        assert_eq!(
            refusal(expression, result).path,
            format!("return_value.value.{path}")
        );
    }
    let unknown = Expr::Call {
        name: "unsupported".into(),
        arguments: vec![],
    };
    let gap = refusal(multiply(unknown.clone(), unknown), Type::JubjubPoint);
    assert_eq!(gap.path, "return_value.value.point");
    assert_eq!(gap.ir_node, "Expr::Call");
}

#[test]
fn point_equality_does_not_expand_scalar_arithmetic() {
    let equal = candidate(
        Expr::Equal {
            left: Box::new(parameter("point")),
            right: Box::new(parameter("point")),
        },
        Type::Boolean,
    );
    super::super::render(&checked(&contract(), &equal).unwrap()).unwrap();
    refusal(
        Expr::Equal {
            left: Box::new(parameter("point")),
            right: Box::new(parameter("scalar")),
        },
        Type::Boolean,
    );
    for (expression, result) in [
        (
            Expr::Add {
                left: Box::new(parameter("point")),
                right: Box::new(parameter("point")),
            },
            Type::Field,
        ),
        (
            Expr::UnsignedAdd {
                max: "65535".into(),
                left: Box::new(parameter("point")),
                right: Box::new(parameter("point")),
            },
            validate::uint("65535"),
        ),
        (
            Expr::FieldCast {
                value: Box::new(parameter("point")),
            },
            Type::Field,
        ),
        (
            Expr::UnsignedCast {
                value: Box::new(parameter("point")),
                max: "65535".into(),
            },
            validate::uint("65535"),
        ),
    ] {
        refusal(expression, result);
    }
}

#[test]
fn point_values_compose_through_surface_branches_and_witnesses() {
    let branch = candidate(
        Expr::If {
            condition: Box::new(Expr::Boolean { value: true }),
            then: Box::new(parameter("point")),
            otherwise: Box::new(parameter("point")),
        },
        Type::JubjubPoint,
    );
    let plan = checked(&contract(), &branch).unwrap();
    super::super::render(&plan).unwrap();
    let mut contract = contract();
    contract.witnesses.push(WitnessDeclaration {
        source: None,
        name: "point_argument".into(),
        parameters: vec![Parameter {
            name: "p".into(),
            ty: Type::JubjubPoint,
        }],
        result: Type::Field,
    });
    let circuit = candidate(
        Expr::WitnessCall {
            name: "point_argument".into(),
            arguments: vec![parameter("point")],
        },
        Type::Field,
    );
    let plan = checked(&contract, &circuit).unwrap();
    super::super::render(&plan).unwrap();
    contract.witnesses[0].parameters.clear();
    contract.witnesses[0].result = Type::JubjubPoint;
    let circuit = candidate(
        Expr::WitnessCall {
            name: "point_argument".into(),
            arguments: vec![],
        },
        Type::JubjubPoint,
    );
    let plan = checked(&contract, &circuit).unwrap();
    super::super::render(&plan).unwrap();
}

#[test]
fn point_cells_follow_declarations_and_identity_coercions_keep_the_same_value() {
    let expression = Expr::Coerce {
        value: Box::new(read()),
        ty: Type::JubjubPoint,
    };
    let circuit = candidate(expression, Type::JubjubPoint);
    let mut contract = contract();
    let checked = checked(&contract, &circuit).unwrap();
    assert_eq!(checked.plan().body.bindings.len(), 1);
    assert_eq!(
        checked.plan().body.result,
        checked.plan().body.bindings[0].value
    );
    contract.ledger_fields[0].declaration = LedgerFieldKind::Cell { ty: Type::Field };
    assert!(matches!(
        super::super::check(
            &circuit,
            &contract
                .ledger_fields
                .iter()
                .map(|f| (f.id.as_str(), f))
                .collect(),
            &HashMap::new()
        ),
        Err(PlanError::Unsupported(_))
    ));
}
