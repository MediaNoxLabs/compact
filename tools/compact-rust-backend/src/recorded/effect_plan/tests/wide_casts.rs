// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Explicit casts share native conversion checks; arithmetic policy is independent.
use super::*;

const MAX128: &str = "340282366920938463463374607431768211455";
const TWO128: &str = "340282366920938463463374607431768211456";
const IRREGULAR129: &str = "680564733841876926926749214863536422910";
const MAX248: &str = "452312848583266388373324160190187140051835877600158453279131187530910662655";
const TWO248: &str = "452312848583266388373324160190187140051835877600158453279131187530910662656";

fn reference() -> Expr {
    Expr::Parameter {
        name: "input".into(),
    }
}
fn cast(value: Expr, max: &str) -> Expr {
    Expr::UnsignedCast {
        value: Box::new(value),
        max: max.into(),
    }
}
fn root(source: Type, value: Expr) -> StatefulCircuit {
    StatefulCircuit {
        source: None,
        internal: false,
        name: "convert".into(),
        parameters: vec![Parameter {
            name: "input".into(),
            ty: source,
        }],
        actions: vec![StateAction::Expression { value }],
        result: Type::Boolean,
        return_value: StateReturn::Expression {
            value: Expr::Boolean { value: true },
        },
    }
}
fn checked(source: &StatefulCircuit) -> CheckedPlan {
    check(source, &HashMap::new(), &HashMap::new()).unwrap()
}
fn authority(source: &StatefulCircuit) -> declarations::Declarations {
    let types = type_environment::TypeEnvironment::from_sources(
        source,
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
    )
    .unwrap();
    declarations::Declarations::default()
        .with_type_environment(types)
        .with_root(source)
}
fn rendered(plan: &CheckedPlan) -> String {
    let steps = render(plan).unwrap().steps;
    let body: syn::Block = syn::parse_quote!({ #(#steps)* });
    quote::quote!(#body).to_string()
}
fn conversion(plan: &mut Plan) -> &mut Binding {
    plan.body
        .bindings
        .iter_mut()
        .find(|b| {
            matches!(
                b.operation,
                Operation::Convert {
                    kind: Conversion::UnsignedCast,
                    ..
                }
            )
        })
        .unwrap()
}

#[test]
fn explicit_casts_cover_all_carriers_and_the_canonical_31_byte_boundary() {
    for (from, to, helper) in [
        ("65535", "255", "cast_unsigned"),
        (MAX128, TWO128, "from_le_bytes"),
        (MAX128, IRREGULAR129, "from_le_bytes"),
        ("0", MAX248, "from_le_bytes"),
        (TWO128, MAX128, "narrow_wide_uint"),
        (MAX248, "0", "narrow_wide_uint"),
        (TWO128, MAX248, "from_le_bytes"),
        (MAX248, TWO128, "from_le_bytes"),
        (MAX248, MAX248, "from_le_bytes"),
    ] {
        let source = root(validate::uint(from), cast(reference(), to));
        let plan = checked(&source);
        let mut candidate = plan.plan().clone();
        let binding = conversion(&mut candidate);
        assert_eq!(binding.value.ty, validate::uint(to));
        let Operation::Convert { value, kind } = &binding.operation else {
            unreachable!()
        };
        assert!(matches!(kind, Conversion::UnsignedCast));
        assert_eq!(value, &plan.plan().parameters[0]);
        validate::check(candidate, authority(&source)).unwrap();
        let text = rendered(&plan);
        assert!(text.contains(helper), "{from} -> {to}: {text}");
    }
}

#[test]
fn cast_lowering_refuses_nonunsigned_operands_and_noncanonical_bounds() {
    for from in [Type::Field, Type::Boolean, Type::OpaqueBytes, Type::Unit] {
        let source = root(from, cast(reference(), MAX248));
        let Err(PlanError::Unsupported(gap)) = check(&source, &HashMap::new(), &HashMap::new())
        else {
            panic!("nonunsigned cast must refuse")
        };
        assert_eq!(gap.path, "actions[0].value");
        assert_eq!(gap.detail, "unsupported unsigned cast");
    }
    for malformed in [
        "",
        "01",
        "+1",
        "-1",
        "invalid",
        TWO248,
        "0340282366920938463463374607431768211456",
    ] {
        for source in [
            root(validate::uint(TWO128), cast(reference(), malformed)),
            root(validate::uint(malformed), cast(reference(), TWO128)),
        ] {
            assert!(
                matches!(
                    check(&source, &HashMap::new(), &HashMap::new()),
                    Err(PlanError::Unsupported(_))
                ),
                "{malformed}"
            );
        }
    }
}

#[test]
fn validator_rechecks_cast_result_operand_and_ssa_against_immutable_authority() {
    let source = root(validate::uint(MAX248), cast(reference(), TWO128));
    let original = checked(&source).plan().clone();
    for mode in 0..6 {
        let mut forged = original.clone();
        let binding = conversion(&mut forged);
        let Operation::Convert { value, .. } = &mut binding.operation else {
            unreachable!()
        };
        match mode {
            0 => value.id = 999,
            1 => value.ty = validate::uint(TWO128),
            2 => binding.value.ty = Type::Field,
            3 => binding.value.ty = Type::Boolean,
            4 => binding.value.ty = Type::OpaqueBytes,
            5 => binding.value.id = value.id,
            _ => unreachable!(),
        }
        let gap = validate::check(forged, authority(&source)).unwrap_err();
        assert_eq!(gap.path, "actions[0].value", "mode {mode}: {gap:?}");
    }
    // The forged operand below is genuinely in scope with its annotated type.
    // It must fail the conversion contract rather than an accidental SSA mismatch.
    for ty in [Type::Field, Type::Boolean, Type::Unit] {
        let source = root(ty.clone(), Expr::Unit);
        let mut forged = original.clone();
        forged.parameters[0].ty = ty.clone();
        let Operation::Convert { value, .. } = &mut conversion(&mut forged).operation else {
            unreachable!()
        };
        value.ty = ty;
        let gap = validate::check(forged, authority(&source)).unwrap_err();
        assert_eq!(gap.path, "actions[0].value");
    }
}

#[test]
fn validator_refuses_malformed_cast_bounds_without_rebuilding_type_authority() {
    let source = root(validate::uint(MAX248), cast(reference(), TWO128));
    let original = checked(&source).plan().clone();
    for max in ["", "01", "+1", "-1", "invalid", TWO248] {
        let mut forged = original.clone();
        conversion(&mut forged).value.ty = validate::uint(max);
        let gap = validate::check(forged, authority(&source)).unwrap_err();
        assert_eq!(gap.path, "actions[0].value", "{max}: {gap:?}");
        let mut forged = original.clone();
        let Operation::Convert { value, .. } = &mut conversion(&mut forged).operation else {
            unreachable!()
        };
        value.ty = validate::uint(max);
        let gap = validate::check(forged, authority(&source)).unwrap_err();
        assert_eq!(gap.path, "actions[0].value", "{max}: {gap:?}");
    }
}

#[test]
fn explicit_narrowing_retains_fallible_conversion_instead_of_implicit_coercion() {
    for (from, to, helper) in [
        (MAX248, "65535", "narrow_wide_uint"),
        (MAX248, TWO128, "from_le_bytes"),
    ] {
        let source = root(validate::uint(from), cast(reference(), to));
        let plan = checked(&source);
        let text = rendered(&plan);
        assert!(text.contains(helper) && text.contains('?'), "{text}");
        let mut implicit = plan.plan().clone();
        let Operation::Convert { kind, .. } = &mut conversion(&mut implicit).operation else {
            unreachable!()
        };
        *kind = Conversion::Coerce;
        assert!(validate::check(implicit, authority(&source)).is_err());
    }
}

#[test]
fn nested_unsigned_casts_lower_the_witness_once_in_source_order() {
    let witness = WitnessDeclaration {
        source: None,
        name: "wide_input".into(),
        parameters: vec![],
        result: validate::uint(MAX248),
    };
    let value = Expr::WitnessCall {
        name: witness.name.clone(),
        arguments: vec![],
    };
    let mut source = root(Type::Unit, cast(cast(value, TWO128), "65535"));
    source.parameters.clear();
    let plan = check(
        &source,
        &HashMap::new(),
        &HashMap::from([("wide_input", &witness)]),
    )
    .unwrap();
    let bindings = &plan.plan().body.bindings;
    assert!(matches!(bindings[0].operation, Operation::Witness { .. }));
    for index in [1, 2] {
        let Operation::Convert {
            value,
            kind: Conversion::UnsignedCast,
        } = &bindings[index].operation
        else {
            panic!("cast after operand")
        };
        assert_eq!(value, &bindings[index - 1].value);
    }
    assert_eq!(
        bindings
            .iter()
            .filter(|b| matches!(b.operation, Operation::Witness { .. }))
            .count(),
        1
    );
    let text = rendered(&plan);
    assert_eq!(text.matches("try_witness_metered").count(), 1);
    assert_eq!(text.matches("narrow_wide_uint").count(), 1);
    assert!(text.find("try_witness_metered").unwrap() < text.find("from_le_bytes").unwrap());
    assert!(text.find("from_le_bytes").unwrap() < text.find("narrow_wide_uint").unwrap());
}

#[test]
fn cast_cannot_use_a_value_that_only_exists_inside_one_branch() {
    let source = root(Type::Boolean, Expr::Unit);
    let condition = Value {
        id: 0,
        ty: Type::Boolean,
    };
    let wide = |id| Value {
        id,
        ty: validate::uint(MAX248),
    };
    let branch = |id| Block {
        bindings: vec![Binding {
            value: wide(id),
            operation: Operation::Literal(Literal::WideUnsigned([0; 32])),
            path: format!("branch.{id}"),
        }],
        result: wide(id),
    };
    let output = Value {
        id: 2,
        ty: validate::uint("65535"),
    };
    let result = Value {
        id: 3,
        ty: Type::Boolean,
    };
    let candidate = Plan {
        parameters: vec![condition.clone()],
        result_type: Type::Boolean,
        body: Block {
            bindings: vec![
                Binding {
                    value: wide(1),
                    operation: Operation::If {
                        condition,
                        then: branch(10),
                        otherwise: branch(11),
                    },
                    path: "branch".into(),
                },
                Binding {
                    value: output,
                    operation: Operation::Convert {
                        value: wide(10),
                        kind: Conversion::UnsignedCast,
                    },
                    path: "escaped_cast".into(),
                },
                Binding {
                    value: result.clone(),
                    operation: Operation::Literal(Literal::Boolean(true)),
                    path: "result".into(),
                },
            ],
            result,
        },
    };
    let error = validate::check(candidate, authority(&source)).unwrap_err();
    assert_eq!(error.path, "escaped_cast");
    assert!(error.detail.contains("out of scope"));
}
