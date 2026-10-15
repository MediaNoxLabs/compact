// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Public ownership does not grant witness authority or trust lowered annotations.
use super::*;

fn structure(name: &str, ty: Type) -> Type {
    Type::Struct {
        name: name.into(),
        fields: vec![
            StructField {
                name: "payload".into(),
                ty,
            },
            StructField {
                name: "fixed".into(),
                ty: Type::Field,
            },
        ],
    }
}
fn root(ty: Type) -> StatefulCircuit {
    StatefulCircuit {
        source: None,
        name: "echo".into(),
        internal: false,
        parameters: vec![Parameter {
            name: "value".into(),
            ty: ty.clone(),
        }],
        actions: vec![],
        result: ty,
        return_value: StateReturn::Expression {
            value: Expr::Parameter {
                name: "value".into(),
            },
        },
    }
}
fn environment(root: &StatefulCircuit) -> type_environment::TypeEnvironment {
    type_environment::TypeEnvironment::from_sources(
        root,
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
    )
    .unwrap()
}
fn authority(root: &StatefulCircuit) -> declarations::Declarations {
    declarations::Declarations::new(&HashMap::new(), &HashMap::new())
        .with_type_environment(environment(root))
        .with_root(root)
}
fn checked(root: &StatefulCircuit) -> CheckedPlan {
    check(root, &HashMap::new(), &HashMap::new()).unwrap()
}
fn rewritten(plan: &Plan, ty: Type) -> Plan {
    let mut plan = plan.clone();
    plan.parameters[0].ty = ty.clone();
    plan.body.result.ty = ty.clone();
    plan.result_type = ty;
    plan
}

#[test]
fn public_opaque_types_are_separate_from_witness_and_local_domains() {
    for ty in [
        Type::OpaqueString,
        Type::OpaqueBytes,
        structure("Message", Type::OpaqueString),
        structure("Envelope", structure("Payload", Type::OpaqueBytes)),
        Type::Unit,
        Type::Field,
        Type::Boolean,
        Type::JubjubPoint,
        Type::Bytes { length: 32 },
        validate::uint("255"),
        Type::Enum {
            name: "Choice".into(),
            variants: vec!["One".into()],
        },
    ] {
        assert!(validate::public_interface_type(&ty), "{ty:?}");
        let source = root(ty);
        let checked = checked(&source);
        validate::check(checked.plan().clone(), authority(&source)).unwrap();
        render(&checked).unwrap();
    }
    assert!(!validate::surface_type(&Type::OpaqueBytes));
    assert!(!validate::surface_type(&Type::OpaqueString));
    assert!(validate::witness_result_type(&Type::OpaqueBytes));
    assert!(validate::witness_result_type(&Type::OpaqueString));
    assert!(validate::witness_result_type(&structure(
        "Message",
        Type::OpaqueBytes
    )));
    assert!(!validate::witness_result_type(&Type::Tuple {
        elements: vec![Type::Field; 12]
    }));
    assert!(validate::cell_type(&Type::OpaqueString));
    assert!(validate::value_type(&Type::OpaqueString));
}

#[test]
fn public_policy_keeps_unencodable_and_invalid_type_boundaries() {
    for ty in [
        Type::Tuple {
            elements: vec![Type::OpaqueBytes; 12],
        },
        Type::Vector {
            element: Box::new(Type::Tuple {
                elements: vec![Type::Field; 12],
            }),
            length: 0,
        },
        validate::uint(
            "452312848583266388373324160190187140051835877600158453279131187530910662656",
        ),
        Type::LedgerMap {
            key: Box::new(Type::Field),
            value: Box::new(Type::OpaqueBytes),
        },
        structure(
            "NamedTupleTooWide",
            Type::Tuple {
                elements: vec![Type::OpaqueBytes; 9],
            },
        ),
    ] {
        assert!(!validate::public_interface_type(&ty), "{ty:?}");
        assert!(matches!(
            check(&root(ty), &HashMap::new(), &HashMap::new()),
            Err(PlanError::Unsupported(_))
        ));
    }
}

#[test]
fn coordinated_forged_root_types_cannot_replace_canonical_source_authority() {
    let original = root(structure(
        "Envelope",
        structure("Payload", Type::OpaqueBytes),
    ));
    let original_plan = checked(&original).plan().clone();
    let mut variants = Vec::new();
    for change in 0..5 {
        let mut ty = original.result.clone();
        let Type::Struct { name, fields } = &mut ty else {
            unreachable!()
        };
        match change {
            0 => *name = "UnknownEnvelope".into(),
            1 => fields.reverse(),
            2 => {
                fields.remove(1);
            }
            3 => fields.push(StructField {
                name: "extra".into(),
                ty: Type::OpaqueBytes,
            }),
            4 => {
                let Type::Struct { fields, .. } = &mut fields[0].ty else {
                    unreachable!()
                };
                fields[0].ty = Type::OpaqueString;
            }
            _ => unreachable!(),
        }
        variants.push(ty);
    }
    for ty in variants {
        // Signature, SSA argument and use agree with each other, but not the
        // immutable canonical environment taken from the original source.
        let forged_root = root(ty.clone());
        let forged_authority = declarations::Declarations::new(&HashMap::new(), &HashMap::new())
            .with_type_environment(environment(&original))
            .with_root(&forged_root);
        assert!(validate::check(rewritten(&original_plan, ty), forged_authority).is_err());
    }
    let gap = validate::check(
        rewritten(&original_plan, Type::OpaqueString),
        authority(&original),
    )
    .unwrap_err();
    assert_eq!(gap.detail, "root plan differs from declared signature");
}

#[test]
fn immutable_root_arity_and_opaque_ssa_scope_are_independently_checked() {
    let source = root(Type::OpaqueBytes);
    let original = checked(&source).plan().clone();
    let mut missing = original.clone();
    missing.parameters.clear();
    assert_eq!(
        validate::check(missing, authority(&source))
            .unwrap_err()
            .detail,
        "root plan differs from declared signature"
    );
    let mut escaped = original.clone();
    escaped.body.result.id = 999;
    assert!(
        validate::check(escaped, authority(&source))
            .unwrap_err()
            .detail
            .contains("out of scope")
    );
    let mut duplicated_source = source.clone();
    duplicated_source.parameters.push(Parameter {
        name: "other".into(),
        ty: Type::OpaqueBytes,
    });
    let mut duplicated = checked(&duplicated_source).plan().clone();
    duplicated.parameters[1].id = duplicated.parameters[0].id;
    assert_eq!(
        validate::check(duplicated, authority(&duplicated_source))
            .unwrap_err()
            .detail,
        "unsupported or duplicate plan parameter"
    );
    let distinct_plan = checked(&duplicated_source).plan().clone();
    duplicated_source.parameters[1].name = "value".into();
    assert_eq!(
        validate::check(distinct_plan, authority(&duplicated_source))
            .unwrap_err()
            .detail,
        "duplicate root formal name"
    );
}

#[test]
fn opaque_root_products_still_obey_static_depth_and_layout_budgets() {
    let mut deep = Type::OpaqueBytes;
    for level in 0..64 {
        deep = structure(&format!("Level{level}"), deep);
    }
    let deep_root = root(deep.clone());
    let Err(PlanError::Unsupported(source_gap)) =
        check(&deep_root, &HashMap::new(), &HashMap::new())
    else {
        panic!("deep source must refuse before recursive lowering")
    };
    assert!(source_gap.detail.contains("resource"), "{source_gap:?}");
    let original = root(Type::OpaqueBytes);
    let gap = validate::check(
        rewritten(checked(&original).plan(), deep),
        authority(&original),
    )
    .unwrap_err();
    assert!(gap.detail.contains("resource"), "{gap:?}");
    let limit = crate::resource_limits::recording_fixed_layout_limit();
    let wide = Type::Struct {
        name: "Many".into(),
        fields: (0..limit)
            .map(|i| StructField {
                name: format!("part{i}"),
                ty: Type::OpaqueBytes,
            })
            .collect(),
    };
    assert!(matches!(
        check(&root(wide), &HashMap::new(), &HashMap::new()),
        Err(PlanError::Unsupported(_))
    ));
    // These static checks do not claim to bound a dynamic opaque payload.
}

#[test]
fn unchanged_point_and_effectful_routes_can_use_the_wider_root_domain() {
    let mut point: Contract = serde_json::from_str(include_str!(
        "../../../../tests/jubjub-scalar-cell-schema20-ir.json"
    ))
    .unwrap();
    point
        .stateful_circuits
        .iter_mut()
        .find(|c| c.name == "apply")
        .unwrap()
        .parameters
        .push(Parameter {
            name: "unused_owned".into(),
            ty: Type::OpaqueString,
        });
    let mut effectful = root(Type::OpaqueBytes);
    effectful.return_value = StateReturn::Effectful {
        body: ReturnPlan::Value {
            value: Expr::Parameter {
                name: "value".into(),
            },
        },
    };
    let effectful = Contract {
        schema_version: 20,
        type_aliases: vec![],
        ledger_fields: vec![],
        constructor: None,
        witnesses: vec![],
        circuits: vec![],
        stateful_circuits: vec![effectful],
    };
    for (contract, name, owner) in [
        (point, "apply", "algebra.ordinary"),
        (effectful, "echo", "effectful.checked-migration"),
    ] {
        let (output, rows) = crate::recorded::admission_probe::trace::capture_json(|| {
            crate::render_with_capabilities(&contract)
        });
        let output = output.unwrap();
        let capability = output
            .capabilities
            .circuits
            .iter()
            .find(|c| c.name == name)
            .unwrap();
        assert!(capability.recorded && capability.observed_call);
        let rows = rows
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| {
                r["circuit"] == name && r["phase"] == "public_emission" && r["helper"] == false
            })
            .collect::<Vec<_>>();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["body_owner"], owner);
    }
}

#[test]
fn fresh_full_source_opaque_exports_have_actual_composite_ownership() {
    let contract: Contract = serde_json::from_str(include_str!(
        "../../../../tests/effect-plan-opaque-public-schema20-ir.json"
    ))
    .unwrap();
    let (output, rows) = crate::recorded::admission_probe::trace::capture_json(|| {
        crate::render_with_capabilities(&contract)
    });
    let output = output.unwrap();
    assert_eq!(output.capabilities.circuits.len(), 3);
    for name in ["run", "text_roundtrip", "bytes_roundtrip"] {
        let capability = output
            .capabilities
            .circuits
            .iter()
            .find(|c| c.name == name)
            .unwrap();
        assert!(capability.recorded && capability.observed_call, "{name}");
        let rows = rows
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| {
                r["circuit"] == name && r["phase"] == "public_emission" && r["helper"] == false
            })
            .collect::<Vec<_>>();
        assert_eq!(rows.len(), 1, "{name}");
        assert_eq!(rows[0]["body_owner"], "algebra.ordinary", "{name}");
    }
}
