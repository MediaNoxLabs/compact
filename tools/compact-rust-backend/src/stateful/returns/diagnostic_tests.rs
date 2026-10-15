//! Direct final return guard source identity; parent replays the genuine source.
use super::*;
use crate::{ir::Expr, source_diagnostics::Slot};
use std::cell::RefCell;

#[derive(Default)]
struct Trace(RefCell<Vec<(&'static str, usize, Slot)>>);
impl RefusalObserver for Trace {
    fn refused(&self, site: RefusalSite<'_>) {
        let Some(owner) = site.owner else {
            panic!("real borrowed site expected")
        };
        let (category, pointer) = match owner {
            Owner::Ledger(field) => ("ledger", std::ptr::from_ref(field) as usize),
            Owner::Expr(value) => ("expr", std::ptr::from_ref(value) as usize),
            Owner::StateReturn(value) => ("return", std::ptr::from_ref(value) as usize),
            Owner::Type(value) => ("type", std::ptr::from_ref(value) as usize),
            _ => panic!("unexpected final return source"),
        };
        self.0.borrow_mut().push((category, pointer, site.slot));
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
fn circuit(result: Type, return_value: StateReturn) -> StatefulCircuit {
    StatefulCircuit {
        source: None,
        name: "probe".into(),
        internal: false,
        parameters: vec![],
        actions: vec![],
        result,
        return_value,
    }
}
fn counter_result() -> Type {
    Type::Unsigned {
        max: u64::MAX.to_string(),
    }
}
fn lower_source(
    source: &StatefulCircuit,
    field: &LedgerField,
    observer: Option<&dyn RefusalObserver>,
) -> Result<(String, usize, bool), RenderError> {
    let ledger = HashMap::from([(field.id.as_str(), field)]);
    let witnesses = HashMap::new();
    let pure = HashMap::new();
    let stateful = HashMap::new();
    let parameters = HashMap::new();
    let environment = NativeExpressionContext {
        witnesses: &witnesses,
        circuits: &pure,
        stateful_circuits: &stateful,
        ledger_fields: &ledger,
        observer,
    };
    let mut statements = Vec::new();
    let mut next = 0;
    let mut witness = false;
    let result_ty = crate::rust_type(&source.result)?;
    let value = lower_observed(
        source,
        &parameters,
        &parameters,
        &environment,
        &result_ty,
        ReturnOutput {
            statements: &mut statements,
            next_temp: &mut next,
            uses_witness: &mut witness,
        },
    )?;
    Ok((
        quote::quote!(#(#statements)* #value).to_string(),
        next,
        witness,
    ))
}
fn same_error(source: &StatefulCircuit, field: &LedgerField, trace: &Trace) -> RenderError {
    let expected = lower_source(source, field, None).unwrap_err();
    let actual = lower_source(source, field, Some(trace)).unwrap_err();
    assert_eq!(actual, expected);
    actual
}

#[test]
fn counter_read_identifier_guard_captures_original_ledger_id_before_later_headers() {
    let field = field("crate", LedgerFieldKind::Counter);
    let source = circuit(
        counter_result(),
        StateReturn::CounterRead {
            field: "crate".into(),
            index: 0,
        },
    );
    let trace = Trace::default();
    assert!(
        matches!(same_error(&source, &field, &trace), RenderError::InvalidIdentifier(ref name) if name == "crate")
    );
    assert_eq!(
        trace.0.borrow().as_slice(),
        &[(
            "ledger",
            std::ptr::from_ref(&field) as usize,
            Slot::Field("id")
        )]
    );
    // The actual stateful junction uses the same native observer, not frame probe.
    let ledger = HashMap::from([("crate", &field)]);
    let trace = Trace::default();
    let actual = crate::stateful::render_stateful_circuit_observed(
        &source,
        &ledger,
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        Some(&trace),
    )
    .err()
    .expect("expected native refusal");
    assert!(matches!(actual, RenderError::InvalidIdentifier(ref name) if name == "crate"));
    assert_eq!(
        trace.0.borrow().as_slice(),
        &[(
            "ledger",
            std::ptr::from_ref(&field) as usize,
            Slot::Field("id")
        )]
    );
}

#[test]
fn earlier_bad_index_and_result_guard_keep_original_return_and_declared_result() {
    let field = field("crate", LedgerFieldKind::Counter);
    let source = circuit(
        counter_result(),
        StateReturn::CounterRead {
            field: "crate".into(),
            index: 1,
        },
    );
    let trace = Trace::default();
    assert!(
        matches!(same_error(&source, &field, &trace), RenderError::UnknownLedgerField(ref name) if name == "crate")
    );
    assert_eq!(
        trace.0.borrow().as_slice(),
        &[(
            "return",
            std::ptr::from_ref(&source.return_value) as usize,
            Slot::Node
        )]
    );
    let source = circuit(
        Type::Boolean,
        StateReturn::CounterRead {
            field: "crate".into(),
            index: 0,
        },
    );
    let trace = Trace::default();
    assert!(matches!(
        same_error(&source, &field, &trace),
        RenderError::TypeMismatch { .. }
    ));
    assert_eq!(
        trace.0.borrow().as_slice(),
        &[(
            "type",
            std::ptr::from_ref(&source.result) as usize,
            Slot::Node
        )]
    );
}

#[test]
fn expression_return_captures_original_first_child_through_same_context() {
    let field = field("unused", LedgerFieldKind::Counter);
    let source = circuit(
        Type::Field,
        StateReturn::Expression {
            value: Expr::Add {
                left: Box::new(Expr::Parameter {
                    name: "first".into(),
                }),
                right: Box::new(Expr::Parameter {
                    name: "second".into(),
                }),
            },
        },
    );
    let StateReturn::Expression {
        value: Expr::Add { left, .. },
    } = &source.return_value
    else {
        panic!("fixture")
    };
    let trace = Trace::default();
    assert!(
        matches!(same_error(&source, &field, &trace), RenderError::UnknownParameter(ref name) if name == "first")
    );
    assert_eq!(
        trace.0.borrow().first(),
        Some(&(
            "expr",
            std::ptr::from_ref(left.as_ref()) as usize,
            Slot::Node
        ))
    );
}

#[test]
fn specialized_set_operand_failure_precedes_bad_ledger_name_and_stays_original() {
    let field = field("crate", LedgerFieldKind::Set { ty: Type::Field });
    let source = circuit(
        Type::Boolean,
        StateReturn::SetMember {
            field: "crate".into(),
            index: 0,
            value: Expr::Parameter {
                name: "missing".into(),
            },
        },
    );
    let StateReturn::SetMember { value, .. } = &source.return_value else {
        panic!("fixture")
    };
    let trace = Trace::default();
    assert!(
        matches!(same_error(&source, &field, &trace), RenderError::UnknownParameter(ref name) if name == "missing")
    );
    assert_eq!(
        trace.0.borrow().first(),
        Some(&("expr", std::ptr::from_ref(value) as usize, Slot::Node))
    );
    assert!(
        trace
            .0
            .borrow()
            .iter()
            .all(|(category, _, _)| *category != "ledger")
    );
}

#[test]
fn successful_counter_and_expression_return_keep_output_state_without_sites() {
    let field = field("count", LedgerFieldKind::Counter);
    for source in [
        circuit(
            counter_result(),
            StateReturn::CounterRead {
                field: "count".into(),
                index: 0,
            },
        ),
        circuit(
            Type::Boolean,
            StateReturn::Expression {
                value: Expr::Boolean { value: true },
            },
        ),
    ] {
        let trace = Trace::default();
        assert_eq!(
            lower_source(&source, &field, Some(&trace)).unwrap(),
            lower_source(&source, &field, None).unwrap()
        );
        assert!(trace.0.borrow().is_empty());
    }
}

#[test]
fn expression_result_mismatch_captures_actual_successful_operand_not_annotation_clone() {
    let field = field("unused", LedgerFieldKind::Counter);
    let source = circuit(
        Type::Field,
        StateReturn::Expression {
            value: Expr::Boolean { value: true },
        },
    );
    let StateReturn::Expression { value } = &source.return_value else {
        panic!("fixture")
    };
    let trace = Trace::default();
    assert!(matches!(
        same_error(&source, &field, &trace),
        RenderError::TypeMismatch {
            expected: Type::Field,
            actual: Type::Boolean
        }
    ));
    assert_eq!(
        trace.0.borrow().as_slice(),
        &[("expr", std::ptr::from_ref(value) as usize, Slot::Node)]
    );
}
