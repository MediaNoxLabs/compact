// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;

#[test]
fn projection_lowering_requires_real_points_at_the_original_operand_path() {
    let lookalike = Type::Struct {
        name: "PointLookalike".into(),
        fields: vec![StructField {
            name: "x".into(),
            ty: Type::Field,
        }],
    };
    for ty in [Type::Field, Type::Boolean, lookalike] {
        for y in [false, true] {
            let mut source = root(project(reference("point"), y));
            source.parameters[0].ty = ty.clone();
            let error = check(&source, &HashMap::new(), &HashMap::new()).unwrap_err();
            let PlanError::Unsupported(gap) = error else {
                panic!("source must be Unsupported")
            };
            assert_eq!(gap.path, "return_value.value.value");
            assert!(gap.detail.contains("JubjubPoint"), "{gap:?}");
        }
    }
}
fn witness(name: &str) -> Expr {
    Expr::WitnessCall {
        name: name.into(),
        arguments: vec![],
    }
}
fn with_witnesses(body: Expr) -> CheckedPlan {
    let declarations = ["left", "right"].map(|name| WitnessDeclaration {
        source: None,
        name: name.into(),
        parameters: vec![],
        result: Type::JubjubPoint,
    });
    check(
        &root(body),
        &HashMap::new(),
        &declarations.iter().map(|w| (w.name.as_str(), w)).collect(),
    )
    .unwrap()
}
#[test]
fn distinct_point_operands_evaluate_once_in_source_order() {
    let checked = with_witnesses(Expr::Add {
        left: Box::new(project(witness("left"), false)),
        right: Box::new(project(witness("right"), true)),
    });
    let bindings = &checked.plan().body.bindings;
    assert_eq!(bindings.len(), 5);
    for (at, name) in [(0, "left"), (2, "right")] {
        let Operation::Witness { declaration, .. } = bindings[at].operation else {
            panic!("ordered witness")
        };
        assert_eq!(
            checked.declarations().witness(declaration).unwrap().name,
            name
        );
    }
    assert!(
        matches!(bindings[1].operation, Operation::JubjubPointX(ref value) if value == &bindings[0].value)
    );
    assert!(
        matches!(bindings[3].operation, Operation::JubjubPointY(ref value) if value == &bindings[2].value)
    );
    let steps = render(&checked).unwrap().steps;
    let syntax = quote::quote!(#(#steps)*).to_string();
    for call in ["left", "right", "jubjub_point_x", "jubjub_point_y"] {
        assert_eq!(syntax.matches(call).count(), 1, "{call}: {syntax}");
    }
}
#[test]
fn two_projections_of_one_local_reuse_one_witness_value() {
    let checked = with_witnesses(Expr::Let {
        bindings: vec![LocalBinding {
            name: "saved".into(),
            ty: Type::JubjubPoint,
            value: witness("left"),
        }],
        body: Box::new(Expr::Add {
            left: Box::new(project(reference("saved"), false)),
            right: Box::new(project(reference("saved"), true)),
        }),
    });
    let bindings = &checked.plan().body.bindings;
    assert_eq!(
        bindings
            .iter()
            .filter(|b| matches!(b.operation, Operation::Witness { .. }))
            .count(),
        1
    );
    let saved = &bindings[0].value;
    assert!(matches!(bindings[1].operation, Operation::JubjubPointX(ref v) if v == saved));
    assert!(matches!(bindings[2].operation, Operation::JubjubPointY(ref v) if v == saved));
}
#[test]
fn coordinate_nesting_still_passes_through_shared_source_resource_preflight() {
    let source =
        include_str!("../../../../../tests/effect-plan-jubjub-projections-schema20-ir.json");
    for depth in [1, 128] {
        let mut contract: Contract = serde_json::from_str(source).unwrap();
        let mut body = project(reference("point"), false);
        for _ in 0..depth {
            body = project(
                Expr::EcMulGenerator {
                    scalar: Box::new(body),
                },
                true,
            );
        }
        contract.stateful_circuits[0].result = Type::Field;
        contract.stateful_circuits[0].return_value = StateReturn::Expression { value: body };
        let result = crate::render_with_capabilities(&contract);
        if depth == 1 {
            result.unwrap();
        } else {
            match result {
                Err(crate::RenderError::ResourceLimit { .. }) => {}
                Err(error) => panic!("expected shared resource refusal: {error:?}"),
                Ok(_) => panic!("deep coordinate nesting bypassed resource preflight"),
            }
        }
    }
}
