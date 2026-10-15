// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Direct VM assembly controls: actual original guards, not frontend ranges.
//! Existing recursion, carrier folds and caller-owned counters remain in order.
use super::*;

type GuardEvent = (&'static str, usize, Slot);
type AssemblyOutcome = (Result<String, RenderError>, usize, usize);

#[derive(Default)]
struct GuardTrace(RefCell<Vec<GuardEvent>>);
impl RefusalObserver for GuardTrace {
    fn refused(&self, site: RefusalSite<'_>) {
        let (kind, pointer) = match site.owner.expect("original VM guard owner") {
            Owner::Expr(value) => ("expr", std::ptr::from_ref(value) as usize),
            Owner::Binding(value) => ("binding", std::ptr::from_ref(value) as usize),
            Owner::Type(value) => ("type", std::ptr::from_ref(value) as usize),
            _ => panic!("unexpected lexical/control guard owner"),
        };
        self.0.borrow_mut().push((kind, pointer, site.slot));
    }
}
fn boolean(value: bool) -> Expr {
    Expr::Boolean { value }
}
fn binding(ty: Type) -> Parameter {
    Parameter {
        name: "item".into(),
        ty,
    }
}
fn empty() -> ConstructorStep {
    ConstructorStep::Sequence { steps: vec![] }
}
fn missing_step(name: &str) -> ConstructorStep {
    ConstructorStep::Expression {
        value: parameter(name),
    }
}
fn only(trace: &GuardTrace, kind: &'static str, pointer: usize, slot: Slot) {
    assert_eq!(trace.0.borrow().as_slice(), &[(kind, pointer, slot)]);
}
fn only_expr(trace: &GuardTrace, value: &Expr) {
    only(
        trace,
        "expr",
        std::ptr::from_ref(value) as usize,
        Slot::Node,
    );
}
// Keep the caller-owned state even on Err; no mirrored VM guard logic.
fn execute<'a>(
    steps: &'a [ConstructorStep],
    parameters: &HashMap<&'a str, (&'a Type, syn::Ident)>,
    observer: Option<&dyn RefusalObserver>,
) -> AssemblyOutcome {
    let fields = HashMap::new();
    let witnesses = HashMap::new();
    let circuits = HashMap::new();
    let stateful = HashMap::new();
    let environment = stateful::NativeExpressionContext {
        witnesses: &witnesses,
        circuits: &circuits,
        stateful_circuits: &stateful,
        ledger_fields: &fields,
        observer,
    };
    let mut next_loop = 7;
    let mut next_temp = 9;
    let result = if observer.is_some() {
        render_constructor_vm_steps_observed(
            steps,
            parameters,
            &environment,
            super::super::Output {
                next_loop: &mut next_loop,
                next_temp: &mut next_temp,
            },
        )
    } else {
        render_constructor_vm_steps(
            steps,
            &fields,
            parameters,
            &witnesses,
            &circuits,
            &stateful,
            &mut next_loop,
            &mut next_temp,
        )
    }
    .map(|statements| quote::quote!(#(#statements)*).to_string());
    (result, next_loop, next_temp)
}
fn error<'a>(
    step: &'a ConstructorStep,
    parameters: &HashMap<&'a str, (&'a Type, syn::Ident)>,
    trace: &GuardTrace,
    expected: RenderError,
) -> (usize, usize) {
    let bare = execute(std::slice::from_ref(step), parameters, None);
    let observed = execute(std::slice::from_ref(step), parameters, Some(trace));
    assert_eq!(observed, bare, "same typed error and partial counter state");
    assert_eq!(observed.0, Err(expected));
    (observed.1, observed.2)
}

#[test]
fn lexical_type_guard_keeps_original_value_after_name_and_before_next_binding() {
    let step = ConstructorStep::Let {
        bindings: vec![
            LocalBinding {
                name: "local".into(),
                ty: Type::Field,
                value: boolean(true),
            },
            LocalBinding {
                name: "crate".into(),
                ty: Type::Field,
                value: parameter("later"),
            },
        ],
        step: Box::new(missing_step("body")),
    };
    let ConstructorStep::Let { bindings, .. } = &step else {
        panic!("fixture")
    };
    let trace = GuardTrace::default();
    assert_eq!(
        error(
            &step,
            &HashMap::new(),
            &trace,
            RenderError::TypeMismatch {
                expected: Type::Field,
                actual: Type::Boolean
            }
        ),
        (7, 9)
    );
    only_expr(&trace, &bindings[0].value);
    let name_first = ConstructorStep::Let {
        bindings: vec![LocalBinding {
            name: "crate".into(),
            ty: Type::Field,
            value: parameter("not_reached"),
        }],
        step: Box::new(empty()),
    };
    let ConstructorStep::Let { bindings, .. } = &name_first else {
        panic!("fixture")
    };
    let trace = GuardTrace::default();
    error(
        &name_first,
        &HashMap::new(),
        &trace,
        RenderError::InvalidIdentifier("crate".into()),
    );
    only(
        &trace,
        "binding",
        std::ptr::from_ref(&bindings[0]) as usize,
        Slot::Field("name"),
    );
}

#[test]
fn assertion_type_guard_keeps_original_condition_but_child_refusal_wins() {
    for (condition, expected) in [
        (
            field("1"),
            RenderError::TypeMismatch {
                expected: Type::Boolean,
                actual: Type::Field,
            },
        ),
        (field("bad"), RenderError::InvalidFieldLiteral("bad".into())),
    ] {
        let step = ConstructorStep::Assert {
            condition,
            message: "runtime message".into(),
        };
        let ConstructorStep::Assert { condition, .. } = &step else {
            panic!("fixture")
        };
        let trace = GuardTrace::default();
        assert_eq!(error(&step, &HashMap::new(), &trace, expected), (7, 9));
        only_expr(&trace, condition);
    }
}

#[test]
fn foreach_infallibility_then_value_type_precedes_later_values_and_body() {
    let rejected = Expr::Add {
        left: Box::new(field("bad")),
        right: Box::new(parameter("later")),
    };
    for (value, expected) in [
        (
            rejected,
            RenderError::InvalidConstructorInitializer("item".into()),
        ),
        (
            boolean(true),
            RenderError::TypeMismatch {
                expected: Type::Field,
                actual: Type::Boolean,
            },
        ),
        (field("bad"), RenderError::InvalidFieldLiteral("bad".into())),
    ] {
        let step = ConstructorStep::ForEach {
            binding: binding(Type::Field),
            values: vec![value, parameter("later_value")],
            steps: vec![missing_step("body")],
        };
        let ConstructorStep::ForEach { values, .. } = &step else {
            panic!("fixture")
        };
        let trace = GuardTrace::default();
        assert_eq!(error(&step, &HashMap::new(), &trace, expected), (8, 9));
        only_expr(&trace, &values[0]);
    }
}

#[test]
fn vector_source_shape_lookup_and_type_keep_actual_source_without_new_fold() {
    let wrong = Type::Boolean;
    let parameters = HashMap::from([("values", (&wrong, syn::parse_quote!(values)))]);
    for (source, expected) in [
        (
            field("bad"),
            RenderError::InvalidConstructorInitializer("item".into()),
        ),
        (
            parameter("absent"),
            RenderError::UnknownParameter("absent".into()),
        ),
        (
            parameter("values"),
            RenderError::TypeMismatch {
                expected: Type::Vector {
                    element: Box::new(Type::Field),
                    length: 2,
                },
                actual: Type::Boolean,
            },
        ),
    ] {
        let step = ConstructorStep::ForEachVector {
            binding: binding(Type::Field),
            source,
            length: 2,
            steps: vec![missing_step("body")],
        };
        let ConstructorStep::ForEachVector { source, .. } = &step else {
            panic!("fixture")
        };
        let trace = GuardTrace::default();
        assert_eq!(error(&step, &parameters, &trace, expected), (8, 9));
        only_expr(&trace, source);
    }
}

#[test]
fn conditional_guard_preserves_infallibility_then_type_then_branch_order() {
    for (condition, expected) in [
        (
            Expr::Add {
                left: Box::new(field("bad")),
                right: Box::new(parameter("not_reached")),
            },
            RenderError::InvalidConstructorInitializer("if".into()),
        ),
        (
            field("1"),
            RenderError::TypeMismatch {
                expected: Type::Boolean,
                actual: Type::Field,
            },
        ),
    ] {
        let step = ConstructorStep::If {
            condition,
            then_steps: vec![missing_step("then_missing")],
            otherwise_steps: vec![missing_step("else_missing")],
        };
        let ConstructorStep::If { condition, .. } = &step else {
            panic!("fixture")
        };
        let trace = GuardTrace::default();
        assert_eq!(error(&step, &HashMap::new(), &trace, expected), (7, 9));
        only_expr(&trace, condition);
    }
    let step = ConstructorStep::If {
        condition: boolean(true),
        then_steps: vec![missing_step("then_missing")],
        otherwise_steps: vec![missing_step("else_missing")],
    };
    let ConstructorStep::If { then_steps, .. } = &step else {
        panic!("fixture")
    };
    let ConstructorStep::Expression { value } = &then_steps[0] else {
        panic!("fixture")
    };
    let trace = GuardTrace::default();
    error(
        &step,
        &HashMap::new(),
        &trace,
        RenderError::UnknownParameter("then_missing".into()),
    );
    only_expr(&trace, value);
}

#[test]
fn existing_carrier_calls_keep_original_type_and_their_late_order() {
    let unsupported = Type::LedgerMap {
        key: Box::new(Type::Field),
        value: Box::new(Type::Field),
    };
    let parameters = HashMap::from([("input", (&unsupported, syn::parse_quote!(input)))]);
    let step = ConstructorStep::Let {
        bindings: vec![LocalBinding {
            name: "local".into(),
            ty: unsupported.clone(),
            value: parameter("input"),
        }],
        step: Box::new(missing_step("later")),
    };
    let ConstructorStep::Let { bindings, .. } = &step else {
        panic!("fixture")
    };
    let trace = GuardTrace::default();
    assert_eq!(
        error(
            &step,
            &parameters,
            &trace,
            RenderError::UnsupportedLedgerValueType(unsupported.clone())
        ),
        (7, 10)
    );
    only(
        &trace,
        "type",
        std::ptr::from_ref(&bindings[0].ty) as usize,
        Slot::Node,
    );
    let step = ConstructorStep::ForEach {
        binding: binding(unsupported.clone()),
        values: vec![],
        steps: vec![],
    };
    let ConstructorStep::ForEach { binding, .. } = &step else {
        panic!("fixture")
    };
    let trace = GuardTrace::default();
    assert_eq!(
        error(
            &step,
            &HashMap::new(),
            &trace,
            RenderError::UnsupportedLedgerValueType(unsupported.clone())
        ),
        (8, 9)
    );
    only(
        &trace,
        "type",
        std::ptr::from_ref(&binding.ty) as usize,
        Slot::Node,
    );
    let step = ConstructorStep::ForEach {
        binding: Parameter {
            name: "item".into(),
            ty: unsupported,
        },
        values: vec![],
        steps: vec![missing_step("body_first")],
    };
    let ConstructorStep::ForEach { steps, .. } = &step else {
        panic!("fixture")
    };
    let ConstructorStep::Expression { value } = &steps[0] else {
        panic!("fixture")
    };
    let trace = GuardTrace::default();
    assert_eq!(
        error(
            &step,
            &HashMap::new(),
            &trace,
            RenderError::UnknownParameter("body_first".into())
        ),
        (8, 9)
    );
    only_expr(&trace, value);
}

#[test]
fn successful_guard_paths_keep_syntax_counters_and_no_extra_binding_name_check() {
    let vector = Type::Vector {
        element: Box::new(Type::Field),
        length: 1,
    };
    let parameters = HashMap::from([("values", (&vector, syn::parse_quote!(values)))]);
    let steps = vec![
        ConstructorStep::Let {
            bindings: vec![LocalBinding {
                name: "local".into(),
                ty: Type::Field,
                value: field("1"),
            }],
            step: Box::new(empty()),
        },
        ConstructorStep::Assert {
            condition: boolean(true),
            message: "ok".into(),
        },
        ConstructorStep::ForEach {
            binding: Parameter {
                name: "crate".into(),
                ty: Type::Field,
            },
            values: vec![field("1")],
            steps: vec![ConstructorStep::Expression {
                value: parameter("crate"),
            }],
        },
        ConstructorStep::ForEachVector {
            binding: Parameter {
                name: "crate".into(),
                ty: Type::Field,
            },
            source: parameter("values"),
            length: 1,
            steps: vec![ConstructorStep::Expression {
                value: parameter("crate"),
            }],
        },
        ConstructorStep::If {
            condition: boolean(true),
            then_steps: vec![],
            otherwise_steps: vec![],
        },
    ];
    let trace = GuardTrace::default();
    let bare = run(&steps, &HashMap::new(), &parameters, &HashMap::new(), None).unwrap();
    let observed = run(
        &steps,
        &HashMap::new(),
        &parameters,
        &HashMap::new(),
        Some(&trace),
    )
    .unwrap();
    assert_eq!(observed, bare);
    assert_eq!((observed.1, observed.2), (9, 10));
    assert!(trace.0.borrow().is_empty());
    assert!(
        observed.0.contains("AssertionFailed"),
        "runtime assertion emission unchanged"
    );
    let original = ConstructorStep::Assert {
        condition: field("1"),
        message: "same".into(),
    };
    let equal_steps = [original.clone(), original];
    let mut identities = Vec::new();
    for step in &equal_steps {
        let ConstructorStep::Assert { condition, .. } = step else {
            panic!("fixture")
        };
        let trace = GuardTrace::default();
        error(
            step,
            &HashMap::new(),
            &trace,
            RenderError::TypeMismatch {
                expected: Type::Boolean,
                actual: Type::Field,
            },
        );
        only_expr(&trace, condition);
        identities.push(std::ptr::from_ref(condition) as usize);
    }
    assert_ne!(identities[0], identities[1]);
}
