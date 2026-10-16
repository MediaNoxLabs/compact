// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//  	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Actual Coin/Merkle guard sites; derived types are policy, never source owners.
use super::*;
use crate::ir::Expr;
use crate::source_diagnostics::Slot;
use std::cell::RefCell;

#[derive(Default)]
struct Trace(RefCell<Vec<(&'static str, usize, Slot)>>);
fn key(owner: Owner<'_>, slot: Slot) -> (&'static str, usize, Slot) {
    let (kind, pointer) = match owner {
        Owner::Action(value) => ("action", std::ptr::from_ref(value) as usize),
        Owner::Expr(value) => ("expr", std::ptr::from_ref(value) as usize),
        Owner::Ledger(value) => ("ledger", std::ptr::from_ref(value) as usize),
        _ => panic!("unexpected Coin/Merkle guard owner"),
    };
    (kind, pointer, slot)
}
impl RefusalObserver for Trace {
    fn refused(&self, site: RefusalSite<'_>) {
        self.0
            .borrow_mut()
            .push(key(site.owner.expect("actual original source"), site.slot));
    }
}
fn one(trace: &Trace, owner: Owner<'_>, slot: Slot) {
    assert_eq!(trace.0.borrow().as_slice(), &[key(owner, slot)]);
}
fn run(
    action: &StateAction,
    fields: &HashMap<&str, &LedgerField>,
    observer: Option<&dyn RefusalObserver>,
) -> (Result<(), RenderError>, String, usize, bool, usize) {
    let witnesses = HashMap::new();
    let circuits = HashMap::new();
    let stateful_circuits = HashMap::new();
    let coin_type = shielded_coin_type();
    let recipient_type = shielded_recipient_type();
    let parameters = HashMap::from([
        ("coin", (&coin_type, syn::parse_quote!(coin))),
        ("recipient", (&recipient_type, syn::parse_quote!(recipient))),
    ]);
    let environment = NativeExpressionContext {
        witnesses: &witnesses,
        circuits: &circuits,
        stateful_circuits: &stateful_circuits,
        ledger_fields: fields,
        observer,
    };
    let mut statements = vec![syn::parse_quote!(let __guard_seed = ();)];
    let mut next_temp = 17;
    let mut uses_witness = false;
    let result = if observer.is_some() {
        super::super::lower_observed(
            action,
            &parameters,
            &environment,
            ActionOutput {
                statements: &mut statements,
                next_temp: &mut next_temp,
                uses_witness: &mut uses_witness,
            },
        )
    } else {
        super::super::lower(
            action,
            &parameters,
            fields,
            &witnesses,
            &circuits,
            &stateful_circuits,
            &mut statements,
            &mut next_temp,
            &mut uses_witness,
        )
    };
    let length = statements.len();
    (
        result,
        quote::quote!(#(#statements)*).to_string(),
        next_temp,
        uses_witness,
        length,
    )
}
fn error(
    action: &StateAction,
    fields: &HashMap<&str, &LedgerField>,
    trace: &Trace,
    expected: RenderError,
    state: (usize, bool, usize),
) {
    let bare = run(action, fields, None);
    let observed = run(action, fields, Some(trace));
    assert_eq!(
        observed, bare,
        "error, statements, temporary count and witness state"
    );
    assert_eq!(observed.0, Err(expected));
    assert_eq!((observed.2, observed.3, observed.4), state);
    if state.2 == 1 {
        assert_eq!(
            observed.1,
            quote::quote!(let __guard_seed = ();).to_string()
        );
    }
}
fn field(id: &str, declaration: LedgerFieldKind) -> LedgerField {
    LedgerField {
        id: id.into(),
        index: 0,
        path: vec![0],
        source: None,
        declaration,
    }
}
fn literal() -> Expr {
    Expr::FieldLiteral { value: "1".into() }
}
fn parameter(name: &str) -> Expr {
    Expr::Parameter { name: name.into() }
}
fn position() -> Expr {
    Expr::UnsignedLiteral {
        value: "0".into(),
        max: u64::MAX.to_string(),
    }
}
fn hash() -> Expr {
    Expr::BytesLiteral { bytes: vec![0; 32] }
}
fn coin_cases(id: &str) -> Vec<(LedgerField, StateAction)> {
    let ty = crate::coin_shapes::qualified_coin_type();
    vec![
        (
            field(id, LedgerFieldKind::Set { ty: ty.clone() }),
            StateAction::SetInsertCoin {
                field: id.into(),
                index: 0,
                coin: parameter("coin"),
                recipient: parameter("recipient"),
            },
        ),
        (
            field(id, LedgerFieldKind::Cell { ty }),
            StateAction::CellWriteCoin {
                field: id.into(),
                index: 0,
                coin: parameter("coin"),
                recipient: parameter("recipient"),
            },
        ),
    ]
}
fn merkle_cases(id: &str) -> Vec<(LedgerField, StateAction)> {
    let plain = || {
        field(
            id,
            LedgerFieldKind::MerkleTree {
                depth: 8,
                ty: Type::Field,
            },
        )
    };
    let historic = || {
        field(
            id,
            LedgerFieldKind::HistoricMerkleTree {
                depth: 8,
                ty: Type::Field,
            },
        )
    };
    vec![
        (
            historic(),
            StateAction::HistoricMerkleResetHistory {
                field: id.into(),
                index: 0,
            },
        ),
        (
            historic(),
            StateAction::HistoricMerkleResetToDefault {
                field: id.into(),
                index: 0,
            },
        ),
        (
            plain(),
            StateAction::MerkleResetToDefault {
                field: id.into(),
                index: 0,
            },
        ),
        (
            historic(),
            StateAction::HistoricMerkleInsert {
                field: id.into(),
                index: 0,
                value: literal(),
            },
        ),
        (
            plain(),
            StateAction::MerkleInsert {
                field: id.into(),
                index: 0,
                value: literal(),
            },
        ),
        (
            historic(),
            StateAction::HistoricMerkleInsertIndex {
                field: id.into(),
                index: 0,
                value: literal(),
                position: position(),
            },
        ),
        (
            plain(),
            StateAction::MerkleInsertIndex {
                field: id.into(),
                index: 0,
                value: literal(),
                position: position(),
            },
        ),
        (
            historic(),
            StateAction::HistoricMerkleInsertHash {
                field: id.into(),
                index: 0,
                hash: hash(),
            },
        ),
        (
            plain(),
            StateAction::MerkleInsertHash {
                field: id.into(),
                index: 0,
                hash: hash(),
            },
        ),
        (
            historic(),
            StateAction::HistoricMerkleInsertHashIndex {
                field: id.into(),
                index: 0,
                hash: hash(),
                position: position(),
            },
        ),
        (
            plain(),
            StateAction::MerkleInsertHashIndex {
                field: id.into(),
                index: 0,
                hash: hash(),
                position: position(),
            },
        ),
        (
            historic(),
            StateAction::HistoricMerkleInsertIndexDefault {
                field: id.into(),
                index: 0,
                position: position(),
            },
        ),
        (
            plain(),
            StateAction::MerkleInsertIndexDefault {
                field: id.into(),
                index: 0,
                position: position(),
            },
        ),
    ]
}

#[test]
fn all_fifteen_variants_lookup_failure_keeps_original_action_field() {
    for (_, action) in coin_cases("absent")
        .into_iter()
        .chain(merkle_cases("absent"))
    {
        let trace = Trace::default();
        error(
            &action,
            &HashMap::new(),
            &trace,
            RenderError::UnknownLedgerField("absent".into()),
            (17, false, 1),
        );
        one(&trace, Owner::Action(&action), Slot::Field("field"));
    }
}

#[test]
fn coin_exact_kind_type_or_index_failure_keeps_whole_node_before_operands() {
    for (mut target, mut action) in coin_cases("crate") {
        let (StateAction::SetInsertCoin {
            coin, recipient, ..
        }
        | StateAction::CellWriteCoin {
            coin, recipient, ..
        }) = &mut action
        else {
            panic!("fixture")
        };
        *coin = parameter("later");
        *recipient = parameter("later");
        for wrong_kind in [true, false] {
            let wrong = field(
                "crate",
                if wrong_kind {
                    LedgerFieldKind::Counter
                } else {
                    target.declaration.clone()
                },
            );
            let mut wrong = wrong;
            wrong.index = if wrong_kind { 0 } else { 1 };
            let trace = Trace::default();
            error(
                &action,
                &HashMap::from([("crate", &wrong)]),
                &trace,
                RenderError::UnknownLedgerField("crate".into()),
                (17, false, 1),
            );
            one(&trace, Owner::Action(&action), Slot::Node);
        }
        target.declaration = match target.declaration {
            LedgerFieldKind::Cell { .. } => LedgerFieldKind::Cell { ty: Type::Field },
            LedgerFieldKind::Set { .. } => LedgerFieldKind::Set { ty: Type::Field },
            _ => panic!("fixture"),
        };
        let trace = Trace::default();
        error(
            &action,
            &HashMap::from([("crate", &target)]),
            &trace,
            RenderError::UnknownLedgerField("crate".into()),
            (17, false, 1),
        );
        one(&trace, Owner::Action(&action), Slot::Node);
    }
}

#[test]
fn merkle_kind_index_and_historic_guards_keep_original_action_without_component_guess() {
    for (mut target, action) in merkle_cases("crate") {
        let wrong = field("crate", LedgerFieldKind::Counter);
        let trace = Trace::default();
        error(
            &action,
            &HashMap::from([("crate", &wrong)]),
            &trace,
            RenderError::UnknownLedgerField("crate".into()),
            (17, false, 1),
        );
        let slot = if matches!(action, StateAction::HistoricMerkleResetHistory { .. }) {
            Slot::Node
        } else {
            Slot::Field("field")
        };
        one(&trace, Owner::Action(&action), slot);
        let swapped = field(
            "crate",
            match &target.declaration {
                LedgerFieldKind::HistoricMerkleTree { .. } => LedgerFieldKind::MerkleTree {
                    depth: 8,
                    ty: Type::Field,
                },
                LedgerFieldKind::MerkleTree { .. } => LedgerFieldKind::HistoricMerkleTree {
                    depth: 8,
                    ty: Type::Field,
                },
                _ => panic!("fixture"),
            },
        );
        let trace = Trace::default();
        error(
            &action,
            &HashMap::from([("crate", &swapped)]),
            &trace,
            RenderError::UnknownLedgerField("crate".into()),
            (17, false, 1),
        );
        one(&trace, Owner::Action(&action), Slot::Node);
        target.index = 1;
        let trace = Trace::default();
        error(
            &action,
            &HashMap::from([("crate", &target)]),
            &trace,
            RenderError::UnknownLedgerField("crate".into()),
            (17, false, 1),
        );
        one(&trace, Owner::Action(&action), Slot::Node);
    }
}

#[test]
fn coin_type_then_recipient_type_keep_original_operands_and_temp_boundary() {
    for recipient_failure in [false, true] {
        for (target, mut action) in coin_cases("crate") {
            let (StateAction::SetInsertCoin {
                coin, recipient, ..
            }
            | StateAction::CellWriteCoin {
                coin, recipient, ..
            }) = &mut action
            else {
                panic!("fixture")
            };
            if recipient_failure {
                *recipient = Expr::Boolean { value: true };
            } else {
                *coin = literal();
                *recipient = parameter("later");
            }
            let (StateAction::SetInsertCoin {
                coin, recipient, ..
            }
            | StateAction::CellWriteCoin {
                coin, recipient, ..
            }) = &action
            else {
                panic!("fixture")
            };
            let trace = Trace::default();
            let (expected, actual, original, next) = if recipient_failure {
                (shielded_recipient_type(), Type::Boolean, recipient, 18)
            } else {
                (shielded_coin_type(), Type::Field, coin, 17)
            };
            error(
                &action,
                &HashMap::from([("crate", &target)]),
                &trace,
                RenderError::TypeMismatch { expected, actual },
                (next, false, 1),
            );
            one(&trace, Owner::Expr(original), Slot::Node);
        }
    }
}

#[test]
fn all_merkle_value_hash_and_position_type_sites_preserve_original_roles_and_order() {
    for (target, mut action) in merkle_cases("crate") {
        let expected = match &mut action {
            StateAction::HistoricMerkleInsert { value, .. }
            | StateAction::MerkleInsert { value, .. } => {
                *value = Expr::Boolean { value: true };
                Type::Field
            }
            StateAction::HistoricMerkleInsertIndex {
                value, position, ..
            }
            | StateAction::MerkleInsertIndex {
                value, position, ..
            } => {
                *value = Expr::Boolean { value: true };
                *position = parameter("later");
                Type::Field
            }
            StateAction::HistoricMerkleInsertHash { hash, .. }
            | StateAction::MerkleInsertHash { hash, .. } => {
                *hash = Expr::Boolean { value: true };
                Type::Bytes { length: 32 }
            }
            StateAction::HistoricMerkleInsertHashIndex { hash, position, .. }
            | StateAction::MerkleInsertHashIndex { hash, position, .. } => {
                *hash = Expr::Boolean { value: true };
                *position = parameter("later");
                Type::Bytes { length: 32 }
            }
            _ => continue,
        };
        let original = match &action {
            StateAction::HistoricMerkleInsert { value, .. }
            | StateAction::MerkleInsert { value, .. }
            | StateAction::HistoricMerkleInsertIndex { value, .. }
            | StateAction::MerkleInsertIndex { value, .. } => value,
            StateAction::HistoricMerkleInsertHash { hash, .. }
            | StateAction::MerkleInsertHash { hash, .. }
            | StateAction::HistoricMerkleInsertHashIndex { hash, .. }
            | StateAction::MerkleInsertHashIndex { hash, .. } => hash,
            _ => panic!("fixture"),
        };
        let trace = Trace::default();
        error(
            &action,
            &HashMap::from([("crate", &target)]),
            &trace,
            RenderError::TypeMismatch {
                expected,
                actual: Type::Boolean,
            },
            (17, false, 1),
        );
        one(&trace, Owner::Expr(original), Slot::Node);
    }
    for (target, mut action) in merkle_cases("crate") {
        let position = match &mut action {
            StateAction::HistoricMerkleInsertIndex { position, .. }
            | StateAction::MerkleInsertIndex { position, .. }
            | StateAction::HistoricMerkleInsertHashIndex { position, .. }
            | StateAction::MerkleInsertHashIndex { position, .. }
            | StateAction::HistoricMerkleInsertIndexDefault { position, .. }
            | StateAction::MerkleInsertIndexDefault { position, .. } => position,
            _ => continue,
        };
        *position = Expr::Boolean { value: true };
        let original = match &action {
            StateAction::HistoricMerkleInsertIndex { position, .. }
            | StateAction::MerkleInsertIndex { position, .. }
            | StateAction::HistoricMerkleInsertHashIndex { position, .. }
            | StateAction::MerkleInsertHashIndex { position, .. }
            | StateAction::HistoricMerkleInsertIndexDefault { position, .. }
            | StateAction::MerkleInsertIndexDefault { position, .. } => position,
            _ => panic!("fixture"),
        };
        let trace = Trace::default();
        error(
            &action,
            &HashMap::from([("crate", &target)]),
            &trace,
            RenderError::TypeMismatch {
                expected: Type::Unsigned {
                    max: u64::MAX.to_string(),
                },
                actual: Type::Boolean,
            },
            (17, false, 1),
        );
        one(&trace, Owner::Expr(original), Slot::Node);
    }
}

#[test]
fn coin_partial_temporaries_and_late_identifier_error_remain_exact() {
    for (target, mut action) in coin_cases("crate") {
        let trace = Trace::default();
        error(
            &action,
            &HashMap::from([("crate", &target)]),
            &trace,
            RenderError::InvalidIdentifier("crate".into()),
            (19, false, 3),
        );
        one(&trace, Owner::Ledger(&target), Slot::Field("id"));
        let (StateAction::SetInsertCoin { recipient, .. }
        | StateAction::CellWriteCoin { recipient, .. }) = &mut action
        else {
            panic!("fixture")
        };
        *recipient = parameter("later");
        let trace = Trace::default();
        error(
            &action,
            &HashMap::from([("crate", &target)]),
            &trace,
            RenderError::UnknownParameter("later".into()),
            (18, false, 1),
        );
        // State-expression refusal owns the original recipient; new outer guard is not reached.
        let (StateAction::SetInsertCoin { recipient, .. }
        | StateAction::CellWriteCoin { recipient, .. }) = &action
        else {
            panic!("fixture")
        };
        one(&trace, Owner::Expr(recipient), Slot::Node);
        let (StateAction::SetInsertCoin { coin, .. } | StateAction::CellWriteCoin { coin, .. }) =
            &mut action
        else {
            panic!("fixture")
        };
        *coin = Expr::Add {
            left: Box::new(literal()),
            right: Box::new(literal()),
        };
        let (StateAction::SetInsertCoin { coin, .. } | StateAction::CellWriteCoin { coin, .. }) =
            &action
        else {
            panic!("fixture")
        };
        let trace = Trace::default();
        error(
            &action,
            &HashMap::from([("crate", &target)]),
            &trace,
            RenderError::TypeMismatch {
                expected: shielded_coin_type(),
                actual: Type::Field,
            },
            (19, false, 1),
        );
        one(&trace, Owner::Expr(coin), Slot::Node);
    }
}

#[test]
fn nested_merkle_first_leaf_precedes_position_type_and_invalid_ledger_name() {
    let target = field(
        "crate",
        LedgerFieldKind::MerkleTree {
            depth: 8,
            ty: Type::Field,
        },
    );
    let action = StateAction::MerkleInsertIndex {
        field: "crate".into(),
        index: 0,
        value: Expr::Add {
            left: Box::new(Expr::FieldLiteral {
                value: "bad".into(),
            }),
            right: Box::new(parameter("later")),
        },
        position: Expr::Boolean { value: true },
    };
    let StateAction::MerkleInsertIndex {
        value: Expr::Add { left, right },
        position,
        ..
    } = &action
    else {
        panic!("fixture")
    };
    let trace = Trace::default();
    error(
        &action,
        &HashMap::from([("crate", &target)]),
        &trace,
        RenderError::InvalidFieldLiteral("bad".into()),
        (17, false, 1),
    );
    assert_eq!(
        trace.0.borrow().first(),
        Some(&key(Owner::Expr(left), Slot::Node))
    );
    assert!(trace.0.borrow().iter().all(|row| row.0 == "expr"
        && row.1 != std::ptr::from_ref(right.as_ref()) as usize
        && row.1 != std::ptr::from_ref(position) as usize));
}

#[test]
fn all_valid_coin_merkle_variants_keep_output_seed_temporaries_and_no_refusals() {
    for (target, action) in coin_cases("slot").into_iter().chain(merkle_cases("slot")) {
        let trace = Trace::default();
        let fields = HashMap::from([("slot", &target)]);
        let bare = run(&action, &fields, None);
        let observed = run(&action, &fields, Some(&trace));
        assert_eq!(observed, bare);
        assert_eq!(observed.0, Ok(()));
        let state = if matches!(
            action,
            StateAction::SetInsertCoin { .. } | StateAction::CellWriteCoin { .. }
        ) {
            (19, false, 6)
        } else {
            (17, false, 4)
        };
        assert_eq!((observed.2, observed.3, observed.4), state);
        assert!(trace.0.borrow().is_empty());
    }
}

#[test]
fn equal_live_action_and_hash_occurrences_keep_distinct_original_identity() {
    let original = StateAction::MerkleResetToDefault {
        field: "absent".into(),
        index: 0,
    };
    let actions = [original.clone(), original];
    let mut ids = Vec::new();
    for action in &actions {
        let trace = Trace::default();
        error(
            action,
            &HashMap::new(),
            &trace,
            RenderError::UnknownLedgerField("absent".into()),
            (17, false, 1),
        );
        one(&trace, Owner::Action(action), Slot::Field("field"));
        ids.push(key(Owner::Action(action), Slot::Field("field")));
    }
    assert_ne!(ids[0], ids[1]);
    let original = StateAction::MerkleInsertHash {
        field: "slot".into(),
        index: 0,
        hash: Expr::Boolean { value: true },
    };
    let actions = [original.clone(), original];
    let mut ids = Vec::new();
    let target = field(
        "slot",
        LedgerFieldKind::MerkleTree {
            depth: 8,
            ty: Type::Field,
        },
    );
    for action in &actions {
        let StateAction::MerkleInsertHash { hash, .. } = action else {
            panic!("fixture")
        };
        let trace = Trace::default();
        error(
            action,
            &HashMap::from([("slot", &target)]),
            &trace,
            RenderError::TypeMismatch {
                expected: Type::Bytes { length: 32 },
                actual: Type::Boolean,
            },
            (17, false, 1),
        );
        one(&trace, Owner::Expr(hash), Slot::Node);
        ids.push(key(Owner::Expr(hash), Slot::Node));
    }
    assert_ne!(ids[0], ids[1]);
}
