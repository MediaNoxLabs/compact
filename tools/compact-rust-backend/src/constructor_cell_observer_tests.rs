// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Non-VM Cell constructor values: original borrowed occurrences and one fold.
//! These are direct-IR source/assembly controls, not frontend or VM coverage.
use super::*;
use crate::ir::{Constructor, LedgerField, Parameter};
use crate::source_diagnostics::Slot;
use std::cell::RefCell;

mod outer_guards;
mod root_layout;

#[derive(Default)]
struct Trace(RefCell<Vec<(Option<usize>, Slot)>>);
impl RefusalObserver for Trace {
    fn refused(&self, site: RefusalSite<'_>) {
        let pointer = match site.owner {
            Some(Owner::Expr(value)) => Some(std::ptr::from_ref(value) as usize),
            Some(Owner::ConstructorStep(value)) => Some(std::ptr::from_ref(value) as usize),
            _ => None,
        };
        self.0.borrow_mut().push((pointer, site.slot));
    }
}

fn field(value: &str) -> Expr {
    Expr::FieldLiteral {
        value: value.into(),
    }
}

fn fixture(value: Expr) -> Contract {
    Contract {
        schema_version: SCHEMA_VERSION,
        type_aliases: vec![],
        witnesses: vec![],
        circuits: vec![],
        stateful_circuits: vec![],
        ledger_fields: vec![LedgerField {
            source: None,
            id: "stored".into(),
            index: 0,
            path: vec![0],
            declaration: LedgerFieldKind::Cell { ty: Type::Field },
        }],
        constructor: Some(Constructor {
            source: None,
            parameters: vec![],
            steps: vec![ConstructorStep::CellWrite {
                field: "stored".into(),
                index: 0,
                value,
            }],
        }),
    }
}

fn value(source: &Contract) -> &Expr {
    let ConstructorStep::CellWrite { value, .. } = &source
        .constructor
        .as_ref()
        .expect("fixture constructor")
        .steps[0]
    else {
        panic!("fixture CellWrite")
    };
    value
}

fn refusal(source: &Contract, observer: Option<&dyn RefusalObserver>) -> RenderError {
    crate::render_with_native_refusal(source, None, observer)
        .err()
        .expect("expected constructor refusal")
}

fn same_error(source: &Contract, trace: &Trace) -> RenderError {
    let original = refusal(source, None);
    let observed = refusal(source, Some(trace));
    assert_eq!(observed, original);
    observed
}

fn first_is(trace: &Trace, original: &Expr) {
    assert_eq!(
        trace.0.borrow().first(),
        Some(&(Some(std::ptr::from_ref(original) as usize), Slot::Node))
    );
}

#[test]
fn constructor_nested_parameter_refusal_keeps_first_original_occurrence() {
    let source = fixture(Expr::Add {
        left: Box::new(Expr::Parameter {
            name: "first".into(),
        }),
        right: Box::new(Expr::Parameter {
            name: "later".into(),
        }),
    });
    let Expr::Add { left, right } = value(&source) else {
        panic!("fixture Add")
    };
    let trace = Trace::default();
    assert_eq!(
        same_error(&source, &trace),
        RenderError::UnknownParameter("first".into())
    );
    first_is(&trace, left);
    assert!(
        trace
            .0
            .borrow()
            .iter()
            .all(|(pointer, _)| *pointer != Some(std::ptr::from_ref(right.as_ref()) as usize))
    );
}

#[test]
fn constructor_literal_failure_precedes_later_parameter_without_error_replacement() {
    let source = fixture(Expr::Add {
        left: Box::new(field("bad")),
        right: Box::new(Expr::Parameter {
            name: "later".into(),
        }),
    });
    let Expr::Add { left, right } = value(&source) else {
        panic!("fixture Add")
    };
    let trace = Trace::default();
    assert_eq!(
        same_error(&source, &trace),
        RenderError::InvalidFieldLiteral("bad".into())
    );
    first_is(&trace, left);
    assert!(
        trace
            .0
            .borrow()
            .iter()
            .all(|(pointer, _)| *pointer != Some(std::ptr::from_ref(right.as_ref()) as usize))
    );
}

#[test]
fn constructor_declaration_index_and_domain_guards_still_precede_value_fold() {
    for case in 0..3 {
        let mut source = fixture(Expr::Parameter {
            name: "not_reached".into(),
        });
        let ConstructorStep::CellWrite { field, index, .. } =
            &mut source.constructor.as_mut().unwrap().steps[0]
        else {
            panic!("fixture CellWrite")
        };
        let expected = match case {
            0 => {
                *field = "absent".into();
                RenderError::UnknownLedgerField("absent".into())
            }
            1 => {
                *index = 1;
                RenderError::InvalidLedgerIndex(1)
            }
            _ => {
                source.ledger_fields[0].declaration = LedgerFieldKind::Counter;
                RenderError::InvalidConstructorInitializer("stored".into())
            }
        };
        let trace = Trace::default();
        assert_eq!(same_error(&source, &trace), expected);
        let step = &source.constructor.as_ref().unwrap().steps[0];
        let slot = if case == 1 {
            Slot::Field("index")
        } else {
            Slot::Field("field")
        };
        assert_eq!(
            trace.0.borrow().as_slice(),
            &[(Some(std::ptr::from_ref(step) as usize), slot)],
            "target guard still precedes value fold"
        );
    }
}

#[test]
fn constructor_outer_type_mismatch_keeps_original_initializer_occurrence() {
    let source = fixture(Expr::Boolean { value: true });
    let trace = Trace::default();
    assert_eq!(
        same_error(&source, &trace),
        RenderError::TypeMismatch {
            expected: Type::Field,
            actual: Type::Boolean,
        }
    );
    first_is(&trace, value(&source));
    assert_eq!(
        trace.0.borrow().len(),
        1,
        "actual outer type guard after successful child fold"
    );
}

#[test]
fn constructor_unused_branch_remains_checked_and_reports_original_child() {
    let source = fixture(Expr::If {
        condition: Box::new(Expr::Boolean { value: true }),
        then: Box::new(field("1")),
        otherwise: Box::new(Expr::Parameter {
            name: "dead".into(),
        }),
    });
    let Expr::If { otherwise, .. } = value(&source) else {
        panic!("fixture If")
    };
    let trace = Trace::default();
    assert_eq!(
        same_error(&source, &trace),
        RenderError::UnknownParameter("dead".into())
    );
    first_is(&trace, otherwise);
}

#[test]
fn constructor_valid_parameter_and_pure_call_keep_complete_output_and_capabilities() {
    let mut source = fixture(Expr::Call {
        name: "identity".into(),
        arguments: vec![Expr::Parameter {
            name: "incoming".into(),
        }],
    });
    source
        .constructor
        .as_mut()
        .unwrap()
        .parameters
        .push(Parameter {
            name: "incoming".into(),
            ty: Type::Field,
        });
    source.circuits.push(PureCircuit {
        source: None,
        name: "identity".into(),
        internal: false,
        parameters: vec![Parameter {
            name: "value".into(),
            ty: Type::Field,
        }],
        result: Type::Field,
        body: Expr::Parameter {
            name: "value".into(),
        },
    });
    let trace = Trace::default();
    let observed = crate::render_with_native_refusal(&source, None, Some(&trace)).unwrap();
    let bare = crate::render_with_capabilities(&source).unwrap();
    assert_eq!(observed.source, bare.source);
    assert_eq!(
        serde_json::to_value(&observed.capabilities).unwrap(),
        serde_json::to_value(&bare.capabilities).unwrap()
    );
    assert!(observed.source.contains("__compact_constructor_value_0"));
    assert!(trace.0.borrow().is_empty());
}

#[test]
fn constructor_pure_call_argument_refusal_keeps_actual_argument_not_callee_body() {
    let mut source = fixture(Expr::Call {
        name: "identity".into(),
        arguments: vec![Expr::Parameter {
            name: "missing_argument".into(),
        }],
    });
    source.circuits.push(PureCircuit {
        source: None,
        name: "identity".into(),
        internal: false,
        parameters: vec![Parameter {
            name: "value".into(),
            ty: Type::Field,
        }],
        result: Type::Field,
        body: Expr::Parameter {
            name: "value".into(),
        },
    });
    let Expr::Call { arguments, .. } = value(&source) else {
        panic!("fixture Call")
    };
    let trace = Trace::default();
    assert_eq!(
        same_error(&source, &trace),
        RenderError::UnknownParameter("missing_argument".into())
    );
    first_is(&trace, &arguments[0]);
}

#[test]
fn full_constructor_cell_observer_keeps_actual_default_two_mib_boundary() {
    fn nested(depth: usize) -> Contract {
        let mut body = field("1");
        for _ in 0..depth {
            body = Expr::Add {
                left: Box::new(body),
                right: Box::new(field("1")),
            };
        }
        fixture(body)
    }
    let depth = (1..=64)
        .take_while(|depth| crate::resource_limits::validate(&nested(*depth)).is_ok())
        .last()
        .expect("small constructor Cell initializer accepted");
    assert!(
        (3..48).contains(&depth),
        "unexpected constructor DEFAULT depth {depth}"
    );
    std::thread::Builder::new().stack_size(2 * 1024 * 1024).spawn(move || {
        let near = nested(depth);
        let trace = Trace::default();
        let observed = crate::render_with_native_refusal(&near, None, Some(&trace)).unwrap();
        let bare = crate::render_with_capabilities(&near).unwrap();
        assert_eq!(observed.source, bare.source);
        assert_eq!(serde_json::to_value(&observed.capabilities).unwrap(),
                   serde_json::to_value(&bare.capabilities).unwrap());
        assert!(trace.0.borrow().is_empty());

        let over = nested(depth + 1);
        let trace = Trace::default();
        let actual = same_error(&over, &trace);
        assert!(matches!(actual, RenderError::ResourceLimit { .. }));

        let mut malformed = nested(depth);
        let ConstructorStep::CellWrite { value, .. } =
            &mut malformed.constructor.as_mut().unwrap().steps[0]
        else { panic!("fixture CellWrite") };
        let mut leaf = value;
        while let Expr::Add { left, .. } = leaf { leaf = left; }
        *leaf = field("bad");
        let pointer = std::ptr::from_ref(leaf) as usize;
        let trace = Trace::default();
        assert_eq!(same_error(&malformed, &trace), RenderError::InvalidFieldLiteral("bad".into()));
        assert_eq!(trace.0.borrow().first(), Some(&(Some(pointer), Slot::Node)));
        eprintln!("Full constructor Cell Some/None: DEFAULT depth {depth} accepted, {} refused, malformed original leaf captured on ordinary2MiB", depth + 1);
    }).unwrap().join().unwrap();
}
