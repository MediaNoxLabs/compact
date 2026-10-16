//! Actual Counter/Cell producer checks; no inferred parent or swallowed probe.
use super::*;
use crate::ir::{CounterAmount, Expr, LedgerFieldKind, StateReturn};
use crate::source_diagnostics::{Owner, RefusalObserver, RefusalSite, Slot};
use std::cell::RefCell;

#[derive(Default)]
struct Trace(RefCell<Vec<(&'static str, usize, Slot)>>);
impl RefusalObserver for Trace {
    fn refused(&self, site: RefusalSite<'_>) {
        let Some(owner) = site.owner else {
            panic!("real source expected")
        };
        let (category, address) = match owner {
            Owner::Ledger(field) => ("ledger", std::ptr::from_ref(field) as usize),
            Owner::Expr(value) => ("expr", std::ptr::from_ref(value) as usize),
            Owner::Action(value) => ("action", std::ptr::from_ref(value) as usize),
            Owner::CounterAmount(value) => ("amount", std::ptr::from_ref(value) as usize),
            _ => panic!("unexpected Counter/Cell site"),
        };
        self.0.borrow_mut().push((category, address, site.slot));
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
fn run(
    action: &StateAction,
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
    if observer.is_some() {
        lower_observed(
            action,
            &parameters,
            &environment,
            ActionOutput {
                statements: &mut statements,
                next_temp: &mut next,
                uses_witness: &mut witness,
            },
        )?;
    } else {
        lower(
            action,
            &parameters,
            &ledger,
            &witnesses,
            &pure,
            &stateful,
            &mut statements,
            &mut next,
            &mut witness,
        )?;
    }
    Ok((quote::quote!(#(#statements)*).to_string(), next, witness))
}
fn same_error(action: &StateAction, field: &LedgerField, trace: &Trace) -> RenderError {
    let expected = run(action, field, None).unwrap_err();
    let actual = run(action, field, Some(trace)).unwrap_err();
    assert_eq!(actual, expected);
    actual
}
fn ledger_site(trace: &Trace, field: &LedgerField) {
    assert_eq!(
        trace.0.borrow().as_slice(),
        &[(
            "ledger",
            std::ptr::from_ref(field) as usize,
            Slot::Field("id")
        )]
    );
}

#[test]
fn counter_name_guard_precedes_amount_lookup_and_keeps_original_declaration() {
    let field = field("crate", LedgerFieldKind::Counter);
    for action in [
        StateAction::CounterIncrement {
            field: "crate".into(),
            index: 0,
            amount: CounterAmount::Parameter {
                name: "missing".into(),
            },
        },
        StateAction::CounterDecrement {
            field: "crate".into(),
            index: 0,
            amount: CounterAmount::Parameter {
                name: "missing".into(),
            },
        },
        StateAction::CounterReset {
            field: "crate".into(),
            index: 0,
        },
    ] {
        let trace = Trace::default();
        assert!(
            matches!(same_error(&action, &field, &trace), RenderError::InvalidIdentifier(ref name) if name == "crate")
        );
        ledger_site(&trace, &field);
    }
}

#[test]
fn cell_operand_failure_precedes_ledger_name_and_keeps_actual_first_child() {
    let field = field("crate", LedgerFieldKind::Cell { ty: Type::Field });
    let action = StateAction::CellWrite {
        field: "crate".into(),
        index: 0,
        value: Expr::Add {
            left: Box::new(Expr::Parameter {
                name: "first".into(),
            }),
            right: Box::new(Expr::Parameter {
                name: "second".into(),
            }),
        },
    };
    let StateAction::CellWrite {
        value: Expr::Add { left, .. },
        ..
    } = &action
    else {
        panic!("fixture")
    };
    let trace = Trace::default();
    assert!(
        matches!(same_error(&action, &field, &trace), RenderError::UnknownParameter(ref name) if name == "first")
    );
    assert_eq!(
        trace.0.borrow().first(),
        Some(&(
            "expr",
            std::ptr::from_ref(left.as_ref()) as usize,
            Slot::Node
        ))
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
fn cell_name_guard_runs_only_after_successful_original_operand() {
    let field = field("crate", LedgerFieldKind::Cell { ty: Type::Field });
    let action = StateAction::CellWrite {
        field: "crate".into(),
        index: 0,
        value: Expr::FieldLiteral { value: "1".into() },
    };
    let trace = Trace::default();
    assert!(
        matches!(same_error(&action, &field, &trace), RenderError::InvalidIdentifier(ref name) if name == "crate")
    );
    ledger_site(&trace, &field);
}

#[test]
fn earlier_combined_target_failure_keeps_original_action_without_identifier_guess() {
    let field = field("crate", LedgerFieldKind::Counter);
    let action = StateAction::CounterReset {
        field: "crate".into(),
        index: 1,
    };
    let trace = Trace::default();
    assert!(
        matches!(same_error(&action, &field, &trace), RenderError::UnknownLedgerField(ref name) if name == "crate")
    );
    assert_eq!(
        trace.0.borrow().as_slice(),
        &[("action", std::ptr::from_ref(&action) as usize, Slot::Node)]
    );
}

#[test]
fn valid_ledger_with_missing_amount_keeps_original_amount_name_site() {
    let field = field("count", LedgerFieldKind::Counter);
    let action = StateAction::CounterIncrement {
        field: "count".into(),
        index: 0,
        amount: CounterAmount::Parameter {
            name: "missing".into(),
        },
    };
    let trace = Trace::default();
    assert!(
        matches!(same_error(&action, &field, &trace), RenderError::UnknownParameter(ref name) if name == "missing")
    );
    let StateAction::CounterIncrement { amount, .. } = &action else {
        panic!("fixture")
    };
    assert_eq!(
        trace.0.borrow().as_slice(),
        &[(
            "amount",
            std::ptr::from_ref(amount) as usize,
            Slot::Field("name")
        )]
    );
}

#[test]
fn actual_stateful_junction_captures_counter_action_after_frame_miss() {
    let field = field("crate", LedgerFieldKind::Counter);
    let ledger = HashMap::from([("crate", &field)]);
    let circuit = StatefulCircuit {
        source: None,
        name: "bump".into(),
        internal: false,
        parameters: vec![],
        actions: vec![StateAction::CounterIncrement {
            field: "crate".into(),
            index: 0,
            amount: CounterAmount::Literal { value: 1 },
        }],
        result: Type::Unit,
        return_value: StateReturn::Unit,
    };
    let expected = crate::stateful::render_stateful_circuit(
        &circuit,
        &ledger,
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
    )
    .err()
    .expect("expected native refusal");
    let trace = Trace::default();
    let actual = crate::stateful::render_stateful_circuit_observed(
        &circuit,
        &ledger,
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        Some(&trace),
    )
    .err()
    .expect("expected native refusal");
    assert_eq!(actual, expected);
    assert!(matches!(actual, RenderError::InvalidIdentifier(ref name) if name == "crate"));
    ledger_site(&trace, &field);
}

#[test]
fn successful_counter_and_cell_actions_keep_statements_counters_and_witness_flag() {
    let counter = field("count", LedgerFieldKind::Counter);
    for action in [
        StateAction::CounterIncrement {
            field: "count".into(),
            index: 0,
            amount: CounterAmount::Literal { value: 1 },
        },
        StateAction::CounterReset {
            field: "count".into(),
            index: 0,
        },
    ] {
        let trace = Trace::default();
        assert_eq!(
            run(&action, &counter, Some(&trace)).unwrap(),
            run(&action, &counter, None).unwrap()
        );
        assert!(trace.0.borrow().is_empty());
    }
    let cell = field("cell", LedgerFieldKind::Cell { ty: Type::Field });
    let action = StateAction::CellWrite {
        field: "cell".into(),
        index: 0,
        value: Expr::FieldLiteral { value: "1".into() },
    };
    let trace = Trace::default();
    assert_eq!(
        run(&action, &cell, Some(&trace)).unwrap(),
        run(&action, &cell, None).unwrap()
    );
    assert!(trace.0.borrow().is_empty());
}
