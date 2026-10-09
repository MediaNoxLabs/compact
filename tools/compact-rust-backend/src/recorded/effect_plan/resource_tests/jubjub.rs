// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Point-domain depth and width at the existing public preflight boundary.
use super::*;

fn parameter(name: &str) -> Expr {
    Expr::Parameter { name: name.into() }
}

fn point_contract(value: Expr, bindings: Vec<LocalBinding>) -> Contract {
    let mut result = contract(read());
    result.ledger_fields[0].declaration = LedgerFieldKind::Cell {
        ty: Type::JubjubPoint,
    };
    let circuit = &mut result.stateful_circuits[0];
    circuit.parameters = vec![Parameter {
        name: "point".into(),
        ty: Type::JubjubPoint,
    }];
    circuit.result = Type::JubjubPoint;
    circuit.actions = vec![StateAction::Let {
        bindings,
        action: Box::new(StateAction::CellWrite {
            field: "state".into(),
            index: 0,
            value,
        }),
    }];
    circuit.return_value = StateReturn::Expression {
        value: parameter("point"),
    };
    result
}

fn deep_point(operations: usize) -> Contract {
    let mut value = parameter("point");
    for _ in 0..operations {
        value = Expr::EcAdd {
            left: Box::new(value),
            right: Box::new(parameter("point")),
        };
    }
    point_contract(value, vec![])
}

fn wide_point(width: usize) -> Contract {
    let bindings = (0..width)
        .map(|i| LocalBinding {
            name: format!("p{i}"),
            ty: Type::JubjubPoint,
            value: Expr::EcMul {
                point: Box::new(parameter("point")),
                scalar: Box::new(Expr::JubjubScalarFromNative {
                    value: Box::new(Expr::FieldLiteral { value: "2".into() }),
                }),
            },
        })
        .collect();
    point_contract(parameter("point"), bindings)
}

fn maximum_depth(make: fn(usize) -> Contract) -> usize {
    let (mut accepted, mut refused) = (0, 1);
    preflight(&make(accepted)).unwrap();
    while preflight(&make(refused)).is_ok() {
        accepted = refused;
        refused *= 2;
    }
    while refused - accepted > 1 {
        let middle = accepted + (refused - accepted) / 2;
        match preflight(&make(middle)) {
            Ok(()) => accepted = middle,
            Err(RenderError::ResourceLimit { .. }) => refused = middle,
            Err(error) => panic!("unexpected preflight error: {error}"),
        }
    }
    accepted
}

#[test]
fn point_boundaries_on_two_mib_worker() {
    isolated("jubjub::point_boundaries_on_two_mib_worker", || {
        let depth = maximum_depth(deep_point);
        let width = maximum_width(wide_point);
        eprintln!("Jubjub admitted point-operation depth={depth}, local width={width}");
        for (label, make, maximum) in [
            (
                "nested point addition",
                deep_point as fn(usize) -> Contract,
                depth,
            ),
            ("point multiplication locals", wide_point, width),
        ] {
            qualify(&make(maximum), label);
            let next = make(maximum + 1);
            for result in [
                preflight(&next),
                crate::render_with_capabilities(&next).map(|_| ()),
            ] {
                assert!(matches!(result, Err(RenderError::ResourceLimit { .. })));
            }
        }
    });
}
