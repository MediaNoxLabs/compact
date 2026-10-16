//! Actual borrowed source controls at recursive native dispatcher/fold seams.
//! Private IR cases; parent separately qualifies genuine source continuity.
use super::*;
use crate::ir::{Parameter, StructField};
use crate::source_diagnostics::Slot;
use std::cell::RefCell;

#[derive(Default)]
struct Trace(RefCell<Vec<usize>>);
impl RefusalObserver for Trace {
    fn refused(&self, site: RefusalSite<'_>) {
        assert_eq!(site.slot, Slot::Node);
        let Some(Owner::Expr(source)) = site.owner else {
            panic!("actual expression owner")
        };
        self.0
            .borrow_mut()
            .push(std::ptr::from_ref(source) as usize);
    }
}
#[derive(Debug, PartialEq, Eq)]
struct Output {
    syntax: String,
    ty: Type,
    witness: bool,
    query: bool,
    next: usize,
}
fn lower(
    source: &Expr,
    pure: &HashMap<&str, &PureCircuit>,
    observer: Option<&dyn RefusalObserver>,
) -> Result<Output, RenderError> {
    let witnesses = HashMap::new();
    let stateful = HashMap::new();
    let ledger = HashMap::new();
    let parameters = HashMap::new();
    let environment = StateExpressionContext {
        witnesses: &witnesses,
        circuits: pure,
        stateful_circuits: &stateful,
        ledger_fields: &ledger,
        observer,
    };
    let mut statements = Vec::new();
    let mut next = 0;
    let mut query = false;
    let (rendered, ty, witness) = crate::stateful::expression::render_state_expression_observed(
        source,
        &parameters,
        &mut statements,
        &mut next,
        &mut query,
        &environment,
    )?;
    Ok(Output {
        syntax: quote::quote!(#(#statements)* #rendered).to_string(),
        ty,
        witness,
        query,
        next,
    })
}
fn same_error(source: &Expr, pure: &HashMap<&str, &PureCircuit>, trace: &Trace) -> RenderError {
    let expected = lower(source, pure, None).unwrap_err();
    let actual = lower(source, pure, Some(trace)).unwrap_err();
    assert_eq!(actual, expected);
    actual
}
fn first(trace: &Trace, source: &Expr) {
    assert_eq!(
        trace.0.borrow().first(),
        Some(&(std::ptr::from_ref(source) as usize))
    );
}
fn field(value: &str) -> Expr {
    Expr::FieldLiteral {
        value: value.into(),
    }
}
fn missing(name: &str) -> Expr {
    Expr::Parameter { name: name.into() }
}
fn vector() -> Expr {
    Expr::Vector {
        element: Type::Field,
        elements: vec![field("1")],
    }
}
fn reducer() -> PureCircuit {
    PureCircuit {
        source: None,
        name: "reduce".into(),
        internal: true,
        parameters: vec![
            Parameter {
                name: "acc".into(),
                ty: Type::Field,
            },
            Parameter {
                name: "item".into(),
                ty: Type::Field,
            },
        ],
        result: Type::Field,
        body: Expr::Add {
            left: Box::new(missing("acc")),
            right: Box::new(missing("item")),
        },
    }
}

#[test]
fn recursive_tuple_keeps_first_original_shared_leaf_without_parent_replacement() {
    let source = Expr::Tuple {
        elements: vec![
            Expr::Add {
                left: Box::new(missing("first")),
                right: Box::new(missing("second")),
            },
            missing("later"),
        ],
    };
    let Expr::Tuple { elements } = &source else {
        panic!("fixture")
    };
    let Expr::Add { left, .. } = &elements[0] else {
        panic!("fixture")
    };
    let trace = Trace::default();
    assert!(
        matches!(same_error(&source, &HashMap::new(), &trace), RenderError::UnknownParameter(ref name) if name == "first")
    );
    first(&trace, left);
    assert!(
        !trace
            .0
            .borrow()
            .contains(&(std::ptr::from_ref(&source) as usize)),
        "no recursive parent catch-all"
    );
}

#[test]
fn aggregate_initializer_guard_keeps_actual_child_before_type_clone() {
    let source = Expr::Tuple {
        elements: vec![Expr::StructLiteral {
            ty: Type::Struct {
                name: "Record".into(),
                fields: vec![StructField {
                    name: "amount".into(),
                    ty: Type::Field,
                }],
            },
            fields: vec![Expr::Boolean { value: true }],
        }],
    };
    let Expr::Tuple { elements } = &source else {
        panic!("fixture")
    };
    let Expr::StructLiteral { fields, .. } = &elements[0] else {
        panic!("fixture")
    };
    let trace = Trace::default();
    assert!(matches!(
        same_error(&source, &HashMap::new(), &trace),
        RenderError::TypeMismatch {
            expected: Type::Field,
            actual: Type::Boolean
        }
    ));
    first(&trace, &fields[0]);
}

#[test]
fn projection_index_guard_captures_original_operator_after_child_success() {
    let source = Expr::TupleIndex {
        value: Box::new(Expr::Tuple {
            elements: vec![field("1")],
        }),
        index: 2,
    };
    let trace = Trace::default();
    assert_eq!(
        same_error(&source, &HashMap::new(), &trace),
        RenderError::InvalidTupleIndex(2)
    );
    first(&trace, &source);
    assert_eq!(trace.0.borrow().len(), 1);
}

#[test]
fn call_count_unknown_and_argument_guards_keep_source_not_callee_clones() {
    let callee = PureCircuit {
        source: None,
        name: "helper".into(),
        internal: true,
        parameters: vec![Parameter {
            name: "value".into(),
            ty: Type::Field,
        }],
        result: Type::Field,
        body: missing("value"),
    };
    let pure = HashMap::from([("helper", &callee)]);
    let count = Expr::Call {
        name: "helper".into(),
        arguments: vec![],
    };
    let trace = Trace::default();
    assert!(matches!(
        same_error(&count, &pure, &trace),
        RenderError::ArgumentCount {
            expected: 1,
            actual: 0,
            ..
        }
    ));
    first(&trace, &count);
    let unknown = Expr::Call {
        name: "absent".into(),
        arguments: vec![],
    };
    let trace = Trace::default();
    assert!(
        matches!(same_error(&unknown, &pure, &trace), RenderError::UnknownCircuit(ref name) if name == "absent")
    );
    first(&trace, &unknown);
    let mismatch = Expr::Call {
        name: "helper".into(),
        arguments: vec![Expr::Boolean { value: false }],
    };
    let Expr::Call { arguments, .. } = &mismatch else {
        panic!("fixture")
    };
    let trace = Trace::default();
    assert!(matches!(
        same_error(&mismatch, &pure, &trace),
        RenderError::TypeMismatch {
            expected: Type::Field,
            actual: Type::Boolean
        }
    ));
    first(&trace, &arguments[0]);
}

#[test]
fn map_body_uses_existing_shared_fold_child_refusal_with_actual_identity() {
    let source = Expr::VectorMap {
        parameter: Parameter {
            name: "item".into(),
            ty: Type::Field,
        },
        source: Box::new(vector()),
        body: Box::new(missing("missing_body")),
        result: Type::Field,
        length: 1,
    };
    let Expr::VectorMap { body, .. } = &source else {
        panic!("fixture")
    };
    let trace = Trace::default();
    assert!(
        matches!(same_error(&source, &HashMap::new(), &trace), RenderError::UnknownParameter(ref name) if name == "missing_body")
    );
    first(&trace, body);
}

#[test]
fn fold_initial_failure_precedes_source_after_signature_check_without_rerun() {
    let callee = reducer();
    let pure = HashMap::from([("reduce", &callee)]);
    let source = Expr::VectorFoldCall {
        name: "reduce".into(),
        initial: Box::new(missing("missing_initial")),
        source: Box::new(missing("missing_vector")),
        accumulator: Type::Field,
        element: Type::Field,
        length: 1,
    };
    let Expr::VectorFoldCall { initial, .. } = &source else {
        panic!("fixture")
    };
    let trace = Trace::default();
    assert!(
        matches!(same_error(&source, &pure, &trace), RenderError::UnknownParameter(ref name) if name == "missing_initial")
    );
    first(&trace, initial);
    let unknown = Expr::VectorFoldCall {
        name: "absent".into(),
        initial: Box::new(missing("missing_initial")),
        source: Box::new(missing("missing_vector")),
        accumulator: Type::Field,
        element: Type::Field,
        length: 1,
    };
    let trace = Trace::default();
    assert!(
        matches!(same_error(&unknown, &pure, &trace), RenderError::UnknownCircuit(ref name) if name == "absent")
    );
    first(&trace, &unknown);
}

#[test]
fn successful_aggregate_call_map_fold_keep_output_flags_and_temporary_counts() {
    let callee = reducer();
    let pure = HashMap::from([("reduce", &callee)]);
    let sources = [
        Expr::Tuple {
            elements: vec![field("1"), field("2")],
        },
        Expr::TupleIndex {
            value: Box::new(Expr::Tuple {
                elements: vec![field("1")],
            }),
            index: 0,
        },
        Expr::Call {
            name: "reduce".into(),
            arguments: vec![field("1"), field("2")],
        },
        Expr::VectorMap {
            parameter: Parameter {
                name: "item".into(),
                ty: Type::Field,
            },
            source: Box::new(vector()),
            body: Box::new(missing("item")),
            result: Type::Field,
            length: 1,
        },
        Expr::VectorFoldCall {
            name: "reduce".into(),
            initial: Box::new(field("0")),
            source: Box::new(vector()),
            accumulator: Type::Field,
            element: Type::Field,
            length: 1,
        },
    ];
    for source in sources {
        let trace = Trace::default();
        assert_eq!(
            lower(&source, &pure, Some(&trace)).unwrap(),
            lower(&source, &pure, None).unwrap()
        );
        assert!(trace.0.borrow().is_empty());
    }
}

#[test]
fn optional_observer_does_not_add_sites_to_unavailable_or_successful_calls() {
    let bad = missing("missing");
    let good = field("1");
    let trace = Trace::default();
    assert!(matches!(
        lower(&bad, &HashMap::new(), None),
        Err(RenderError::UnknownParameter(_))
    ));
    assert!(trace.0.borrow().is_empty());
    assert!(lower(&bad, &HashMap::new(), Some(&trace)).is_err());
    let before = trace.0.borrow().clone();
    assert!(lower(&good, &HashMap::new(), Some(&trace)).is_ok());
    assert_eq!(*trace.0.borrow(), before);
}
