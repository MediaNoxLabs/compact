// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Actual full-render non-VM Cell route: target and outer type guards.
use super::*;
use crate::ir::SourceLocation;
fn original_step(source: &Contract) -> &ConstructorStep {
    &source.constructor.as_ref().unwrap().steps[0]
}
fn step_site(trace: &Trace, step: &ConstructorStep, slot: Slot) {
    assert_eq!(
        trace.0.borrow().as_slice(),
        &[(Some(std::ptr::from_ref(step) as usize), slot)]
    );
}

#[test]
fn non_vm_target_guards_precede_invalid_child_and_preserve_constructor_location() {
    for case in 0..3 {
        let mut source = fixture(field("bad"));
        source.constructor.as_mut().unwrap().source = Some(SourceLocation {
            file: "constructor.compact".into(),
            line: 7,
            column: 2,
        });
        let expected = match case {
            0 => {
                let ConstructorStep::CellWrite { field, .. } =
                    &mut source.constructor.as_mut().unwrap().steps[0]
                else {
                    panic!("fixture")
                };
                *field = "absent".into();
                RenderError::UnknownLedgerField("absent".into())
            }
            1 => {
                let ConstructorStep::CellWrite { index, .. } =
                    &mut source.constructor.as_mut().unwrap().steps[0]
                else {
                    panic!("fixture")
                };
                *index = 1;
                RenderError::InvalidLedgerIndex(1)
            }
            _ => {
                source.ledger_fields[0].declaration = LedgerFieldKind::Counter;
                RenderError::InvalidConstructorInitializer("stored".into())
            }
        }
        .at(source.constructor.as_ref().unwrap().source.as_ref());
        // One CellWrite, no repeated target or stateful/witness call: old selector is non-VM.
        let trace = Trace::default();
        assert_eq!(same_error(&source, &trace), expected);
        step_site(
            &trace,
            original_step(&source),
            if case == 1 {
                Slot::Field("index")
            } else {
                Slot::Field("field")
            },
        );
    }
}

#[test]
fn non_vm_outer_type_keeps_actual_value_but_nested_failed_child_stays_first() {
    let source = fixture(Expr::Boolean { value: true });
    let trace = Trace::default();
    assert_eq!(
        same_error(&source, &trace),
        RenderError::TypeMismatch {
            expected: Type::Field,
            actual: Type::Boolean
        }
    );
    first_is(&trace, value(&source));
    assert_eq!(trace.0.borrow().len(), 1);
    let source = fixture(Expr::Add {
        left: Box::new(field("bad")),
        right: Box::new(Expr::Parameter {
            name: "later".into(),
        }),
    });
    let Expr::Add { left, right } = value(&source) else {
        panic!("fixture")
    };
    let trace = Trace::default();
    assert_eq!(
        same_error(&source, &trace),
        RenderError::InvalidFieldLiteral("bad".into())
    );
    first_is(&trace, left);
    assert!(trace.0.borrow().iter().all(|(pointer, _)| *pointer
        != Some(std::ptr::from_ref(right.as_ref()) as usize)
        && *pointer != Some(std::ptr::from_ref(original_step(&source)) as usize)));
}

#[test]
fn later_non_vm_initializer_guard_follows_earlier_success_without_parent_guessing() {
    let mut source = fixture(field("1"));
    source
        .constructor
        .as_mut()
        .unwrap()
        .steps
        .push(ConstructorStep::CellWrite {
            field: "absent".into(),
            index: 0,
            value: field("bad"),
        });
    let trace = Trace::default();
    assert_eq!(
        same_error(&source, &trace),
        RenderError::UnknownLedgerField("absent".into())
    );
    step_site(
        &trace,
        &source.constructor.as_ref().unwrap().steps[1],
        Slot::Field("field"),
    );
    // Different targets retain non-VM selection. No returned partial output is claimed on Err.
}

#[test]
fn non_vm_prepared_output_and_equal_distinct_initializers_retain_original_behavior() {
    let source = fixture(field("1"));
    let trace = Trace::default();
    let bare = crate::render_with_native_refusal(&source, None, None).unwrap();
    let observed = crate::render_with_native_refusal(&source, None, Some(&trace)).unwrap();
    assert_eq!(observed.source, bare.source);
    assert_eq!(
        serde_json::to_value(observed.capabilities).unwrap(),
        serde_json::to_value(bare.capabilities).unwrap()
    );
    assert!(
        observed.source.contains("__compact_constructor_value_0"),
        "existing non-VM preparation path"
    );
    assert!(trace.0.borrow().is_empty());
    let original = fixture(Expr::Boolean { value: true });
    let sources = [original.clone(), original];
    let mut identities = Vec::new();
    for source in &sources {
        let trace = Trace::default();
        assert_eq!(
            same_error(source, &trace),
            RenderError::TypeMismatch {
                expected: Type::Field,
                actual: Type::Boolean
            }
        );
        first_is(&trace, value(source));
        assert_eq!(trace.0.borrow().len(), 1);
        identities.push(std::ptr::from_ref(value(source)) as usize);
    }
    assert_ne!(identities[0], identities[1]);
}
