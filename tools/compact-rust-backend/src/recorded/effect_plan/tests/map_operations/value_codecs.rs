// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Map values and qualified ordinary storage reuse the same existing CellValue codecs.
use super::*;
fn vector(element: Type, length: usize) -> Type {
    Type::Vector {
        element: Box::new(element),
        length,
    }
}
fn tuple(length: usize) -> Type {
    Type::Tuple {
        elements: vec![Type::Field; length],
    }
}
fn product(ty: Type) -> Type {
    Type::Struct {
        name: "CompositeEntry".into(),
        fields: vec![StructField {
            name: "payload".into(),
            ty,
        }],
    }
}
#[test]
fn recursive_map_value_codecs_cover_empty_owned_products_vectors_and_wide_integers() {
    for value in [
        tuple(0),
        tuple(8),
        vector(Type::Field, 0),
        vector(tuple(8), 2),
        validate::uint("680564733841876926926749214863536422911"),
        validate::uint(
            "452312848583266388373324160190187140051835877600158453279131187530910662655",
        ),
        product(vector(Type::OpaqueBytes, 2)),
        product(Type::Unit),
        product(Type::Tuple {
            elements: vec![Type::Boolean, vector(Type::OpaqueString, 2)],
        }),
        Type::Struct {
            name: "EmptyEntry".into(),
            fields: vec![],
        },
    ] {
        let c = contract(Type::OpaqueString, value.clone());
        let plan = checked(&c).unwrap();
        let bindings = &plan.plan().body.bindings;
        assert!(matches!(bindings[0].operation, Operation::MapInsert { .. }));
        assert!(matches!(bindings[1].operation, Operation::MapLookup { .. }));
        assert_eq!(bindings[1].value.ty, value);
        assert!(matches!(bindings[2].operation, Operation::MapRemove { .. }));
        assert!(matches!(bindings[3].operation, Operation::MapMember { .. }));
        let source = rendered(&plan);
        assert!(source.find("record_insert").unwrap() < source.find("record_lookup").unwrap());
        assert!(source.find("record_lookup").unwrap() < source.find("record_remove").unwrap());
        crate::render_with_capabilities(&c).unwrap();
    }
}
#[test]
fn map_codec_policy_shares_qualified_storage_and_preserves_tuple_eight_boundary() {
    for value in [
        vector(Type::Field, 2),
        tuple(8),
        validate::uint("680564733841876926926749214863536422911"),
        Type::Struct {
            name: "EmptyEntry".into(),
            fields: vec![],
        },
    ] {
        assert!(validate::maps::scalar_value_type(&value));
        assert!(validate::cell_type(&value));
        assert!(validate::set_element_type(&value));
        assert!(validate::map_key_type(&value));
        let c = contract(value, Type::Field);
        assert!(
            checked(&c).is_ok(),
            "qualified CellValue now authorizes Map keys"
        );
    }
    for value in [
        Type::Unit,
        tuple(9),
        product(tuple(9)),
        vector(tuple(9), 0),
        product(Type::LedgerMap {
            key: Box::new(Type::Field),
            value: Box::new(Type::Field),
        }),
    ] {
        assert!(!validate::maps::scalar_value_type(&value));
        assert!(checked(&contract(Type::Field, value)).is_err());
    }
    // Empty tuple is a qualified product; top-level Unit keeps the established
    // admission restriction despite the shared Rust representation of both.
    assert!(validate::maps::scalar_value_type(&tuple(0)));
    assert!(!validate::maps::scalar_value_type(&Type::Unit));
}
#[test]
fn composite_map_values_keep_canonical_types_bounds_and_resource_preflight() {
    let c = contract(Type::Field, product(vector(Type::Field, 2)));
    let original = checked(&c).unwrap().plan().clone();
    let mut forged = original.clone();
    let Type::Struct { fields, .. } = &mut forged.parameters[1].ty else {
        unreachable!()
    };
    fields[0].ty = vector(Type::Boolean, 2);
    let replacement = forged.parameters[1].clone();
    let Operation::MapInsert { value, .. } = &mut forged.body.bindings[0].operation else {
        unreachable!()
    };
    *value = replacement;
    assert!(
        validate::check(forged, declarations(&c)).is_err(),
        "consistent local forgery cannot change canonical declaration"
    );
    for ty in [
        validate::uint("not a maximum"),
        validate::uint(
            "452312848583266388373324160190187140051835877600158453279131187530910662656",
        ),
        vector(
            Type::Field,
            crate::resource_limits::recording_fixed_layout_limit() + 1,
        ),
        Type::Bytes {
            length: crate::resource_limits::recording_byte_payload_limit() + 1,
        },
    ] {
        assert!(checked(&contract(Type::Field, product(ty))).is_err());
    }
}
#[test]
fn independent_validation_rechecks_expanded_map_operand_and_result_authority() {
    let c = contract(Type::Field, product(vector(Type::Field, 2)));
    let original = checked(&c).unwrap().plan().clone();
    for mutation in ["missing", "self", "annotation", "lookup_result"] {
        let mut plan = original.clone();
        let self_id = plan.body.bindings[0].value.id;
        if mutation == "lookup_result" {
            plan.body.bindings[1].value.ty = product(vector(Type::Boolean, 2));
        } else {
            let Operation::MapInsert { value, .. } = &mut plan.body.bindings[0].operation else {
                unreachable!()
            };
            match mutation {
                "missing" => value.id = 999,
                "self" => value.id = self_id,
                "annotation" => value.ty = Type::Boolean,
                _ => unreachable!(),
            }
        }
        assert!(
            validate::check(plan, declarations(&c)).is_err(),
            "{mutation}"
        );
    }
}
#[test]
fn original_guarded_map_domain_with_composite_members_now_has_checked_support() {
    use serde_json::{Value as Json, json};
    fn append(v: &mut Json, ty: &Json) {
        match v {
            Json::Object(m) => {
                if m.get("kind") == Some(&json!("struct"))
                    && m.get("name") == Some(&json!("AssetRecord"))
                {
                    m["fields"]
                        .as_array_mut()
                        .unwrap()
                        .push(json!({"name":"codec_boundary","ty":ty}));
                }
                m.values_mut().for_each(|v| append(v, ty));
            }
            Json::Array(a) => a.iter_mut().for_each(|v| append(v, ty)),
            _ => {}
        }
    }
    for ty in [
        json!({"kind":"vector","element":{"kind":"field"},"length":2}),
        json!({"kind":"tuple","elements":[{"kind":"field"},{"kind":"boolean"}]}),
        json!({"kind":"unsigned","max":"680564733841876926926749214863536422911"}),
    ] {
        let mut v: Json = serde_json::from_str(include_str!(
            "../../../../../tests/asset-registry-schema20-ir.json"
        ))
        .unwrap();
        append(&mut v, &ty);
        let c: Contract = serde_json::from_value(v).unwrap();
        assert_whole_old_and_checked(&c, "assertStoredRecordFresh");
    }
}

#[test]
fn genuine_map_value_products_source_retains_complete_old_and_checked_admission() {
    let c: Contract = serde_json::from_str(include_str!(
        "../../../../../tests/effect-plan-map-value-products-schema20-ir.json"
    ))
    .unwrap();
    assert_whole_old_and_checked(&c, "readFresh");
}

fn assert_whole_old_and_checked(c: &Contract, root_name: &str) {
    use crate::recorded::{
        admission_probe::trace, legacy::guarded_map::closed_guarded_struct_map_read_steps,
    };
    let root = c
        .stateful_circuits
        .iter()
        .find(|r| r.name == root_name)
        .unwrap();
    let ledger = c.ledger_fields.iter().map(|x| (x.id.as_str(), x)).collect();
    let pure = c.circuits.iter().map(|x| (x.name.as_str(), x)).collect();
    let parameters = root
        .parameters
        .iter()
        .enumerate()
        .map(|(i, p)| {
            (
                p.name.as_str(),
                (&p.ty, crate::ident(&format!("arg{i}")).unwrap()),
            )
        })
        .collect();
    assert!(
        closed_guarded_struct_map_read_steps(root, &parameters, &ledger, &pure)
            .unwrap()
            .is_some(),
        "whole old premise"
    );
    let plan = check_with_calls(
        root,
        &ledger,
        &c.witnesses.iter().map(|x| (x.name.as_str(), x)).collect(),
        &pure,
        &c.stateful_circuits
            .iter()
            .map(|x| (x.name.as_str(), x))
            .collect(),
    )
    .unwrap();
    let source = rendered(&plan);
    assert_eq!(source.matches("record_lookup").count(), 1);
    let (output, rows) = trace::capture_json(|| crate::render_with_capabilities(c));
    let output = output.unwrap();
    assert!(
        output
            .capabilities
            .circuits
            .iter()
            .any(|r| r.name == root.name && r.recorded && r.observed_call)
    );
    let row = rows
        .as_array()
        .unwrap()
        .iter()
        .find(|r| {
            r["circuit"] == root.name && r["phase"] == "public_emission" && r["helper"] == false
        })
        .unwrap();
    assert_eq!(row["body_owner"], "algebra.unit-composition");
    assert!(
        !row["attempts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|a| a["route"] == "guard.guarded-map-read"),
        "retirement removes the production attempt; the complete old oracle above remains"
    );
}
