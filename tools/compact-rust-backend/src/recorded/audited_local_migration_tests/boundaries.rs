// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;

#[test]
fn local_values_and_root_products_preserve_canonical_domains_and_source_exclusions() {
    // These are finite native-IR qualification controls, not additional source fixtures.
    // The measured existing priority differs for the two-Struct interface.
    // These are exact owner assertions, never a route-priority change.
    for (label, ty, owner) in [
        (
            "opaque_bytes",
            json!({"kind":"opaque_bytes"}),
            "algebra.ordinary",
        ),
        ("unit", json!({"kind":"unit"}), "algebra.ordinary"),
        (
            "tuple11",
            json!({"kind":"tuple","elements":vec![json!({"kind":"field"});11]}),
            "algebra.ordinary",
        ),
        (
            "wide129",
            json!({"kind":"unsigned","max":"680564733841876926926749214863536422911"}),
            "algebra.ordinary",
        ),
        (
            "named_product",
            json!({"kind":"struct","name":"LocalProduct","fields":[{"name":"items","ty":{"kind":"vector","length":2,"element":{"kind":"field"}}},{"name":"label","ty":{"kind":"opaque_string"}}]}),
            "algebra.ordinary",
        ),
    ] {
        let mut c = fixture();
        local(&mut c, vec![]);
        let c = change(c, |v| {
            // Change only the stateful wrapper/callee signatures and their call
            // coercions. The unrelated native attestationDigest pure circuit still
            // returns its original Vector<Field,4> literal.
            edit(&mut v["stateful_circuits"], &mut |m| {
                if m.get("kind") == Some(&json!("vector"))
                    && m.get("length") == Some(&json!(4))
                    && m.get("element") == Some(&json!({"kind":"field"}))
                {
                    *m = ty.as_object().unwrap().clone();
                }
            })
        });
        included_as(&c, VERIFY, owner);
        eprintln!("old-positive interface {label}: {owner}");
    }
    // Local Default has its own tuple12 capability; public FAB remains tuple11
    // and named-product derives retain their separate nested tuple8 boundary.
    for (ty, value) in [
        (
            Type::Tuple {
                elements: vec![Type::Field; 12],
            },
            Expr::Default {
                ty: Type::Tuple {
                    elements: vec![Type::Field; 12],
                },
            },
        ),
        (
            Type::Field,
            Expr::FieldCast {
                value: Box::new(Expr::UnsignedLiteral {
                    value: "1".into(),
                    max: "680564733841876926926749214863536422911".into(),
                }),
            },
        ),
        (
            Type::JubjubPoint,
            Expr::EcNeg {
                value: Box::new(param("pk")),
            },
        ),
    ] {
        let mut c = fixture();
        local(&mut c, vec![statement(value, ty)]);
        included(&c, ACCEPT);
    }
    // Historical exception: Rust's native hash trait permits this synthetic IR,
    // but Compact's frontend excludes opaque persistent inputs. Preserve the
    // complete old positive without falsely claiming source-language inclusion.
    let mut excluded = fixture();
    local(
        &mut excluded,
        vec![statement(
            Expr::PersistentHash {
                value: Box::new(Expr::Default {
                    ty: Type::OpaqueString,
                }),
            },
            Type::Bytes { length: 32 },
        )],
    );
    assert!(matches!(
        old(&excluded, VERIFY).unwrap(),
        Some(RecordingOutcome::Supported(_))
    ));
    crate::render_with_capabilities(&excluded).unwrap();
    let Err(effect_plan::PlanError::Unsupported(gap)) = checked(&excluded, VERIFY) else {
        panic!("opaque persistent hashing remains excluded");
    };
    assert!(gap.detail.contains("persistent") || gap.detail.contains("Persistent"));
}

#[test]
fn old_audit_refusals_are_separate_from_checked_expansion_and_real_fold_gaps() {
    let mut mapped = fixture();
    local(
        &mut mapped,
        vec![statement(
            test_support::fixed_map_input(),
            Type::Vector {
                length: 1,
                element: Box::new(Type::Field),
            },
        )],
    );
    assert!(matches!(
        old(&mapped, VERIFY).unwrap(),
        Some(RecordingOutcome::Unsupported(_))
    ));
    // A checked expansion, never counted as a complete old-positive inclusion.
    let methods = calls(&current(&mapped, VERIFY, "algebra.ordinary"));
    assert_eq!(
        methods
            .iter()
            .filter(|n| n.as_str() == "record_read")
            .count(),
        2
    );
    let mut fold = test_support::with_unsupported_fold(fixture());
    local(
        &mut fold,
        vec![statement(
            test_support::unsupported_fixed_input(),
            Type::Field,
        )],
    );
    assert!(matches!(
        old(&fold, VERIFY).unwrap(),
        Some(RecordingOutcome::Unsupported(_))
    ));
    crate::render_with_capabilities(&fold).unwrap();
    let Err(effect_plan::PlanError::Unsupported(gap)) = checked(&fold, VERIFY) else {
        panic!("direct fold must remain unsupported");
    };
    assert_eq!(gap.ir_node, "Expr::VectorFoldCall");
    assert!(
        gap.path.ends_with("actions[0].bindings[0].value"),
        "{}",
        gap.path
    );
    let mut recursive = fixture();
    let arguments = target(&recursive, LOCAL)
        .parameters
        .iter()
        .map(|p| param(&p.name))
        .collect();
    local(
        &mut recursive,
        vec![StateAction::CircuitCall {
            name: LOCAL.into(),
            arguments,
        }],
    );
    assert!(matches!(
        old(&recursive, VERIFY).unwrap(),
        Some(RecordingOutcome::Unsupported(_))
    ));
    assert!(crate::render_with_capabilities(&recursive).is_err());
    assert!(checked(&recursive, VERIFY).is_err());
}

#[test]
fn checked_read_only_sibling_internal_policy_and_optional_tail_boundaries() {
    let sibling: Contract =
        serde_json::from_str(include_str!("../../../tests/did-digest-schema20-ir.json")).unwrap();
    current(
        &sibling,
        "verifySchnorrJubjubDigestSignature",
        "algebra.ordinary",
    );
    let mut internal = fixture();
    target_mut(&mut internal, VERIFY).internal = true;
    assert!(old(&internal, VERIFY).unwrap().is_none());
    let output = crate::render_with_capabilities(&internal).unwrap();
    assert!(
        !output
            .capabilities
            .circuits
            .iter()
            .any(|r| r.name == VERIFY)
    );
    // Non-literal-one tail is outside the old renderer, but remains valid checked
    // Counter composition. It must not reintroduce a shape-dependent rejection.
    let mut different_tail = fixture();
    if let StateAction::Let { bindings, .. } =
        &mut target_mut(&mut different_tail, ACCEPT).actions[2]
    {
        bindings[0].value = Expr::UnsignedLiteral {
            value: "2".into(),
            max: "65535".into(),
        };
    }
    assert!(old(&different_tail, ACCEPT).unwrap().is_none());
    assert!(
        calls(&current(&different_tail, ACCEPT, "algebra.ordinary"))
            .iter()
            .any(|n| n == "record_increment")
    );
    // Reuse the separately executed ordinary/recursive worker gates: this slice
    // changes no depth/work/layout limit and needs no new resource multiplier.
}
