// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Collection/tree mutations keep one fallible frame operation and no Unit payload.
use super::*;
use crate::ir::{LedgerField, LedgerFieldKind};

fn declarations(kind: LedgerFieldKind) -> Declarations {
    let field = LedgerField {
        id: "store".into(),
        index: 0,
        path: vec![],
        source: None,
        declaration: kind,
    };
    Declarations::new(
        &std::collections::HashMap::from([("store", &field)]),
        &Default::default(),
    )
}

fn field(id: usize) -> Value {
    Value {
        id,
        ty: Type::Field,
    }
}

fn tree_slot() -> MerkleSlot {
    MerkleSlot {
        declaration: LedgerId(0),
        index: 0,
        depth: 16,
        historic: true,
    }
}

fn tree_declarations() -> Declarations {
    declarations(LedgerFieldKind::HistoricMerkleTree {
        depth: 16,
        ty: Type::Field,
    })
}

fn check_action(statement: &syn::Stmt, method: &str, arguments: usize) {
    let syn::Stmt::Local(local) = statement else {
        panic!("frame binding lost")
    };
    let syn::Pat::Ident(target) = &local.pat else {
        panic!("synthetic payload retained")
    };
    assert_eq!(target.ident, "frame");
    let syn::Expr::Try(call) = local.init.as_ref().unwrap().expr.as_ref() else {
        panic!("operation failure propagation lost")
    };
    let syn::Expr::MethodCall(call) = call.expr.as_ref() else {
        panic!("slot call lost")
    };
    assert_eq!(call.method, method);
    let syn::Expr::Path(receiver) = call.receiver.as_ref() else {
        panic!("slot identity lost")
    };
    assert_eq!(receiver.path.segments.last().unwrap().ident, "store");
    assert_eq!(call.args.len(), arguments + 1);
    let syn::Expr::Path(frame) = call.args.first().unwrap() else {
        panic!("input frame lost")
    };
    assert!(frame.path.is_ident("frame"));
}

#[test]
fn map_mutations_keep_the_fallible_operation_without_unit_binding() {
    let slot = MapSlot {
        declaration: LedgerId(0),
        index: 0,
    };
    let declarations = declarations(LedgerFieldKind::Map {
        key: Type::Field,
        value: Type::Field,
    });
    for (operation, method, arguments) in [
        (
            Operation::MapInsert {
                slot,
                key: field(1),
                value: field(2),
            },
            "record_insert",
            2,
        ),
        (
            Operation::MapRemove {
                slot,
                key: field(1),
            },
            "record_remove",
            1,
        ),
        (
            Operation::MapInsertDefault {
                slot,
                key: field(1),
            },
            "record_insert_default",
            1,
        ),
        (Operation::MapReset { slot }, "record_reset", 0),
    ] {
        let mut steps = Vec::new();
        maps::render(&output(operation), &declarations, &mut steps).unwrap();
        assert_eq!(steps.len(), 1);
        check_action(&steps[0], method, arguments);
    }
}

#[test]
fn list_mutations_keep_the_fallible_operation_without_unit_binding() {
    let slot = ListSlot {
        declaration: LedgerId(0),
        index: 0,
    };
    let declarations = declarations(LedgerFieldKind::List { ty: Type::Field });
    for (operation, method, arguments) in [
        (ListOperation::PushFront(field(1)), "record_push_front", 1),
        (ListOperation::PopFront, "record_pop_front", 0),
        (ListOperation::Reset, "record_reset", 0),
    ] {
        let mut steps = Vec::new();
        lists::render(
            &output(Operation::Literal(Literal::Unit)),
            slot,
            &operation,
            &declarations,
            &mut steps,
        )
        .unwrap();
        assert_eq!(steps.len(), 1);
        check_action(&steps[0], method, arguments);
    }
}

#[test]
fn merkle_mutations_keep_the_fallible_operation_without_unit_binding() {
    let index = Value {
        id: 2,
        ty: Type::Unsigned {
            max: "18446744073709551615".into(),
        },
    };
    let hash = Value {
        id: 3,
        ty: Type::Bytes { length: 32 },
    };
    for (operation, method, arguments) in [
        (
            MerkleOperation::Insert {
                value: field(1),
                position: None,
            },
            "record_insert",
            1,
        ),
        (
            MerkleOperation::Insert {
                value: field(1),
                position: Some(index.clone()),
            },
            "record_insert_index",
            2,
        ),
        (
            MerkleOperation::InsertDefault {
                position: index.clone(),
            },
            "record_insert_index_default",
            1,
        ),
        (
            MerkleOperation::InsertHash {
                hash: hash.clone(),
                position: None,
            },
            "record_insert_hash",
            1,
        ),
        (
            MerkleOperation::InsertHash {
                hash,
                position: Some(index),
            },
            "record_insert_hash_index",
            2,
        ),
        (MerkleOperation::ResetDefault, "record_reset_to_default", 0),
        (MerkleOperation::ResetHistory, "record_reset_history", 0),
    ] {
        let mut steps = Vec::new();
        merkle::render(
            &output(Operation::Literal(Literal::Unit)),
            tree_slot(),
            &operation,
            &tree_declarations(),
            &mut steps,
        )
        .unwrap();
        assert_eq!(steps.len(), 1);
        check_action(&steps[0], method, arguments);
    }
}

fn check_query(steps: &[syn::Stmt]) {
    let syn::Stmt::Local(local) = &steps[0] else {
        panic!("query binding lost")
    };
    let syn::Pat::Type(typed) = &local.pat else {
        panic!("query payload type lost")
    };
    let syn::Pat::Tuple(pair) = typed.pat.as_ref() else {
        panic!("query output lost")
    };
    assert_eq!(pair.elems.len(), 2);
    let syn::Pat::Ident(payload) = pair.elems.last().unwrap() else {
        panic!("query payload lost")
    };
    assert_eq!(payload.ident, "__compact_effect_7");
    assert!(matches!(
        local.init.as_ref().unwrap().expr.as_ref(),
        syn::Expr::Try(_)
    ));
}

#[test]
fn collection_and_tree_queries_still_bind_their_payloads() {
    let slot = MapSlot {
        declaration: LedgerId(0),
        index: 0,
    };
    let declared = declarations(LedgerFieldKind::Map {
        key: Type::Field,
        value: Type::Field,
    });
    for (operation, ty) in [
        (
            Operation::MapMember {
                slot,
                key: field(1),
            },
            Type::Boolean,
        ),
        (
            Operation::MapLookup {
                slot,
                key: field(1),
            },
            Type::Field,
        ),
        (Operation::MapIsEmpty { slot }, Type::Boolean),
    ] {
        let mut binding = output(operation);
        binding.value.ty = ty;
        let mut steps = Vec::new();
        maps::render(&binding, &declared, &mut steps).unwrap();
        assert_eq!(steps.len(), 1);
        check_query(&steps);
    }
    let mut binding = output(Operation::Literal(Literal::Unit));
    binding.value.ty = Type::Boolean;
    let mut steps = Vec::new();
    lists::render(
        &binding,
        ListSlot {
            declaration: LedgerId(0),
            index: 0,
        },
        &ListOperation::IsEmpty,
        &declarations(LedgerFieldKind::List { ty: Type::Field }),
        &mut steps,
    )
    .unwrap();
    check_query(&steps);
    for operation in [
        MerkleOperation::IsFull,
        MerkleOperation::CheckRoot { root: field(1) },
    ] {
        let mut steps = Vec::new();
        merkle::render(
            &binding,
            tree_slot(),
            &operation,
            &tree_declarations(),
            &mut steps,
        )
        .unwrap();
        assert_eq!(steps.len(), 1);
        check_query(&steps);
    }
}

#[test]
fn ordered_mutations_keep_each_frame_and_failure_boundary() {
    let slot = MapSlot {
        declaration: LedgerId(0),
        index: 0,
    };
    let declarations = declarations(LedgerFieldKind::Map {
        key: Type::Field,
        value: Type::Field,
    });
    let mut steps = Vec::new();
    for operation in [
        Operation::MapReset { slot },
        Operation::MapRemove {
            slot,
            key: field(1),
        },
    ] {
        maps::render(&output(operation), &declarations, &mut steps).unwrap();
    }
    assert_eq!(steps.len(), 2);
    check_action(&steps[0], "record_reset", 0);
    check_action(&steps[1], "record_remove", 1);
    assert!(matches!(value(&unit(7)), syn::Expr::Tuple(unit) if unit.elems.is_empty()));
}
