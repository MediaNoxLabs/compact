// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Invalid plans cannot replace declaration, scope or value authority.
use super::*;

#[test]
fn each_map_operation_checks_identity_kind_index_and_exact_result() {
    let c = contract(Type::Field, Type::Field);
    let original = checked(&c).unwrap().plan().clone();
    assert_eq!(original.body.bindings.len(), 4);
    for position in 0..4 {
        let mut isolated = original.clone();
        isolated.body.bindings = vec![original.body.bindings[position].clone()];
        isolated.body.result = isolated.body.bindings[0].value.clone();
        isolated.result_type = isolated.body.result.ty.clone();
        validate::check(isolated.clone(), declarations(&c)).unwrap();
        for mutation in ["id", "index", "kind", "result"] {
            let mut plan = isolated.clone();
            let mut source = c.clone();
            let binding = &mut plan.body.bindings[0];
            match mutation {
                "id" => map_slot(&mut binding.operation).declaration = LedgerId(999),
                "index" => map_slot(&mut binding.operation).index = 1,
                "kind" => source.ledger_fields[0].declaration = LedgerFieldKind::Counter,
                "result" => {
                    binding.value.ty = Type::OpaqueString;
                    plan.body.result = binding.value.clone();
                    plan.result_type = Type::OpaqueString;
                }
                _ => unreachable!(),
            }
            assert!(
                validate::check(plan, declarations(&source)).is_err(),
                "{position}/{mutation}"
            );
        }
    }
}
#[test]
fn maps_reject_missing_self_and_in_scope_wrong_typed_operands() {
    let c = contract(Type::Field, Type::Field);
    let original = checked(&c).unwrap().plan().clone();
    for position in 0..4 {
        for mode in ["missing", "self", "annotation", "wrong_value"] {
            let mut plan = original.clone();
            let self_id = plan.body.bindings[position].value.id;
            let bad = plan.parameters[2].clone();
            let key = map_key(&mut plan.body.bindings[position].operation);
            match mode {
                "missing" => key.id = 999,
                "self" => key.id = self_id,
                "annotation" => key.ty = Type::Boolean,
                "wrong_value" => *key = bad,
                _ => unreachable!(),
            };
            assert!(
                validate::check(plan, declarations(&c)).is_err(),
                "{position}/{mode}"
            );
        }
    }
    let mut plan = original;
    let bad = plan.parameters[2].clone();
    let Operation::MapInsert { value, .. } = &mut plan.body.bindings[0].operation else {
        unreachable!()
    };
    *value = bad;
    assert!(validate::check(plan, declarations(&c)).is_err());
}
#[test]
fn scalar_codec_limits_remain_distinct_from_map_membership() {
    for value in [
        Type::Unit,
        validate::uint("340282366920938463463374607431768211456"),
        Type::Tuple { elements: vec![] },
        Type::Struct {
            name: "Empty".into(),
            fields: vec![],
        },
    ] {
        let c = contract(Type::Field, value);
        assert!(matches!(checked(&c), Err(PlanError::Unsupported(_))));
    }
    for key in [
        Type::Unit,
        validate::uint("340282366920938463463374607431768211456"),
    ] {
        let mut c = contract(key, Type::Field);
        c.stateful_circuits[0].actions.clear();
        assert!(matches!(checked(&c), Err(PlanError::Unsupported(_))));
    }
}
#[test]
fn map_slot_selection_preserves_distinct_identity_and_chunked_paths() {
    let mut c = contract(Type::Field, Type::Field);
    // Seventeen declarations use the native [2, 15] chunk layout. The two
    // Maps deliberately share their first index but have distinct identities.
    c.ledger_fields[0].path = vec![0, 0];
    let mut second = c.ledger_fields[0].clone();
    second.id = "another".into();
    second.path = vec![0, 1];
    c.ledger_fields.push(second);
    for index in 0..15 {
        c.ledger_fields.push(LedgerField {
            id: format!("padding{index}"),
            index: 1,
            path: vec![1, index],
            source: None,
            declaration: LedgerFieldKind::Counter,
        });
    }
    c.stateful_circuits[0].actions.clear();
    c.stateful_circuits[0].return_value = StateReturn::Expression {
        value: Expr::MapMember {
            field: "another".into(),
            index: 0,
            key: Box::new(parameter("key")),
        },
    };
    let checked = checked(&c).unwrap();
    assert!(rendered(&checked).contains("another . record_member"));
    let operation = &checked.plan().body.bindings[0].operation;
    let Operation::MapMember { slot, .. } = operation else {
        unreachable!()
    };
    assert_eq!(
        checked.declarations().field(slot.declaration).unwrap().path,
        vec![0, 1]
    );
    let full = crate::render_with_capabilities(&c).unwrap();
    let syntax: syn::File = syn::parse_str(&full.source).unwrap();
    let slot = syntax
        .items
        .iter()
        .find_map(|item| match item {
            syn::Item::Mod(module) if module.ident == "ledger_slots" => module
                .content
                .as_ref()
                .unwrap()
                .1
                .iter()
                .find_map(|item| match item {
                    syn::Item::Const(slot) if slot.ident == "another" => Some(slot),
                    _ => None,
                }),
            _ => None,
        })
        .unwrap();
    let syn::Expr::Call(call) = &*slot.expr else {
        panic!("slot constructor")
    };
    let syn::Expr::Reference(reference) = &call.args[0] else {
        panic!("slot path reference")
    };
    let syn::Expr::Array(array) = &*reference.expr else {
        panic!("slot path array")
    };
    let path: Vec<u8> = array
        .elems
        .iter()
        .map(|value| {
            let syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Int(value),
                ..
            }) = value
            else {
                panic!("path byte")
            };
            value.base10_parse().unwrap()
        })
        .collect();
    assert_eq!(path, vec![0, 1]);
}

#[test]
fn insertion_values_reject_forged_references_and_branch_local_escape() {
    let c = contract(Type::Field, Type::Field);
    let original = checked(&c).unwrap().plan().clone();
    for mode in ["missing", "self", "annotation", "wrong_value"] {
        let mut plan = original.clone();
        let self_id = plan.body.bindings[0].value.id;
        let bad = plan.parameters[2].clone();
        let Operation::MapInsert { value, .. } = &mut plan.body.bindings[0].operation else {
            unreachable!()
        };
        match mode {
            "missing" => value.id = 999,
            "self" => value.id = self_id,
            "annotation" => value.ty = Type::Boolean,
            "wrong_value" => *value = bad,
            _ => unreachable!(),
        };
        assert!(validate::check(plan, declarations(&c)).is_err(), "{mode}");
    }
    let mut source = c.clone();
    source.stateful_circuits[0].actions.insert(
        0,
        StateAction::If {
            condition: Expr::Boolean { value: true },
            then: Box::new(StateAction::Expression {
                value: Expr::FieldLiteral { value: "9".into() },
            }),
            otherwise: Box::new(StateAction::Sequence { actions: vec![] }),
        },
    );
    let mut plan = checked(&source).unwrap().plan().clone();
    let hidden = plan
        .body
        .bindings
        .iter()
        .find_map(|b| match &b.operation {
            Operation::If { then, .. } => then
                .bindings
                .iter()
                .find(|b| b.value.ty == Type::Field)
                .map(|b| b.value.clone()),
            _ => None,
        })
        .unwrap();
    for binding in &mut plan.body.bindings {
        if let Operation::MapInsert { value, .. } = &mut binding.operation {
            *value = hidden.clone();
        }
    }
    assert!(validate::check(plan, declarations(&source)).is_err());
}
#[test]
fn maps_keep_canonical_named_definitions_authoritative() {
    for named in [
        Type::Struct {
            name: "Entry".into(),
            fields: vec![StructField {
                name: "field".into(),
                ty: Type::Field,
            }],
        },
        Type::Enum {
            name: "Choice".into(),
            variants: vec!["First".into(), "Last".into()],
        },
    ] {
        for named_key in [true, false] {
            let c = if named_key {
                contract(named.clone(), Type::Field)
            } else {
                contract(Type::Field, named.clone())
            };
            let mut plan = checked(&c).unwrap().plan().clone();
            let parameter_index = if named_key { 0 } else { 1 };
            let mut forged = plan.parameters[parameter_index].clone();
            match &mut forged.ty {
                Type::Struct { fields, .. } => fields[0].name = "replacement".into(),
                Type::Enum { variants, .. } => variants[0] = "Replacement".into(),
                _ => unreachable!(),
            }
            // Even a consistent input annotation cannot change a named definition.
            plan.parameters[parameter_index] = forged.clone();
            for binding in &mut plan.body.bindings {
                if named_key {
                    *map_key(&mut binding.operation) = forged.clone();
                } else if let Operation::MapInsert { value, .. } = &mut binding.operation {
                    *value = forged.clone();
                }
            }
            assert!(validate::check(plan, declarations(&c)).is_err());
        }
    }
}
#[test]
fn nested_membership_uses_native_slot_availability_without_scalar_permission() {
    for nested_key in [
        Type::Struct {
            name: "not a Rust identifier".into(),
            fields: vec![],
        },
        Type::LedgerMap {
            key: Box::new(Type::Field),
            value: Box::new(Type::Field),
        },
    ] {
        let mut c = contract(
            Type::Field,
            Type::LedgerMap {
                key: Box::new(nested_key),
                value: Box::new(Type::Field),
            },
        );
        c.stateful_circuits[0].parameters.remove(1);
        c.stateful_circuits[0].actions.clear();
        assert!(matches!(checked(&c), Err(PlanError::Unsupported(_))));
    }
}
