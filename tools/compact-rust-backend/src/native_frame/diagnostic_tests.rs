//! Selected frame producers and exact selection-miss boundaries.
use super::*;
use crate::{
    ir::{Contract, Parameter, SCHEMA_VERSION},
    source_diagnostics::{RefusalObserver, Slot},
};
use std::cell::RefCell;

#[derive(Default)]
struct Trace(RefCell<Vec<(&'static str, usize, Slot)>>);
impl RefusalObserver for Trace {
    fn refused(&self, site: RefusalSite<'_>) {
        let Some(owner) = site.owner else { return };
        let entry = match owner {
            Owner::Expr(node) => Some(("expr", std::ptr::from_ref(node) as usize)),
            Owner::Ledger(node) => Some(("ledger", std::ptr::from_ref(node) as usize)),
            Owner::Action(node) => Some(("action", std::ptr::from_ref(node) as usize)),
            Owner::Parameter(node) => Some(("parameter", std::ptr::from_ref(node) as usize)),
            Owner::Stateful(node) => Some(("circuit", std::ptr::from_ref(node) as usize)),
            _ => None,
        };
        if let Some((category, pointer)) = entry {
            self.0.borrow_mut().push((category, pointer, site.slot));
        }
    }
}
fn fixture(id: &str, witness: &str) -> Contract {
    Contract {
        schema_version: SCHEMA_VERSION,
        type_aliases: vec![],
        constructor: None,
        circuits: vec![],
        ledger_fields: vec![LedgerField {
            source: None,
            id: id.into(),
            index: 0,
            path: vec![0],
            declaration: LedgerFieldKind::Cell { ty: Type::Field },
        }],
        witnesses: vec![WitnessDeclaration {
            source: None,
            name: witness.into(),
            parameters: vec![],
            result: Type::Field,
        }],
        stateful_circuits: vec![StatefulCircuit {
            source: None,
            name: "update".into(),
            internal: false,
            parameters: vec![],
            actions: vec![StateAction::CellWrite {
                field: id.into(),
                index: 0,
                value: Expr::WitnessCall {
                    name: witness.into(),
                    arguments: vec![],
                },
            }],
            result: Type::Unit,
            return_value: StateReturn::Unit,
        }],
    }
}
fn run(
    c: &Contract,
    observer: Option<&dyn RefusalObserver>,
) -> Result<Option<String>, RenderError> {
    let ledger = c.ledger_fields.iter().map(|f| (f.id.as_str(), f)).collect();
    let witnesses = c.witnesses.iter().map(|w| (w.name.as_str(), w)).collect();
    let stateful = c
        .stateful_circuits
        .iter()
        .map(|s| (s.name.as_str(), s))
        .collect();
    let pure = HashMap::new();
    let result = if observer.is_some() {
        let environment = NativeExpressionContext {
            witnesses: &witnesses,
            circuits: &pure,
            stateful_circuits: &stateful,
            ledger_fields: &ledger,
            observer,
        };
        render_if_supported_observed(&c.stateful_circuits[0], &environment)
    } else {
        render_if_supported(&c.stateful_circuits[0], &ledger, &witnesses, &stateful)
    }?;
    Ok(result.map(|item| quote::quote!(#item).to_string()))
}
fn same_error(c: &Contract, trace: &Trace) -> RenderError {
    let expected = run(c, None).expect_err("actual selected producer error");
    let actual = run(c, Some(trace)).expect_err("same selected producer error");
    assert_eq!(actual, expected);
    actual
}

#[test]
fn selected_witnessed_cell_captures_original_ledger_id_and_stateful_junction() {
    let c = fixture("crate", "input");
    let trace = Trace::default();
    assert!(
        matches!(same_error(&c, &trace), RenderError::InvalidIdentifier(ref name) if name == "crate")
    );
    assert_eq!(
        trace.0.borrow().as_slice(),
        &[(
            "ledger",
            std::ptr::from_ref(&c.ledger_fields[0]) as usize,
            Slot::Field("id")
        )]
    );
    let ledger = c.ledger_fields.iter().map(|f| (f.id.as_str(), f)).collect();
    let witnesses = c.witnesses.iter().map(|w| (w.name.as_str(), w)).collect();
    let trace = Trace::default();
    let error = crate::stateful::render_stateful_circuit_observed(
        &c.stateful_circuits[0],
        &ledger,
        &witnesses,
        &HashMap::new(),
        &HashMap::new(),
        Some(&trace),
    )
    .err()
    .expect("frame error before general action");
    assert!(matches!(error, RenderError::InvalidIdentifier(ref name) if name == "crate"));
    assert_eq!(
        trace.0.borrow().as_slice(),
        &[(
            "ledger",
            std::ptr::from_ref(&c.ledger_fields[0]) as usize,
            Slot::Field("id")
        )]
    );
}

#[test]
fn actual_witness_name_and_earlier_literal_failures_keep_source_order() {
    let c = fixture("crate", "crate");
    let trace = Trace::default();
    let StateAction::CellWrite { value, .. } = &c.stateful_circuits[0].actions[0] else {
        panic!("fixture")
    };
    assert!(
        matches!(same_error(&c, &trace), RenderError::InvalidIdentifier(ref name) if name == "crate")
    );
    assert_eq!(
        trace.0.borrow().as_slice(),
        &[(
            "expr",
            std::ptr::from_ref(value) as usize,
            Slot::Field("name")
        )]
    );
    let mut c = fixture("crate", "crate");
    if let StateAction::CellWrite { value, .. } = &mut c.stateful_circuits[0].actions[0] {
        *value = Expr::Add {
            left: Box::new(Expr::FieldLiteral {
                value: "bad".into(),
            }),
            right: Box::new(Expr::WitnessCall {
                name: "crate".into(),
                arguments: vec![],
            }),
        };
    }
    let StateAction::CellWrite {
        value: Expr::Add { left, .. },
        ..
    } = &c.stateful_circuits[0].actions[0]
    else {
        panic!("fixture")
    };
    let trace = Trace::default();
    assert!(
        matches!(same_error(&c, &trace), RenderError::InvalidFieldLiteral(ref value) if value == "bad")
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
            .all(|entry| entry.2 != Slot::Field("name") && entry.0 != "ledger")
    );
}

#[test]
fn selected_call_name_reports_original_action_not_named_callee() {
    let mut c = fixture("slot", "input");
    let mut callee = c.stateful_circuits[0].clone();
    callee.name = "crate".into();
    callee.internal = true;
    c.stateful_circuits[0].actions = vec![StateAction::CircuitCall {
        name: "crate".into(),
        arguments: vec![],
    }];
    c.stateful_circuits.push(callee);
    let action = &c.stateful_circuits[0].actions[0];
    let trace = Trace::default();
    assert!(
        matches!(same_error(&c, &trace), RenderError::InvalidIdentifier(ref name) if name == "crate")
    );
    assert_eq!(
        trace.0.borrow().as_slice(),
        &[(
            "action",
            std::ptr::from_ref(action) as usize,
            Slot::Field("name")
        )]
    );
}

#[test]
fn selected_parameter_duplicate_and_final_circuit_names_keep_actual_owners() {
    let mut c = fixture("slot", "input");
    c.stateful_circuits[0].parameters = vec![Parameter {
        name: "crate".into(),
        ty: Type::Field,
    }];
    let trace = Trace::default();
    assert!(
        matches!(same_error(&c, &trace), RenderError::InvalidIdentifier(ref name) if name == "crate")
    );
    assert_eq!(
        trace.0.borrow().first(),
        Some(&(
            "parameter",
            std::ptr::from_ref(&c.stateful_circuits[0].parameters[0]) as usize,
            Slot::Field("name")
        ))
    );
    let mut c = fixture("slot", "input");
    c.stateful_circuits[0].parameters = vec![
        Parameter {
            name: "arg".into(),
            ty: Type::Field
        };
        2
    ];
    let trace = Trace::default();
    assert!(
        matches!(same_error(&c, &trace), RenderError::DuplicateParameter(ref name) if name == "arg")
    );
    assert_eq!(
        trace.0.borrow().first(),
        Some(&(
            "parameter",
            std::ptr::from_ref(&c.stateful_circuits[0].parameters[1]) as usize,
            Slot::Field("name")
        ))
    );
    let mut c = fixture("slot", "input");
    c.stateful_circuits[0].name = "crate".into();
    let trace = Trace::default();
    assert!(
        matches!(same_error(&c, &trace), RenderError::InvalidIdentifier(ref name) if name == "crate")
    );
    assert_eq!(
        trace.0.borrow().first(),
        Some(&(
            "circuit",
            std::ptr::from_ref(&c.stateful_circuits[0]) as usize,
            Slot::Field("name")
        ))
    );
}

#[test]
fn swallowed_simple_leaf_and_partial_selection_misses_publish_no_sites() {
    let mut c = fixture("crate", "input");
    c.ledger_fields[0].declaration = LedgerFieldKind::Cell { ty: Type::Boolean };
    c.stateful_circuits[0].actions = vec![StateAction::CellWrite {
        field: "crate".into(),
        index: 0,
        value: Expr::Boolean { value: true },
    }];
    let trace = Trace::default();
    assert!(run(&c, Some(&trace)).unwrap().is_none());
    assert!(trace.0.borrow().is_empty());
    let mut c = fixture("slot", "input");
    c.stateful_circuits[0]
        .actions
        .push(StateAction::CounterReset {
            field: "slot".into(),
            index: 0,
        });
    let trace = Trace::default();
    assert!(run(&c, Some(&trace)).unwrap().is_none());
    assert!(trace.0.borrow().is_empty());
    let mut c = fixture("slot", "input");
    if let StateAction::CellWrite { value, .. } = &mut c.stateful_circuits[0].actions[0] {
        *value = Expr::Add {
            left: Box::new(Expr::WitnessCall {
                name: "input".into(),
                arguments: vec![],
            }),
            right: Box::new(Expr::Parameter {
                name: "absent".into(),
            }),
        };
    }
    let trace = Trace::default();
    assert!(run(&c, Some(&trace)).unwrap().is_none());
    assert!(trace.0.borrow().is_empty());
}

#[test]
fn valid_selected_witnessed_frame_preserves_complete_generated_item_without_refusals() {
    let mut c = fixture("slot", "input");
    if let StateAction::CellWrite { value, .. } = &mut c.stateful_circuits[0].actions[0] {
        *value = Expr::Add {
            left: Box::new(Expr::WitnessCall {
                name: "input".into(),
                arguments: vec![],
            }),
            right: Box::new(Expr::FieldLiteral { value: "1".into() }),
        };
    }
    let trace = Trace::default();
    let observed = run(&c, Some(&trace))
        .unwrap()
        .expect("selected witnessed frame");
    assert_eq!(Some(observed), run(&c, None).unwrap());
    assert!(trace.0.borrow().is_empty());
}

#[test]
fn enabled_selected_witnessed_frame_keeps_actual_default_two_mib_boundary() {
    fn nested(depth: usize) -> Contract {
        let mut c = fixture("slot", "input");
        let mut body = Expr::WitnessCall {
            name: "input".into(),
            arguments: vec![],
        };
        for _ in 0..depth {
            body = Expr::Add {
                left: Box::new(Expr::FieldLiteral { value: "1".into() }),
                right: Box::new(body),
            };
        }
        if let StateAction::CellWrite { value, .. } = &mut c.stateful_circuits[0].actions[0] {
            *value = body;
        }
        c
    }
    let depth = (1..=64)
        .take_while(|d| crate::resource_limits::validate(&nested(*d)).is_ok())
        .last()
        .expect("small actual witnessed field cell accepted");
    assert!((3..48).contains(&depth), "unexpected DEFAULT depth{depth}");
    std::thread::Builder::new().stack_size(2 * 1024 * 1024).spawn(move || {
        let near = nested(depth); let trace = Trace::default(); let _preflight = crate::resource_limits::validate_with_observer(&near, Some(&trace)).unwrap();
        assert_eq!(run(&near, Some(&trace)).unwrap().expect("selected near boundary"), run(&near, None).unwrap().unwrap()); assert!(trace.0.borrow().is_empty());
        let over = nested(depth + 1); let trace = Trace::default(); let actual = crate::resource_limits::validate_with_observer(&over, Some(&trace)).err().expect("one over actual DEFAULT");
        assert!(matches!(actual, RenderError::ResourceLimit { .. })); assert_eq!(actual, crate::resource_limits::validate(&over).err().expect("same original refusal"));
        eprintln!("Selected witnessed frame Some/None: DEFAULT depth{depth} accepted, {} refused on ordinary2MiB", depth + 1);
    }).unwrap().join().unwrap();
}
