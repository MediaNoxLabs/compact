//! Real borrowed formal checker producers. Public native errors still run first.
use super::*;
use crate::{
    RenderError,
    ir::{Parameter, Type},
    source_diagnostics::{Owner, RefusalObserver, RefusalSite, Slot},
};
use std::cell::RefCell;

#[derive(Default)]
struct Trace(RefCell<Vec<(&'static str, usize, Slot)>>);
impl RefusalObserver for Trace {
    fn refused(&self, site: RefusalSite<'_>) {
        let Some(owner) = site.owner else {
            return;
        };
        let identity = match owner {
            Owner::Expr(source) => ("expr", std::ptr::from_ref(source) as usize),
            Owner::Type(source) => ("type", std::ptr::from_ref(source) as usize),
            Owner::Pure(source) => ("pure", std::ptr::from_ref(source) as usize),
            Owner::Parameter(source) => ("parameter", std::ptr::from_ref(source) as usize),
            _ => panic!("unexpected formal source owner"),
        };
        self.0
            .borrow_mut()
            .push((identity.0, identity.1, site.slot));
    }
}
fn pure(body: Expr, result: Type) -> PureCircuit {
    PureCircuit {
        source: None,
        name: "helper".into(),
        internal: true,
        parameters: vec![],
        result,
        body,
    }
}

#[test]
fn formal_unit_checker_captures_original_first_child_before_string_conversion() {
    let source = pure(
        Expr::Add {
            left: Box::new(Expr::Parameter {
                name: "missing_first".into(),
            }),
            right: Box::new(Expr::Parameter {
                name: "missing_second".into(),
            }),
        },
        Type::Field,
    );
    let Expr::Add { left, .. } = &source.body else {
        panic!("fixture")
    };
    let expected = checker::check(&source, &HashMap::new()).unwrap_err();
    let trace = Trace::default();
    let actual = checker::check_observed(&source, &HashMap::new(), Some(&trace)).unwrap_err();
    assert_eq!(actual, expected);
    assert!(matches!(actual,RenderError::UnknownParameter(ref name) if name=="missing_first"));
    assert_eq!(
        trace.0.borrow().first(),
        Some(&(
            "expr",
            std::ptr::from_ref(left.as_ref()) as usize,
            Slot::Node
        ))
    );
    assert_eq!(
        trace.0.borrow().last(),
        Some(&(
            "expr",
            std::ptr::from_ref(&source.body) as usize,
            Slot::Node
        ))
    );
}

#[test]
fn certifier_default_guard_uses_original_type_and_does_not_clone_failed_source() {
    let ty = Type::Tuple {
        elements: vec![Type::Field; 13],
    };
    let source = pure(Expr::Default { ty: ty.clone() }, ty);
    let Expr::Default { ty: target } = &source.body else {
        panic!("fixture")
    };
    let sources = HashMap::from([("helper", &source)]);
    let stateful = HashMap::new();
    let mut plain = Certifier::new(&sources, &stateful);
    let expected = plain.certify("helper", "return_value.value").unwrap_err();
    let mut observed = Certifier::new(&sources, &stateful);
    let trace = Trace::default();
    let actual = observed
        .certify_observed("helper", "return_value.value", Some(&trace))
        .unwrap_err();
    assert_eq!(actual, expected);
    assert!(actual.detail.contains("pure Default requires tuple arity"));
    assert_eq!(
        trace.0.borrow().first(),
        Some(&("type", std::ptr::from_ref(target) as usize, Slot::Node))
    );
    assert!(plain.get("helper").is_none());
    assert!(
        observed.get("helper").is_none(),
        "failed source never becomes a cloned certificate"
    );
}

#[test]
fn formal_name_refusal_keeps_parameter_slot_and_original_error_precedence() {
    let mut source = pure(
        Expr::Parameter {
            name: "missing".into(),
        },
        Type::Field,
    );
    source.parameters.push(Parameter {
        name: "crate".into(),
        ty: Type::Field,
    });
    let expected = checker::check(&source, &HashMap::new()).unwrap_err();
    let trace = Trace::default();
    let actual = checker::check_observed(&source, &HashMap::new(), Some(&trace)).unwrap_err();
    assert_eq!(actual, expected);
    assert!(matches!(actual,RenderError::InvalidIdentifier(ref name) if name=="crate"));
    assert_eq!(
        trace.0.borrow().first(),
        Some(&(
            "parameter",
            std::ptr::from_ref(&source.parameters[0]) as usize,
            Slot::Field("name")
        ))
    );
}

#[test]
fn cached_success_is_not_rechecked_or_given_a_phantom_refusal_site() {
    let source = pure(Expr::FieldLiteral { value: "1".into() }, Type::Field);
    let sources = HashMap::from([("helper", &source)]);
    let stateful = HashMap::new();
    let trace = Trace::default();
    let mut certifier = Certifier::new(&sources, &stateful);
    certifier
        .certify_observed("helper", "one", Some(&trace))
        .unwrap();
    let before = std::ptr::from_ref(certifier.get("helper").unwrap());
    certifier
        .certify_observed("helper", "two", Some(&trace))
        .unwrap();
    assert_eq!(before, std::ptr::from_ref(certifier.get("helper").unwrap()));
    assert!(trace.0.borrow().is_empty());
}
