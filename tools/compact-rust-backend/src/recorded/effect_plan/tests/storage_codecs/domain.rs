// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;

#[test]
fn storage_codecs_compose_all_three_roles_for_recursive_cell_values() {
    for ty in [
        vector(Type::Field, 2),
        tuple(8),
        vector(tuple(8), 2),
        validate::uint("680564733841876926926749214863536422911"),
        validate::uint(
            "452312848583266388373324160190187140051835877600158453279131187530910662655",
        ),
        product(vector(Type::OpaqueBytes, 2)),
        product(Type::Tuple {
            elements: vec![Type::OpaqueString, Type::Unit],
        }),
    ] {
        let c = contract(ty.clone());
        let plan = checked(&c).unwrap();
        assert_eq!(plan.plan().body.result.ty, ty);
        let operations: Vec<_> = plan
            .plan()
            .body
            .bindings
            .iter()
            .filter_map(|b| match b.operation {
                Operation::CellWrite { .. } => Some("cell.write"),
                Operation::SetInsert { .. } => Some("set.insert"),
                Operation::SetMember { .. } => Some("set.member"),
                Operation::MapInsert { .. } => Some("map.insert"),
                Operation::MapLookup { .. } => Some("map.lookup"),
                Operation::MapRemove { .. } => Some("map.remove"),
                Operation::SetRemove { .. } => Some("set.remove"),
                Operation::CellRead { .. } => Some("cell.read"),
                _ => None,
            })
            .collect();
        assert_eq!(
            operations,
            [
                "cell.write",
                "set.insert",
                "set.member",
                "map.insert",
                "map.lookup",
                "map.remove",
                "set.remove",
                "cell.read"
            ]
        );
        render(&plan).unwrap();
        let output = crate::render_with_capabilities(&c).unwrap();
        assert!(
            output.capabilities.circuits[0].recorded
                && output.capabilities.circuits[0].observed_call
        );
    }
}

#[test]
fn storage_codecs_empty_products_do_not_erase_member_authority() {
    for ty in [
        tuple(0),
        vector(Type::Field, 0),
        product(Type::Unit),
        Type::Struct {
            name: "Empty".into(),
            fields: vec![],
        },
    ] {
        let c = contract(ty);
        let plan = checked(&c).unwrap();
        render(&plan).unwrap();
        crate::render_with_capabilities(&c).unwrap();
    }
    for ty in [
        Type::Unit,
        tuple(9),
        product(tuple(9)),
        vector(tuple(9), 0),
        Type::LedgerMap {
            key: Box::new(Type::Field),
            value: Box::new(Type::Field),
        },
    ] {
        assert!(!validate::cell_type(&ty));
        assert!(!validate::set_element_type(&ty));
        assert!(!validate::map_key_type(&ty));
        assert!(!validate::maps::scalar_value_type(&ty));
        assert!(checked(&contract(ty)).is_err());
    }
}

#[test]
fn storage_codecs_preserve_map_values_and_the_exact_prior_merkle_relation() {
    // This independently spells the former relation as an immutable policy law.
    fn old_merkle(ty: &Type) -> bool {
        fn member(ty: &Type) -> bool {
            match ty {
                Type::OpaqueString | Type::OpaqueBytes => true,
                Type::Struct { fields, .. } => {
                    !fields.is_empty() && fields.iter().all(|f| member(&f.ty))
                }
                _ => validate::surface_type(ty),
            }
        }
        *ty != Type::Unit && member(ty)
    }
    let leaves = [
        Type::Unit,
        Type::Boolean,
        Type::Field,
        Type::OpaqueString,
        Type::OpaqueBytes,
        Type::JubjubPoint,
        Type::Bytes { length: 32 },
        tuple(0),
        tuple(8),
        tuple(9),
        vector(Type::Field, 2),
        validate::uint("680564733841876926926749214863536422911"),
        Type::Struct {
            name: "Empty".into(),
            fields: vec![],
        },
    ];
    for leaf in leaves {
        for ty in [leaf.clone(), product(leaf.clone()), vector(leaf, 0)] {
            assert_eq!(validate::merkle_leaf_type(&ty), old_merkle(&ty), "{ty:?}");
            let old_map = ty != Type::Unit && validate::value_type(&ty) && named_derives(&ty);
            assert_eq!(validate::maps::scalar_value_type(&ty), old_map, "{ty:?}");
        }
    }
    fn named_derives(ty: &Type) -> bool {
        match ty {
            Type::Tuple { elements } => elements.len() <= 8 && elements.iter().all(named_derives),
            Type::Vector { element, .. } => named_derives(element),
            Type::Struct { fields, .. } => fields.iter().all(|f| named_derives(&f.ty)),
            _ => true,
        }
    }
}
