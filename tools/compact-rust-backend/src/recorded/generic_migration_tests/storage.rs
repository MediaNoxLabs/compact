// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;

#[test]
fn complete_generic_storage_actions_and_every_direct_collection_return() {
    // A01 Counter effects/R03,R04,R05: whole bodies, not append-only prefixes.
    for action in [
        StateAction::CounterIncrement {
            field: "count".into(),
            index: 0,
            amount: CounterAmount::Literal { value: 1 },
        },
        StateAction::CounterDecrement {
            field: "count".into(),
            index: 0,
            amount: CounterAmount::Literal { value: 1 },
        },
        StateAction::CounterReset {
            field: "count".into(),
            index: 0,
        },
    ] {
        let mut c = root(
            uint64(),
            StateReturn::CounterRead {
                field: "count".into(),
                index: 0,
            },
            vec![action],
        );
        slot(&mut c, "count", LedgerFieldKind::Counter);
        included(&c, "root", "counter action/read");
    }
    let mut c = root(
        Type::Bytes { length: 32 },
        StateReturn::Expression {
            value: Expr::FieldToBytes32 {
                value: Box::new(Expr::FieldCast {
                    value: Box::new(Expr::CounterRead {
                        field: "count".into(),
                        index: 0,
                    }),
                }),
            },
        },
        vec![],
    );
    slot(&mut c, "count", LedgerFieldKind::Counter);
    included(&c, "root", "counter bytes");
    // Cell old codec relation; products are synthetic canonical IR, not frontend evidence.
    for ty in [
        Type::Boolean,
        Type::Field,
        uint64(),
        Type::Bytes { length: 32 },
        Type::Struct {
            name: "Empty".into(),
            fields: vec![],
        },
        Type::Tuple {
            elements: vec![Type::Field, Type::Boolean],
        },
        Type::Vector {
            length: 2,
            element: Box::new(Type::Field),
        },
    ] {
        let mut c = root(
            ty.clone(),
            StateReturn::CellRead {
                field: "cell".into(),
                index: 0,
            },
            vec![StateAction::CellWrite {
                field: "cell".into(),
                index: 0,
                value: var("value"),
            }],
        );
        c.stateful_circuits[0].parameters = vec![parameter("value", ty.clone())];
        slot(&mut c, "cell", LedgerFieldKind::Cell { ty });
        included(&c, "root", "Cell codec/write/read");
    }
    for action in ["set_insert", "set_remove", "set_reset"] {
        for (ret, ty) in [
            (
                json!({"kind":"set_member","field":"items","index":0,"value":{"kind":"field_literal","value":"1"}}),
                Type::Boolean,
            ),
            (
                json!({"kind":"set_size","field":"items","index":0}),
                uint64(),
            ),
            (
                json!({"kind":"set_is_empty","field":"items","index":0}),
                Type::Boolean,
            ),
        ] {
            let mut a = json!({"kind":action,"field":"items","index":0});
            if action != "set_reset" {
                a["value"] = json!({"kind":"field_literal","value":"1"});
            }
            let mut c = root(
                ty,
                serde_json::from_value(ret).unwrap(),
                vec![serde_json::from_value(a).unwrap()],
            );
            slot(&mut c, "items", LedgerFieldKind::Set { ty: Type::Field });
            included(&c, "root", action);
        }
    }
    let maybe = Type::Struct {
        name: "Maybe".into(),
        fields: vec![
            StructField {
                name: "is_some".into(),
                ty: Type::Boolean,
            },
            StructField {
                name: "value".into(),
                ty: Type::Field,
            },
        ],
    };
    for action in ["list_push_front", "list_pop_front", "list_reset"] {
        for (ret, ty) in [
            ("list_length", uint64()),
            ("list_is_empty", Type::Boolean),
            ("list_head", maybe.clone()),
        ] {
            let mut a = json!({"kind":action,"field":"items","index":0});
            if action == "list_push_front" {
                a["value"] = json!({"kind":"field_literal","value":"1"});
            }
            let mut c = root(
                ty,
                serde_json::from_value(json!({"kind":ret,"field":"items","index":0})).unwrap(),
                vec![serde_json::from_value(a).unwrap()],
            );
            slot(&mut c, "items", LedgerFieldKind::List { ty: Type::Field });
            included(&c, "root", ret);
        }
    }
    for action in [
        "map_insert",
        "map_insert_default",
        "map_remove",
        "map_reset",
    ] {
        for (ret, ty) in [
            ("map_member", Type::Boolean),
            ("map_lookup", Type::Field),
            ("map_size", uint64()),
            ("map_is_empty", Type::Boolean),
        ] {
            let mut a = json!({"kind":action,"field":"items","index":0});
            if action != "map_reset" {
                a["key"] = json!({"kind":"field_literal","value":"1"});
            }
            if action == "map_insert" {
                a["value"] = json!({"kind":"field_literal","value":"2"});
            }
            let mut r = json!({"kind":ret,"field":"items","index":0});
            if matches!(ret, "map_member" | "map_lookup") {
                r["key"] = json!({"kind":"field_literal","value":"1"});
            }
            let mut c = root(
                ty,
                serde_json::from_value(r).unwrap(),
                vec![serde_json::from_value(a).unwrap()],
            );
            slot(
                &mut c,
                "items",
                LedgerFieldKind::Map {
                    key: Type::Field,
                    value: Type::Field,
                },
            );
            included(&c, "root", ret);
        }
    }
    let mut nested = root(
        Type::Boolean,
        StateReturn::MapMember {
            field: "items".into(),
            index: 0,
            key: field(1),
        },
        vec![StateAction::MapRemove {
            field: "items".into(),
            index: 0,
            key: field(1),
        }],
    );
    slot(
        &mut nested,
        "items",
        LedgerFieldKind::Map {
            key: Type::Field,
            value: Type::LedgerMap {
                key: Box::new(Type::Field),
                value: Box::new(Type::Field),
            },
        },
    );
    included(
        &nested,
        "root",
        "key-only nested remove/member, no witness view",
    );
}
