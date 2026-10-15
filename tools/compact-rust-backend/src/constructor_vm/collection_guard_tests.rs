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

//! Private VM core controls; original source observations add no admission rules.
use crate::ir::{
    ConstructorStep, Expr, LedgerField, LedgerFieldKind, LocalBinding, PureCircuit, Type,
};
use crate::source_diagnostics::{Owner, RefusalObserver, RefusalSite, Slot};
use crate::{
    RenderError, constructor_vm, render_constructor_vm_steps, render_constructor_vm_steps_observed,
    stateful,
};
use std::cell::RefCell;
use std::collections::HashMap;

#[derive(Default)]
struct Trace(RefCell<Vec<(&'static str, usize, Slot)>>);
fn key(owner: Owner<'_>, slot: Slot) -> (&'static str, usize, Slot) {
    let (kind, pointer) = match owner {
        Owner::ConstructorStep(value) => ("step", std::ptr::from_ref(value) as usize),
        Owner::Expr(value) => ("expr", std::ptr::from_ref(value) as usize),
        Owner::Type(value) => ("type", std::ptr::from_ref(value) as usize),
        _ => panic!("unexpected VM collection guard owner"),
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
fn field(value: &str) -> Expr {
    Expr::FieldLiteral {
        value: value.into(),
    }
}
fn missing() -> Expr {
    Expr::Parameter {
        name: "later".into(),
    }
}
fn call() -> Expr {
    Expr::Call {
        name: "global".into(),
        arguments: vec![],
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
fn cases(id: &str) -> Vec<(LedgerField, ConstructorStep)> {
    vec![
        (
            ledger(id, LedgerFieldKind::Set { ty: Type::Field }),
            ConstructorStep::SetInsert {
                field: id.into(),
                index: 0,
                value: field("1"),
            },
        ),
        (
            ledger(id, LedgerFieldKind::Set { ty: Type::Field }),
            ConstructorStep::SetRemove {
                field: id.into(),
                index: 0,
                value: field("1"),
            },
        ),
        (
            ledger(id, LedgerFieldKind::Set { ty: Type::Field }),
            ConstructorStep::SetReset {
                field: id.into(),
                index: 0,
            },
        ),
        (
            ledger(id, LedgerFieldKind::List { ty: Type::Field }),
            ConstructorStep::ListPushFront {
                field: id.into(),
                index: 0,
                value: field("1"),
            },
        ),
        (
            ledger(id, LedgerFieldKind::List { ty: Type::Field }),
            ConstructorStep::ListPopFront {
                field: id.into(),
                index: 0,
            },
        ),
        (
            ledger(id, LedgerFieldKind::List { ty: Type::Field }),
            ConstructorStep::ListReset {
                field: id.into(),
                index: 0,
            },
        ),
        (
            ledger(id, map_kind()),
            ConstructorStep::MapInsert {
                field: id.into(),
                index: 0,
                key: field("1"),
                value: field("2"),
            },
        ),
        (
            ledger(id, map_kind()),
            ConstructorStep::MapInsertDefault {
                field: id.into(),
                index: 0,
                key: field("1"),
            },
        ),
        (
            ledger(id, map_kind()),
            ConstructorStep::MapRemove {
                field: id.into(),
                index: 0,
                key: field("1"),
            },
        ),
        (
            ledger(id, map_kind()),
            ConstructorStep::MapReset {
                field: id.into(),
                index: 0,
            },
        ),
    ]
}
fn run(
    steps: &[ConstructorStep],
    fields: &HashMap<&str, &LedgerField>,
    observer: Option<&dyn RefusalObserver>,
) -> (Result<String, RenderError>, usize, usize) {
    let parameters = HashMap::new();
    let witnesses = HashMap::new();
    let stateful_circuits = HashMap::new();
    // Existing restricted operands still must not use this available callee.
    let global = PureCircuit {
        source: None,
        name: "global".into(),
        internal: false,
        parameters: vec![],
        result: Type::Field,
        body: field("1"),
    };
    let circuits = HashMap::from([("global", &global)]);
    let environment = stateful::NativeExpressionContext {
        witnesses: &witnesses,
        circuits: &circuits,
        stateful_circuits: &stateful_circuits,
        ledger_fields: fields,
        observer,
    };
    let mut next_loop = 7;
    let mut next_temp = 9;
    let result = if observer.is_some() {
        render_constructor_vm_steps_observed(
            steps,
            &parameters,
            &environment,
            constructor_vm::Output {
                next_loop: &mut next_loop,
                next_temp: &mut next_temp,
            },
        )
    } else {
        render_constructor_vm_steps(
            steps,
            fields,
            &parameters,
            &witnesses,
            &circuits,
            &stateful_circuits,
            &mut next_loop,
            &mut next_temp,
        )
    }
    .map(|statements| quote::quote!(#(#statements)*).to_string());
    (result, next_loop, next_temp)
}
fn error(
    step: &ConstructorStep,
    fields: &HashMap<&str, &LedgerField>,
    trace: &Trace,
    expected: RenderError,
) {
    let steps = std::slice::from_ref(step);
    let bare = run(steps, fields, None);
    let observed = run(steps, fields, Some(trace));
    assert_eq!(observed, bare, "original error and loop/temp state");
    assert_eq!(observed.0, Err(expected));
    assert_eq!((observed.1, observed.2), (7, 9));
}

#[test]
fn all_ten_collection_variants_lookup_before_index_kind_or_operands() {
    for (_, step) in cases("absent") {
        let trace = Trace::default();
        error(
            &step,
            &HashMap::new(),
            &trace,
            RenderError::UnknownLedgerField("absent".into()),
        );
        one(&trace, Owner::ConstructorStep(&step), Slot::Field("field"));
    }
}

#[test]
fn distinct_vm_index_before_kind_before_infallibility_keeps_original_target_slots() {
    for (_, mut step) in cases("stored") {
        let index = match &mut step {
            ConstructorStep::SetInsert { index, value, .. }
            | ConstructorStep::SetRemove { index, value, .. }
            | ConstructorStep::ListPushFront { index, value, .. } => {
                *value = call();
                index
            }
            ConstructorStep::MapInsert {
                index, key, value, ..
            } => {
                *key = call();
                *value = call();
                index
            }
            ConstructorStep::MapInsertDefault { index, key, .. }
            | ConstructorStep::MapRemove { index, key, .. } => {
                *key = call();
                index
            }
            _ => continue,
        };
        *index = 1;
        let wrong = ledger("stored", LedgerFieldKind::Counter);
        let fields = HashMap::from([("stored", &wrong)]);
        let trace = Trace::default();
        error(&step, &fields, &trace, RenderError::InvalidLedgerIndex(1));
        one(&trace, Owner::ConstructorStep(&step), Slot::Field("index"));
        let index = match &mut step {
            ConstructorStep::SetInsert { index, .. }
            | ConstructorStep::SetRemove { index, .. }
            | ConstructorStep::ListPushFront { index, .. }
            | ConstructorStep::MapInsert { index, .. }
            | ConstructorStep::MapInsertDefault { index, .. }
            | ConstructorStep::MapRemove { index, .. } => index,
            _ => panic!("fixture"),
        };
        *index = 0;
        let trace = Trace::default();
        error(
            &step,
            &fields,
            &trace,
            RenderError::UnknownLedgerField("stored".into()),
        );
        one(&trace, Owner::ConstructorStep(&step), Slot::Field("field"));
    }
}

#[test]
fn combined_reset_and_pop_wrong_kind_or_index_uses_whole_original_step() {
    for (target, mut step) in cases("stored") {
        let index = match &mut step {
            ConstructorStep::SetReset { index, .. }
            | ConstructorStep::ListPopFront { index, .. }
            | ConstructorStep::ListReset { index, .. }
            | ConstructorStep::MapReset { index, .. } => index,
            _ => continue,
        };
        *index = 1;
        let wrong = ledger("stored", LedgerFieldKind::Counter);
        for declaration in [&wrong, &target] {
            let trace = Trace::default();
            error(
                &step,
                &HashMap::from([("stored", declaration)]),
                &trace,
                RenderError::UnknownLedgerField("stored".into()),
            );
            one(&trace, Owner::ConstructorStep(&step), Slot::Node);
        }
    }
}

#[test]
fn restricted_operand_guards_precede_folds_without_new_callee_admission() {
    for (target, mut step) in cases("stored") {
        match &mut step {
            ConstructorStep::SetInsert { value, .. }
            | ConstructorStep::SetRemove { value, .. }
            | ConstructorStep::ListPushFront { value, .. } => *value = call(),
            ConstructorStep::MapInsertDefault { key, .. }
            | ConstructorStep::MapRemove { key, .. } => *key = call(),
            ConstructorStep::MapInsert { key, value, .. } => {
                *key = missing();
                *value = call();
            }
            _ => continue,
        }
        let trace = Trace::default();
        error(
            &step,
            &HashMap::from([("stored", &target)]),
            &trace,
            RenderError::InvalidConstructorInitializer("stored".into()),
        );
        match &step {
            ConstructorStep::SetInsert { value, .. }
            | ConstructorStep::SetRemove { value, .. }
            | ConstructorStep::ListPushFront { value, .. } => {
                one(&trace, Owner::Expr(value), Slot::Node)
            }
            ConstructorStep::MapInsertDefault { key, .. }
            | ConstructorStep::MapRemove { key, .. } => one(&trace, Owner::Expr(key), Slot::Node),
            ConstructorStep::MapInsert { .. } => {
                one(&trace, Owner::ConstructorStep(&step), Slot::Node)
            }
            _ => panic!("fixture"),
        }
    }
    let target = ledger("stored", map_kind());
    for (key, value) in [(call(), missing()), (Expr::Boolean { value: true }, call())] {
        let step = ConstructorStep::MapInsert {
            field: "stored".into(),
            index: 0,
            key,
            value,
        };
        let trace = Trace::default();
        error(
            &step,
            &HashMap::from([("stored", &target)]),
            &trace,
            RenderError::InvalidConstructorInitializer("stored".into()),
        );
        one(&trace, Owner::ConstructorStep(&step), Slot::Node);
    }
}

#[test]
fn all_five_outer_type_sites_keep_original_operands_and_key_before_value() {
    for (target, mut step) in cases("stored") {
        match &mut step {
            ConstructorStep::SetInsert { value, .. }
            | ConstructorStep::SetRemove { value, .. }
            | ConstructorStep::ListPushFront { value, .. } => {
                *value = Expr::Boolean { value: true }
            }
            ConstructorStep::MapInsertDefault { key, .. }
            | ConstructorStep::MapRemove { key, .. } => *key = Expr::Boolean { value: true },
            ConstructorStep::MapInsert { key, value, .. } => {
                *key = Expr::Boolean { value: true };
                *value = missing();
            }
            _ => continue,
        }
        let original = match &step {
            ConstructorStep::SetInsert { value, .. }
            | ConstructorStep::SetRemove { value, .. }
            | ConstructorStep::ListPushFront { value, .. } => value,
            ConstructorStep::MapInsertDefault { key, .. }
            | ConstructorStep::MapRemove { key, .. }
            | ConstructorStep::MapInsert { key, .. } => key,
            _ => panic!("fixture"),
        };
        let trace = Trace::default();
        error(
            &step,
            &HashMap::from([("stored", &target)]),
            &trace,
            RenderError::TypeMismatch {
                expected: Type::Field,
                actual: Type::Boolean,
            },
        );
        one(&trace, Owner::Expr(original), Slot::Node);
    }
    let target = ledger("stored", map_kind());
    let step = ConstructorStep::MapInsert {
        field: "stored".into(),
        index: 0,
        key: field("1"),
        value: Expr::Boolean { value: true },
    };
    let ConstructorStep::MapInsert { value, .. } = &step else {
        panic!("fixture")
    };
    let trace = Trace::default();
    error(
        &step,
        &HashMap::from([("stored", &target)]),
        &trace,
        RenderError::TypeMismatch {
            expected: Type::Field,
            actual: Type::Boolean,
        },
    );
    one(&trace, Owner::Expr(value), Slot::Node);
}

#[test]
fn default_value_carrier_observes_original_borrow_only_after_key_checks() {
    let target = ledger(
        "stored",
        LedgerFieldKind::Map {
            key: Type::Field,
            value: Type::LedgerMap {
                key: Box::new(Type::Field),
                value: Box::new(Type::Field),
            },
        },
    );
    let LedgerFieldKind::Map {
        value: original, ..
    } = &target.declaration
    else {
        panic!("fixture")
    };
    let step = ConstructorStep::MapInsertDefault {
        field: "stored".into(),
        index: 0,
        key: field("1"),
    };
    let trace = Trace::default();
    error(
        &step,
        &HashMap::from([("stored", &target)]),
        &trace,
        RenderError::UnsupportedLedgerValueType(original.clone()),
    );
    one(&trace, Owner::Type(original), Slot::Node);
    let earlier = ConstructorStep::MapInsertDefault {
        field: "stored".into(),
        index: 0,
        key: Expr::Boolean { value: true },
    };
    let ConstructorStep::MapInsertDefault {
        key: original_key, ..
    } = &earlier
    else {
        panic!("fixture")
    };
    let trace = Trace::default();
    error(
        &earlier,
        &HashMap::from([("stored", &target)]),
        &trace,
        RenderError::TypeMismatch {
            expected: Type::Field,
            actual: Type::Boolean,
        },
    );
    one(&trace, Owner::Expr(original_key), Slot::Node);
    let remove = ConstructorStep::MapRemove {
        field: "stored".into(),
        index: 0,
        key: field("1"),
    };
    let trace = Trace::default();
    let observed = run(
        std::slice::from_ref(&remove),
        &HashMap::from([("stored", &target)]),
        Some(&trace),
    );
    assert!(observed.0.is_ok());
    assert_eq!(
        observed,
        run(
            std::slice::from_ref(&remove),
            &HashMap::from([("stored", &target)]),
            None
        )
    );
    assert!(
        trace.0.borrow().is_empty(),
        "MapRemove gains no unused default carrier check"
    );
}

#[test]
fn nested_first_child_failure_precedes_new_outer_guard() {
    let target = ledger("stored", map_kind());
    let step = ConstructorStep::MapInsert {
        field: "stored".into(),
        index: 0,
        key: Expr::Coerce {
            value: Box::new(field("bad")),
            ty: Type::Field,
        },
        value: missing(),
    };
    let ConstructorStep::MapInsert {
        key: Expr::Coerce { value: child, .. },
        value,
        ..
    } = &step
    else {
        panic!("fixture")
    };
    let trace = Trace::default();
    error(
        &step,
        &HashMap::from([("stored", &target)]),
        &trace,
        RenderError::InvalidFieldLiteral("bad".into()),
    );
    assert_eq!(
        trace.0.borrow().first(),
        Some(&key(Owner::Expr(child), Slot::Node))
    );
    assert!(
        trace
            .0
            .borrow()
            .iter()
            .all(|row| row.0 == "expr" && row.1 != std::ptr::from_ref(value) as usize)
    );
}

#[test]
fn earlier_steps_and_loop_temp_state_are_preserved_when_collection_refuses() {
    let target = ledger("stored", map_kind());
    let steps = [
        ConstructorStep::Let {
            bindings: vec![LocalBinding {
                name: "previous".into(),
                ty: Type::Field,
                value: field("1"),
            }],
            step: Box::new(ConstructorStep::Expression { value: Expr::Unit }),
        },
        ConstructorStep::MapReset {
            field: "stored".into(),
            index: 1,
        },
    ];
    let trace = Trace::default();
    let fields = HashMap::from([("stored", &target)]);
    let observed = run(&steps, &fields, Some(&trace));
    assert_eq!(observed, run(&steps, &fields, None));
    assert_eq!(
        observed.0,
        Err(RenderError::UnknownLedgerField("stored".into()))
    );
    assert_eq!((observed.1, observed.2), (7, 10));
    one(&trace, Owner::ConstructorStep(&steps[1]), Slot::Node);
}

#[test]
fn valid_all_ten_variants_keep_exact_output_and_zero_refusals() {
    for (target, step) in cases("stored") {
        let fields = HashMap::from([("stored", &target)]);
        let trace = Trace::default();
        let steps = std::slice::from_ref(&step);
        let observed = run(steps, &fields, Some(&trace));
        assert!(observed.0.is_ok());
        assert_eq!(observed, run(steps, &fields, None));
        assert_eq!((observed.1, observed.2), (7, 9));
        assert!(trace.0.borrow().is_empty());
    }
}

#[test]
fn equal_live_step_and_key_occurrences_are_not_collapsed_by_value_equality() {
    let first = ConstructorStep::MapRemove {
        field: "absent".into(),
        index: 0,
        key: field("1"),
    };
    let steps = [first.clone(), first];
    let mut ids = Vec::new();
    for step in &steps {
        let trace = Trace::default();
        error(
            step,
            &HashMap::new(),
            &trace,
            RenderError::UnknownLedgerField("absent".into()),
        );
        one(&trace, Owner::ConstructorStep(step), Slot::Field("field"));
        ids.push(key(Owner::ConstructorStep(step), Slot::Field("field")));
    }
    assert_ne!(ids[0], ids[1]);
    let first = ConstructorStep::MapRemove {
        field: "stored".into(),
        index: 0,
        key: Expr::Boolean { value: true },
    };
    let steps = [first.clone(), first];
    let mut ids = Vec::new();
    let target = ledger("stored", map_kind());
    for step in &steps {
        let ConstructorStep::MapRemove {
            key: original_key, ..
        } = step
        else {
            panic!("fixture")
        };
        let trace = Trace::default();
        error(
            step,
            &HashMap::from([("stored", &target)]),
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
