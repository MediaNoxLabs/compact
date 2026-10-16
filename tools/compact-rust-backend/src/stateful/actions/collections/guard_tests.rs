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

//! Existing native collection failure producers and exact original occurrence identity.
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
        _ => panic!("unexpected collection guard owner"),
    };
    (kind, pointer, slot)
}
impl RefusalObserver for Trace {
    fn refused(&self, site: RefusalSite<'_>) {
        self.0
            .borrow_mut()
            .push(key(site.owner.expect("original source"), site.slot));
    }
}
fn one(trace: &Trace, owner: Owner<'_>, slot: Slot) {
    assert_eq!(trace.0.borrow().as_slice(), &[key(owner, slot)]);
}
fn literal(value: &str) -> Expr {
    Expr::FieldLiteral {
        value: value.into(),
    }
}
fn missing() -> Expr {
    Expr::Parameter {
        name: "later".into(),
    }
}
fn ledger(id: &str, declaration: LedgerFieldKind) -> LedgerField {
    LedgerField {
        source: None,
        id: id.into(),
        index: 0,
        path: vec![0],
        declaration,
    }
}
fn map_kind() -> LedgerFieldKind {
    LedgerFieldKind::Map {
        key: Type::Field,
        value: Type::Field,
    }
}
fn cases(id: &str) -> Vec<(LedgerField, StateAction)> {
    vec![
        (
            ledger(id, LedgerFieldKind::List { ty: Type::Field }),
            StateAction::ListPushFront {
                field: id.into(),
                index: 0,
                value: literal("1"),
            },
        ),
        (
            ledger(id, LedgerFieldKind::List { ty: Type::Field }),
            StateAction::ListPopFront {
                field: id.into(),
                index: 0,
            },
        ),
        (
            ledger(id, LedgerFieldKind::List { ty: Type::Field }),
            StateAction::ListReset {
                field: id.into(),
                index: 0,
            },
        ),
        (
            ledger(id, LedgerFieldKind::Set { ty: Type::Field }),
            StateAction::SetInsert {
                field: id.into(),
                index: 0,
                value: literal("1"),
            },
        ),
        (
            ledger(id, LedgerFieldKind::Set { ty: Type::Field }),
            StateAction::SetRemove {
                field: id.into(),
                index: 0,
                value: literal("1"),
            },
        ),
        (
            ledger(id, LedgerFieldKind::Set { ty: Type::Field }),
            StateAction::SetReset {
                field: id.into(),
                index: 0,
            },
        ),
        (
            ledger(id, map_kind()),
            StateAction::MapInsert {
                field: id.into(),
                index: 0,
                key: literal("1"),
                value: literal("2"),
            },
        ),
        (
            ledger(id, map_kind()),
            StateAction::MapInsertDefault {
                field: id.into(),
                index: 0,
                key: literal("1"),
            },
        ),
        (
            ledger(id, map_kind()),
            StateAction::MapRemove {
                field: id.into(),
                index: 0,
                key: literal("1"),
            },
        ),
        (
            ledger(id, map_kind()),
            StateAction::MapReset {
                field: id.into(),
                index: 0,
            },
        ),
    ]
}
fn run(
    action: &StateAction,
    fields: &HashMap<&str, &LedgerField>,
    observer: Option<&dyn RefusalObserver>,
) -> (Result<(), RenderError>, String, usize, bool, usize) {
    let witnesses = HashMap::new();
    let circuits = HashMap::new();
    let stateful_circuits = HashMap::new();
    let parameters = HashMap::new();
    let environment = NativeExpressionContext {
        witnesses: &witnesses,
        circuits: &circuits,
        stateful_circuits: &stateful_circuits,
        ledger_fields: fields,
        observer,
    };
    let mut statements = vec![syn::parse_quote!(let __guard_seed = ();)];
    let mut next_temp = 19;
    let mut uses_witness = true;
    let result = if observer.is_some() {
        lower_observed(
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
        lower(action, &parameters, fields, &circuits, &mut statements)
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
) {
    let bare = run(action, fields, None);
    let observed = run(action, fields, Some(trace));
    assert_eq!(observed, bare, "exact error and caller-owned partial state");
    assert_eq!(observed.0, Err(expected));
    assert_eq!(
        observed.1,
        quote::quote!(let __guard_seed = ();).to_string()
    );
    assert_eq!((observed.2, observed.3, observed.4), (19, true, 1));
}

#[test]
fn all_ten_variants_lookup_failure_keeps_original_action_field_before_later_checks() {
    for (_, action) in cases("absent") {
        let trace = Trace::default();
        error(
            &action,
            &HashMap::new(),
            &trace,
            RenderError::UnknownLedgerField("absent".into()),
        );
        one(&trace, Owner::Action(&action), Slot::Field("field"));
    }
}

#[test]
fn distinct_collection_kind_then_index_guards_precede_operand_and_identifier() {
    for (mut target, mut action) in cases("crate") {
        let index = match &mut action {
            StateAction::ListPushFront { index, value, .. }
            | StateAction::SetInsert { index, value, .. }
            | StateAction::SetRemove { index, value, .. } => {
                *value = missing();
                index
            }
            StateAction::MapInsert {
                index, key, value, ..
            } => {
                *key = missing();
                *value = missing();
                index
            }
            StateAction::MapInsertDefault { index, key, .. }
            | StateAction::MapRemove { index, key, .. } => {
                *key = missing();
                index
            }
            _ => continue,
        };
        *index = 1;
        let wrong = ledger("crate", LedgerFieldKind::Counter);
        let trace = Trace::default();
        error(
            &action,
            &HashMap::from([("crate", &wrong)]),
            &trace,
            RenderError::UnknownLedgerField("crate".into()),
        );
        one(&trace, Owner::Action(&action), Slot::Field("field"));
        let trace = Trace::default();
        error(
            &action,
            &HashMap::from([("crate", &target)]),
            &trace,
            RenderError::UnknownLedgerField("crate".into()),
        );
        one(&trace, Owner::Action(&action), Slot::Field("index"));
        target.index = 1;
        let trace = Trace::default();
        error(
            &action,
            &HashMap::from([("crate", &target)]),
            &trace,
            RenderError::UnknownParameter("later".into()),
        );
        assert_eq!(trace.0.borrow().first().map(|row| row.0), Some("expr"));
        assert!(
            trace
                .0
                .borrow()
                .iter()
                .all(|row| row.0 != "action" && row.0 != "ledger")
        );
    }
}

#[test]
fn combined_reset_and_pop_guards_keep_whole_original_node_for_kind_or_index() {
    for (target, mut action) in cases("crate") {
        let index = match &mut action {
            StateAction::ListPopFront { index, .. }
            | StateAction::ListReset { index, .. }
            | StateAction::SetReset { index, .. }
            | StateAction::MapReset { index, .. } => index,
            _ => continue,
        };
        *index = 1;
        let wrong = ledger("crate", LedgerFieldKind::Counter);
        for declaration in [&wrong, &target] {
            let trace = Trace::default();
            error(
                &action,
                &HashMap::from([("crate", declaration)]),
                &trace,
                RenderError::UnknownLedgerField("crate".into()),
            );
            one(&trace, Owner::Action(&action), Slot::Node);
        }
    }
}

#[test]
fn all_six_type_checksites_keep_original_operand_before_later_value_or_identifier() {
    let boolean = || Expr::Boolean { value: true };
    let cases = [
        (
            LedgerFieldKind::List { ty: Type::Field },
            StateAction::ListPushFront {
                field: "crate".into(),
                index: 0,
                value: boolean(),
            },
        ),
        (
            LedgerFieldKind::Set { ty: Type::Field },
            StateAction::SetInsert {
                field: "crate".into(),
                index: 0,
                value: boolean(),
            },
        ),
        (
            LedgerFieldKind::Set { ty: Type::Field },
            StateAction::SetRemove {
                field: "crate".into(),
                index: 0,
                value: boolean(),
            },
        ),
        (
            map_kind(),
            StateAction::MapInsert {
                field: "crate".into(),
                index: 0,
                key: boolean(),
                value: missing(),
            },
        ),
        (
            map_kind(),
            StateAction::MapInsert {
                field: "crate".into(),
                index: 0,
                key: literal("1"),
                value: boolean(),
            },
        ),
        (
            map_kind(),
            StateAction::MapInsertDefault {
                field: "crate".into(),
                index: 0,
                key: boolean(),
            },
        ),
        (
            map_kind(),
            StateAction::MapRemove {
                field: "crate".into(),
                index: 0,
                key: boolean(),
            },
        ),
    ];
    for (kind, action) in &cases {
        let original = match action {
            StateAction::ListPushFront { value, .. }
            | StateAction::SetInsert { value, .. }
            | StateAction::SetRemove { value, .. } => value,
            StateAction::MapInsert { key, value, .. } => {
                if matches!(key, Expr::Boolean { .. }) {
                    key
                } else {
                    value
                }
            }
            StateAction::MapInsertDefault { key, .. } | StateAction::MapRemove { key, .. } => key,
            _ => panic!("fixture"),
        };
        let target = ledger("crate", kind.clone());
        let trace = Trace::default();
        error(
            action,
            &HashMap::from([("crate", &target)]),
            &trace,
            RenderError::TypeMismatch {
                expected: Type::Field,
                actual: Type::Boolean,
            },
        );
        one(&trace, Owner::Expr(original), Slot::Node);
    }
}

#[test]
fn nested_first_key_refusal_is_not_replaced_by_action_or_later_value_type() {
    let target = ledger("crate", map_kind());
    let action = StateAction::MapInsert {
        field: "crate".into(),
        index: 0,
        key: Expr::Add {
            left: Box::new(literal("bad")),
            right: Box::new(missing()),
        },
        value: missing(),
    };
    let StateAction::MapInsert {
        key: Expr::Add { left, right },
        value,
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
    );
    assert_eq!(
        trace.0.borrow().first(),
        Some(&key(Owner::Expr(left), Slot::Node))
    );
    assert!(trace.0.borrow().iter().all(|row| row.0 == "expr"
        && row.1 != std::ptr::from_ref(right.as_ref()) as usize
        && row.1 != std::ptr::from_ref(value) as usize));
}

#[test]
fn every_valid_variant_keeps_output_seed_temp_witness_and_zero_callbacks() {
    for (target, action) in cases("values") {
        let trace = Trace::default();
        let fields = HashMap::from([("values", &target)]);
        let bare = run(&action, &fields, None);
        let observed = run(&action, &fields, Some(&trace));
        assert_eq!(observed, bare);
        assert_eq!(observed.0, Ok(()));
        assert_eq!((observed.2, observed.3, observed.4), (19, true, 4));
        assert!(trace.0.borrow().is_empty());
    }
}

#[test]
fn equal_live_action_and_operand_occurrences_keep_distinct_borrowed_identity() {
    let original = StateAction::MapRemove {
        field: "absent".into(),
        index: 0,
        key: literal("1"),
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
        );
        one(&trace, Owner::Action(action), Slot::Field("field"));
        ids.push(key(Owner::Action(action), Slot::Field("field")));
    }
    assert_ne!(ids[0], ids[1]);
    let original = StateAction::MapRemove {
        field: "values".into(),
        index: 0,
        key: Expr::Boolean { value: true },
    };
    let actions = [original.clone(), original];
    let mut ids = Vec::new();
    let target = ledger("values", map_kind());
    for action in &actions {
        let StateAction::MapRemove {
            key: original_key, ..
        } = action
        else {
            panic!("fixture")
        };
        let trace = Trace::default();
        error(
            action,
            &HashMap::from([("values", &target)]),
            &trace,
            RenderError::TypeMismatch {
                expected: Type::Field,
                actual: Type::Boolean,
            },
        );
        one(&trace, Owner::Expr(original_key), Slot::Node);
        ids.push(key(Owner::Expr(original_key), Slot::Node));
    }
    assert_ne!(ids[0], ids[1]);
}
