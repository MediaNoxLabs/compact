// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Wide value laws remain separate from operation-specific arithmetic policies.
use super::*;

#[path = "wide_values/field_cast_source.rs"]
mod field_cast_source;

const TWO128: &str = "340282366920938463463374607431768211456";
const MAX248: &str = "452312848583266388373324160190187140051835877600158453279131187530910662655";
const TWO248: &str = "452312848583266388373324160190187140051835877600158453279131187530910662656";

fn unsigned(value: &str, max: &str) -> Expr {
    Expr::UnsignedLiteral {
        value: value.into(),
        max: max.into(),
    }
}
fn reference(name: &str) -> Expr {
    Expr::Parameter { name: name.into() }
}
fn as_field(value: Expr) -> Expr {
    Expr::Coerce {
        value: Box::new(value),
        ty: Type::Field,
    }
}
fn circuit(value: Expr) -> StatefulCircuit {
    StatefulCircuit {
        source: None,
        name: "wide".into(),
        internal: false,
        parameters: vec![],
        actions: vec![],
        result: Type::Field,
        return_value: StateReturn::Effectful {
            body: ReturnPlan::Value { value },
        },
    }
}
fn check_expression(value: Expr) -> Result<CheckedPlan, PlanError> {
    check(&circuit(value), &HashMap::new(), &HashMap::new())
}
fn expect_unsupported(value: Expr) {
    assert!(matches!(
        check_expression(value),
        Err(PlanError::Unsupported(_))
    ));
}
fn direct_field_cast(value: Expr) -> Expr {
    Expr::FieldCast {
        value: Box::new(value),
    }
}
fn rendered_steps(checked: &CheckedPlan) -> String {
    let steps = render(checked).unwrap().steps;
    quote::quote!(#(#steps)*).to_string()
}
#[test]
fn direct_field_cast_matches_coercion_for_small_and_canonical_wide_sources() {
    for (max, value) in [
        ("0", "0"),
        ("255", "1"),
        (
            "340282366920938463463374607431768211455",
            "340282366920938463463374607431768211455",
        ),
        (TWO128, "0"),
        (TWO128, "1"),
        (TWO128, "340282366920938463463374607431768211455"),
        (TWO128, TWO128),
        ("680564733841876926926749214863536422910", TWO128),
        (MAX248, TWO128),
        (MAX248, MAX248),
    ] {
        let coerced = check_expression(as_field(unsigned(value, max))).unwrap();
        let direct = check_expression(direct_field_cast(unsigned(value, max)));
        assert_eq!(rendered_steps(&direct.unwrap()), rendered_steps(&coerced));
    }
}
#[test]
fn direct_field_cast_rejects_non_unsigned_values_and_malformed_bounds() {
    for value in [
        Expr::Boolean { value: true },
        Expr::FieldLiteral { value: "1".into() },
        Expr::BytesLiteral { bytes: vec![1; 32] },
    ] {
        let Err(PlanError::Unsupported(gap)) = check_expression(direct_field_cast(value)) else {
            panic!("direct FieldCast requires an unsigned source");
        };
        assert_eq!(gap.path, "return_value.body.value");
        assert_eq!(gap.detail, "Field cast requires unsigned input");
    }
    for (value, max) in [
        ("340282366920938463463374607431768211457", TWO128),
        (TWO248, MAX248),
        ("1", TWO248),
        ("01", TWO128),
        ("-1", TWO128),
        ("1", "0340282366920938463463374607431768211456"),
        ("1", "invalid"),
    ] {
        expect_unsupported(direct_field_cast(unsigned(value, max)));
    }
}
#[test]
fn canonical_wide_literals_cover_small_values_two128_and_max248() {
    for (max, value) in [
        (TWO128, "0"),
        (TWO128, "1"),
        (TWO128, "340282366920938463463374607431768211455"),
        (TWO128, TWO128),
        (MAX248, TWO128),
        (MAX248, MAX248),
    ] {
        let checked = check_expression(as_field(unsigned(value, max))).unwrap();
        let first = &checked.plan().body.bindings[0];
        let Operation::Literal(Literal::WideUnsigned(bytes)) = &first.operation else {
            panic!("expected canonical wide literal for {value}");
        };
        assert_eq!(*bytes, crate::field_literal_bytes(value).unwrap());
        let rendered = render(&checked).unwrap();
        let steps = rendered.steps;
        let source = quote::quote!(#(#steps)*).to_string();
        assert!(source.contains("WideUint"));
        assert!(source.contains("as_field"));
        let _: syn::Block = syn::parse_quote!({ #(#steps)* });
    }
    let small = check_expression(as_field(unsigned(
        "340282366920938463463374607431768211455",
        "340282366920938463463374607431768211455",
    )))
    .unwrap();
    assert!(matches!(
        small.plan().body.bindings[0].operation,
        Operation::Literal(Literal::Unsigned(u128::MAX))
    ));
}
#[test]
fn wide_references_equality_and_shadowing_preserve_scoped_values() {
    let ty = validate::uint(TWO128);
    let mut candidate = circuit(direct_field_cast(reference("value")));
    candidate.return_value = StateReturn::Effectful {
        body: ReturnPlan::Let {
            bindings: vec![LocalBinding {
                name: "value".into(),
                ty: ty.clone(),
                value: unsigned(TWO128, TWO128),
            }],
            result: Box::new(ReturnPlan::Conditional {
                condition: Expr::Equal {
                    left: Box::new(reference("value")),
                    right: Box::new(unsigned(TWO128, TWO128)),
                },
                then: Box::new(ReturnPlan::Let {
                    bindings: vec![LocalBinding {
                        name: "value".into(),
                        ty,
                        value: unsigned("1", TWO128),
                    }],
                    result: Box::new(ReturnPlan::Value {
                        value: direct_field_cast(reference("value")),
                    }),
                }),
                otherwise: Box::new(ReturnPlan::Value {
                    value: direct_field_cast(reference("value")),
                }),
            }),
        },
    };
    let checked = check(&candidate, &HashMap::new(), &HashMap::new()).unwrap();
    let steps = &checked.plan().body.bindings;
    assert!(
        matches!(&steps[2].operation, Operation::Equal(a, b) if a.id == 0 && b.id == 1 && a.ty == validate::uint(TWO128))
    );
    let Operation::If {
        then, otherwise, ..
    } = &steps[3].operation
    else {
        panic!("branch")
    };
    assert!(
        matches!(&then.bindings[1].operation, Operation::Convert { value, .. } if value.id == then.bindings[0].value.id)
    );
    assert!(
        matches!(&otherwise.bindings[0].operation, Operation::Convert { value, .. } if value.id == 0)
    );
    render(&checked).unwrap();
}
#[test]
fn malformed_literals_and_bounds_are_refused_before_rendering() {
    for (value, max) in [
        ("340282366920938463463374607431768211457", TWO128),
        (TWO248, MAX248),
        ("1", TWO248),
        ("01", TWO128),
        ("-1", TWO128),
        ("1", "0340282366920938463463374607431768211456"),
        ("1", "invalid"),
    ] {
        expect_unsupported(as_field(unsigned(value, max)));
    }
}
fn validator_candidate() -> Plan {
    let wide = Value {
        id: 0,
        ty: validate::uint(TWO128),
    };
    Plan {
        parameters: vec![],
        result_type: Type::Field,
        body: Block {
            bindings: vec![
                Binding {
                    value: wide.clone(),
                    operation: Operation::Literal(Literal::WideUnsigned(
                        crate::field_literal_bytes(TWO128).unwrap(),
                    )),
                    path: "literal".into(),
                },
                Binding {
                    value: field(1),
                    operation: Operation::Convert {
                        value: wide,
                        kind: Conversion::Coerce,
                    },
                    path: "coerce".into(),
                },
            ],
            result: field(1),
        },
    }
}
#[test]
fn independent_validator_rejects_forged_wide_bytes_bounds_and_references() {
    assert!(validate::check(validator_candidate(), declarations::Declarations::default()).is_ok());
    for mode in 0..5 {
        let mut candidate = validator_candidate();
        match mode {
            0 => {
                let Operation::Literal(Literal::WideUnsigned(bytes)) =
                    &mut candidate.body.bindings[0].operation
                else {
                    unreachable!()
                };
                bytes[0] = 1; // 2^128 + 1 exceeds the bound.
            }
            1 => {
                let Operation::Literal(Literal::WideUnsigned(bytes)) =
                    &mut candidate.body.bindings[0].operation
                else {
                    unreachable!()
                };
                bytes[31] = 1;
            }
            2 => candidate.body.bindings[0].value.ty = validate::uint(TWO248),
            3 => {
                let Operation::Convert { value, .. } = &mut candidate.body.bindings[1].operation
                else {
                    unreachable!()
                };
                value.id = 99;
            }
            4 => {
                let Operation::Convert { value, .. } = &mut candidate.body.bindings[1].operation
                else {
                    unreachable!()
                };
                value.ty = validate::uint(MAX248);
            }
            _ => unreachable!(),
        }
        let gap = validate::check(candidate, declarations::Declarations::default()).unwrap_err();
        assert_eq!(gap.path, if mode < 3 { "literal" } else { "coerce" });
    }
}
#[test]
fn wide_widening_and_joins_preserve_arithmetic_and_implicit_narrowing_limits() {
    let literal = || unsigned(TWO128, TWO128);
    render(&check_expression(direct_field_cast(literal())).unwrap()).unwrap();
    // An explicit narrowing cast is now recorded and keeps the runtime range
    // check; this out-of-range value fails when executed, not while lowering.
    let cast = check_expression(as_field(Expr::UnsignedCast {
        value: Box::new(literal()),
        max: "65535".into(),
    }))
    .unwrap();
    render(&cast).unwrap();
    let sum = check_expression(as_field(Expr::UnsignedAdd {
        left: Box::new(literal()),
        right: Box::new(literal()),
        max: MAX248.into(),
    }))
    .unwrap();
    render(&sum).unwrap();
    expect_unsupported(as_field(Expr::Coerce {
        value: Box::new(literal()),
        ty: validate::uint("65535"),
    }));
    // Owned-value composition now admits native widening and local joins.
    // Neither operation grants implicit narrowing; addition has its own carrier law.
    let widened = check_expression(as_field(Expr::Coerce {
        value: Box::new(unsigned("1", "65535")),
        ty: validate::uint(TWO128),
    }))
    .unwrap();
    render(&widened).unwrap();
    for selected in [false, true] {
        let joined = check_expression(as_field(Expr::If {
            condition: Box::new(Expr::Boolean { value: selected }),
            then: Box::new(literal()),
            otherwise: Box::new(unsigned("0", TWO128)),
        }))
        .unwrap();
        render(&joined).unwrap();
    }
}
#[test]
fn wide_public_and_witness_values_preserve_cell_boundaries() {
    let mut candidate = circuit(as_field(unsigned(TWO128, TWO128)));
    candidate.parameters.push(Parameter {
        name: "wide".into(),
        ty: validate::uint(TWO128),
    });
    render(&check(&candidate, &HashMap::new(), &HashMap::new()).unwrap()).unwrap();
    candidate.parameters.clear();
    candidate.result = validate::uint(TWO128);
    candidate.return_value = StateReturn::Effectful {
        body: ReturnPlan::Value {
            value: unsigned(TWO128, TWO128),
        },
    };
    render(&check(&candidate, &HashMap::new(), &HashMap::new()).unwrap()).unwrap();

    let slot = LedgerField {
        id: "wide".into(),
        index: 0,
        path: vec![0],
        source: None,
        declaration: LedgerFieldKind::Cell {
            ty: validate::uint(TWO128),
        },
    };
    let fields = HashMap::from([("wide", &slot)]);
    candidate = circuit(as_field(Expr::CellRead {
        field: "wide".into(),
        index: 0,
    }));
    assert!(matches!(
        check(&candidate, &fields, &HashMap::new()),
        Err(PlanError::Unsupported(_))
    ));
    candidate = circuit(as_field(unsigned(TWO128, TWO128)));
    candidate.return_value = StateReturn::Effectful {
        body: ReturnPlan::Sequence {
            actions: vec![StateAction::CellWrite {
                field: "wide".into(),
                index: 0,
                value: unsigned(TWO128, TWO128),
            }],
            result: Box::new(ReturnPlan::Value {
                value: Expr::FieldLiteral { value: "0".into() },
            }),
        },
    };
    assert!(matches!(
        check(&candidate, &fields, &HashMap::new()),
        Err(PlanError::Unsupported(_))
    ));

    let mut witness = WitnessDeclaration {
        source: None,
        name: "mark".into(),
        parameters: vec![Parameter {
            name: "wide".into(),
            ty: validate::uint(TWO128),
        }],
        result: Type::Field,
    };
    candidate = circuit(Expr::WitnessCall {
        name: "mark".into(),
        arguments: vec![unsigned(TWO128, TWO128)],
    });
    // Callback arguments have a separate typed-value policy; the cell refusals
    // above remain intact when public wide values become encodable.
    let checked = check(
        &candidate,
        &HashMap::new(),
        &HashMap::from([("mark", &witness)]),
    )
    .unwrap();
    render(&checked).unwrap();
    witness.parameters.clear();
    witness.result = validate::uint(TWO128);
    candidate = circuit(as_field(Expr::WitnessCall {
        name: "mark".into(),
        arguments: vec![],
    }));
    // Wide results independently enter the qualified witness-result domain.
    // Neither public nor callback eligibility admits wide ledger cells.
    let checked = check(
        &candidate,
        &HashMap::new(),
        &HashMap::from([("mark", &witness)]),
    )
    .unwrap();
    render(&checked).unwrap();
}
