//! Original dependency occurrences retained by the existing certifier DFS.
use super::*;
use crate::ir::Type;
use crate::source_diagnostics::Slot;
use std::cell::RefCell;

#[derive(Default)]
struct Trace {
    refused: RefCell<Vec<(&'static str, usize, Slot)>>,
    related: RefCell<Vec<usize>>,
}
impl RefusalObserver for Trace {
    fn refused(&self, site: RefusalSite<'_>) {
        let (category, address) = match site.owner.expect("original owner") {
            Owner::Expr(source) => ("expr", std::ptr::from_ref(source) as usize),
            Owner::Type(source) => ("type", std::ptr::from_ref(source) as usize),
            Owner::Pure(source) => ("pure", std::ptr::from_ref(source) as usize),
            _ => panic!("unexpected source"),
        };
        self.refused
            .borrow_mut()
            .push((category, address, site.slot));
    }
    fn related(&self, site: RefusalSite<'_>) {
        let Some(Owner::Expr(source)) = site.owner else {
            panic!("actual call expression")
        };
        assert_eq!(site.slot, Slot::Node);
        self.related
            .borrow_mut()
            .push(std::ptr::from_ref(source) as usize);
    }
}
fn call(name: &str) -> Expr {
    Expr::Call {
        name: name.into(),
        arguments: vec![],
    }
}
fn source(name: &str, body: Expr) -> PureCircuit {
    PureCircuit {
        source: None,
        name: name.into(),
        internal: true,
        parameters: vec![],
        result: Type::Field,
        body,
    }
}
fn ptr(source: &Expr) -> usize {
    std::ptr::from_ref(source) as usize
}
fn same_error(sources: &[PureCircuit], name: &str, trace: &Trace) -> RecordingGap {
    let pure = sources
        .iter()
        .map(|source| (source.name.as_str(), source))
        .collect();
    let stateful = HashMap::new();
    let expected = Certifier::new(&pure, &stateful)
        .certify(name, "root.call")
        .unwrap_err();
    let actual = Certifier::new(&pure, &stateful)
        .certify_observed(name, "root.call", Some(trace))
        .unwrap_err();
    assert_eq!(actual, expected);
    actual
}
fn bad_default() -> PureCircuit {
    let ty = Type::Tuple {
        elements: vec![Type::Field; 13],
    };
    let mut bad = source("bad", Expr::Default { ty: ty.clone() });
    bad.result = ty;
    bad
}

#[test]
fn transitive_callee_default_preserves_primary_and_inner_to_outer_actual_edges() {
    let sources = [
        source("root", call("mid")),
        source("mid", call("bad")),
        bad_default(),
    ];
    let Expr::Default { ty } = &sources[2].body else {
        panic!("fixture")
    };
    let trace = Trace::default();
    let error = same_error(&sources, "root", &trace);
    assert!(error.detail.contains("pure Default requires tuple arity"));
    assert_eq!(
        trace.refused.borrow().first(),
        Some(&("type", std::ptr::from_ref(ty) as usize, Slot::Node))
    );
    assert_eq!(
        trace.related.borrow().as_slice(),
        &[ptr(&sources[1].body), ptr(&sources[0].body)]
    );
}

#[test]
fn transitive_body_failure_keeps_original_first_leaf_and_join_order() {
    let bad = source(
        "bad",
        Expr::Add {
            left: Box::new(Expr::Parameter {
                name: "first".into(),
            }),
            right: Box::new(Expr::Parameter {
                name: "second".into(),
            }),
        },
    );
    let sources = [source("root", call("mid")), source("mid", call("bad")), bad];
    let Expr::Add { left, .. } = &sources[2].body else {
        panic!("fixture")
    };
    let trace = Trace::default();
    let error = same_error(&sources, "root", &trace);
    assert!(error.detail.contains("first"));
    assert_eq!(
        trace.refused.borrow().first(),
        Some(&("expr", ptr(left), Slot::Node))
    );
    assert_eq!(
        trace.related.borrow().as_slice(),
        &[ptr(&sources[1].body), ptr(&sources[0].body)]
    );
}

#[test]
fn equal_distinct_calls_keep_two_dependencies_but_only_selected_failure_edge() {
    let root = source(
        "root",
        Expr::Add {
            left: Box::new(call("bad")),
            right: Box::new(call("bad")),
        },
    );
    let sources = [root, bad_default()];
    let Expr::Add { left, right } = &sources[0].body else {
        panic!("fixture")
    };
    assert_eq!(left, right);
    assert_ne!(ptr(left), ptr(right));
    let pure = sources
        .iter()
        .map(|source| (source.name.as_str(), source))
        .collect();
    let stateful = HashMap::new();
    let mut certifier = Certifier::new(&pure, &stateful);
    let frame = certifier.enter("root", "root.call", None).unwrap();
    assert_eq!(frame.dependencies.len(), 2);
    assert_eq!(ptr(frame.dependencies[0].source), ptr(left));
    assert_eq!(ptr(frame.dependencies[1].source), ptr(right));
    let trace = Trace::default();
    same_error(&sources, "root", &trace);
    assert_eq!(trace.related.borrow().as_slice(), &[ptr(left)]);
}

#[test]
fn cycle_guard_captures_actual_closing_call_and_retains_original_gap() {
    let sources = [source("root", call("mid")), source("mid", call("root"))];
    let trace = Trace::default();
    let error = same_error(&sources, "root", &trace);
    assert!(error.detail.contains("recursive pure helper"));
    assert_eq!(
        trace.refused.borrow().as_slice(),
        &[("expr", ptr(&sources[1].body), Slot::Field("name"))]
    );
    assert_eq!(trace.related.borrow().as_slice(), &[ptr(&sources[0].body)]);
}

#[test]
fn success_and_cached_reentry_emit_no_phantom_failure_or_related_edges() {
    let sources = [
        source("root", call("leaf")),
        source("leaf", Expr::FieldLiteral { value: "1".into() }),
    ];
    let pure = sources
        .iter()
        .map(|source| (source.name.as_str(), source))
        .collect();
    let stateful = HashMap::new();
    let mut certifier = Certifier::new(&pure, &stateful);
    let trace = Trace::default();
    certifier
        .certify_observed("root", "one", Some(&trace))
        .unwrap();
    let before = std::ptr::from_ref(certifier.get("root").unwrap());
    let work = certifier.get("root").unwrap().expanded_work();
    certifier
        .certify_observed("root", "two", Some(&trace))
        .unwrap();
    assert_eq!(before, std::ptr::from_ref(certifier.get("root").unwrap()));
    assert_eq!(work, certifier.get("root").unwrap().expanded_work());
    assert!(trace.refused.borrow().is_empty());
    assert!(trace.related.borrow().is_empty());
}

#[test]
fn certifier_dependency_guard_still_precedes_caller_body_argument_error() {
    // This is certifier order, not Lowerer::pure_call argument order (unchanged).
    let root = source(
        "root",
        Expr::Call {
            name: "bad".into(),
            arguments: vec![Expr::Parameter {
                name: "later_argument".into(),
            }],
        },
    );
    let sources = [root, bad_default()];
    let trace = Trace::default();
    let error = same_error(&sources, "root", &trace);
    assert!(error.detail.contains("pure Default requires tuple arity"));
    assert!(!error.detail.contains("later_argument"));
    assert_eq!(trace.related.borrow().as_slice(), &[ptr(&sources[0].body)]);
}

#[test]
fn missing_dependency_keeps_actual_incoming_name_and_exact_call_context() {
    let sources = [source("root", call("absent"))];
    let trace = Trace::default();
    let error = same_error(&sources, "root", &trace);
    assert!(error.detail.contains("unknown pure callee"));
    assert_eq!(
        trace.refused.borrow().as_slice(),
        &[("expr", ptr(&sources[0].body), Slot::Field("name"))]
    );
    assert_eq!(trace.related.borrow().as_slice(), &[ptr(&sources[0].body)]);
}

#[test]
fn existing_depth_limit_bounds_related_edges_without_new_admission_policy() {
    let (limit, _) = crate::resource_limits::recording_call_limits();
    let sources: Vec<_> = (0..=limit)
        .map(|index| {
            source(
                &format!("helper_{index}"),
                if index == limit {
                    Expr::FieldLiteral { value: "1".into() }
                } else {
                    call(&format!("helper_{}", index + 1))
                },
            )
        })
        .collect();
    let trace = Trace::default();
    let error = same_error(&sources, "helper_0", &trace);
    assert!(
        error
            .detail
            .contains("pure helper call depth limit exceeded")
    );
    assert_eq!(
        trace.refused.borrow().as_slice(),
        &[("expr", ptr(&sources[limit - 1].body), Slot::Field("name"))]
    );
    assert_eq!(trace.related.borrow().len(), limit - 1);
    for (actual, source) in trace
        .related
        .borrow()
        .iter()
        .zip(sources[..limit - 1].iter().rev())
    {
        assert_eq!(*actual, ptr(&source.body));
    }
}
