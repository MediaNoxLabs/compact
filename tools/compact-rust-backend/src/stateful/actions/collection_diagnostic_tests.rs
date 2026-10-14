//! Original collection producers; no synthetic owners or extra semantic checks.
use super::*;
use crate::ir::{Expr, LedgerFieldKind, StateReturn};
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
            _ => panic!("unexpected collection site"),
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
    let mut next = 17;
    let mut witness = true;
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

fn literal() -> Expr {
    Expr::FieldLiteral { value: "1".into() }
}
fn missing(name: &str) -> Expr {
    Expr::Parameter { name: name.into() }
}
fn collection_cases(id: &str) -> Vec<(LedgerField, StateAction)> {
    let list = || field(id, LedgerFieldKind::List { ty: Type::Field });
    let set = || field(id, LedgerFieldKind::Set { ty: Type::Field });
    let map = || {
        field(
            id,
            LedgerFieldKind::Map {
                key: Type::Field,
                value: Type::Field,
            },
        )
    };
    vec![
        (
            list(),
            StateAction::ListPushFront {
                field: id.into(),
                index: 0,
                value: literal(),
            },
        ),
        (
            list(),
            StateAction::ListPopFront {
                field: id.into(),
                index: 0,
            },
        ),
        (
            list(),
            StateAction::ListReset {
                field: id.into(),
                index: 0,
            },
        ),
        (
            set(),
            StateAction::SetInsert {
                field: id.into(),
                index: 0,
                value: literal(),
            },
        ),
        (
            set(),
            StateAction::SetRemove {
                field: id.into(),
                index: 0,
                value: literal(),
            },
        ),
        (
            set(),
            StateAction::SetReset {
                field: id.into(),
                index: 0,
            },
        ),
        (
            map(),
            StateAction::MapInsert {
                field: id.into(),
                index: 0,
                key: literal(),
                value: literal(),
            },
        ),
        (
            map(),
            StateAction::MapInsertDefault {
                field: id.into(),
                index: 0,
                key: literal(),
            },
        ),
        (
            map(),
            StateAction::MapRemove {
                field: id.into(),
                index: 0,
                key: literal(),
            },
        ),
        (
            map(),
            StateAction::MapReset {
                field: id.into(),
                index: 0,
            },
        ),
    ]
}
fn first_expr(trace: &Trace, expected: &Expr) {
    assert_eq!(
        trace.0.borrow().first(),
        Some(&("expr", std::ptr::from_ref(expected) as usize, Slot::Node))
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
fn all_eight_collection_name_checksites_keep_original_ledger_identity() {
    // Ten variants exercise eight match-arm name producers.
    for (field, action) in collection_cases("crate") {
        let trace = Trace::default();
        assert!(matches!(same_error(&action, &field, &trace),
            RenderError::InvalidIdentifier(ref name) if name == "crate"));
        ledger_site(&trace, &field);
    }
}

#[test]
fn all_six_original_operand_calls_capture_failed_child_before_name_guard() {
    let actions = [
        StateAction::ListPushFront {
            field: "crate".into(),
            index: 0,
            value: missing("first"),
        },
        StateAction::SetInsert {
            field: "crate".into(),
            index: 0,
            value: missing("first"),
        },
        StateAction::MapInsert {
            field: "crate".into(),
            index: 0,
            key: missing("first"),
            value: literal(),
        },
        StateAction::MapInsert {
            field: "crate".into(),
            index: 0,
            key: literal(),
            value: missing("first"),
        },
        StateAction::MapInsertDefault {
            field: "crate".into(),
            index: 0,
            key: missing("first"),
        },
        StateAction::MapRemove {
            field: "crate".into(),
            index: 0,
            key: missing("first"),
        },
    ];
    for action in &actions {
        let (kind, original) = match action {
            StateAction::ListPushFront { value, .. } => {
                (LedgerFieldKind::List { ty: Type::Field }, value)
            }
            StateAction::SetInsert { value, .. } => {
                (LedgerFieldKind::Set { ty: Type::Field }, value)
            }
            StateAction::MapInsert { key, value, .. } => (
                LedgerFieldKind::Map {
                    key: Type::Field,
                    value: Type::Field,
                },
                if matches!(key, Expr::Parameter { .. }) {
                    key
                } else {
                    value
                },
            ),
            StateAction::MapInsertDefault { key, .. } | StateAction::MapRemove { key, .. } => (
                LedgerFieldKind::Map {
                    key: Type::Field,
                    value: Type::Field,
                },
                key,
            ),
            _ => panic!("fixture"),
        };
        let field = field("crate", kind);
        let trace = Trace::default();
        assert!(matches!(same_error(action, &field, &trace),
            RenderError::UnknownParameter(ref name) if name == "first"));
        first_expr(&trace, original);
    }
}

#[test]
fn nested_map_key_leaf_precedes_value_and_invalid_ledger_name() {
    let field = field(
        "crate",
        LedgerFieldKind::Map {
            key: Type::Field,
            value: Type::Field,
        },
    );
    let action = StateAction::MapInsert {
        field: "crate".into(),
        index: 0,
        key: Expr::Add {
            left: Box::new(missing("first")),
            right: Box::new(missing("second")),
        },
        value: missing("later"),
    };
    let StateAction::MapInsert {
        key: Expr::Add { left, .. },
        ..
    } = &action
    else {
        panic!("fixture")
    };
    let trace = Trace::default();
    assert!(matches!(same_error(&action, &field, &trace),
        RenderError::UnknownParameter(ref name) if name == "first"));
    first_expr(&trace, left.as_ref());
}

#[test]
fn existing_key_type_guard_precedes_later_operand_and_keeps_original_key() {
    let field = field(
        "crate",
        LedgerFieldKind::Map {
            key: Type::Field,
            value: Type::Field,
        },
    );
    let action = StateAction::MapInsert {
        field: "crate".into(),
        index: 0,
        key: Expr::Boolean { value: true },
        value: missing("later"),
    };
    let trace = Trace::default();
    assert!(matches!(
        same_error(&action, &field, &trace),
        RenderError::TypeMismatch {
            expected: Type::Field,
            actual: Type::Boolean
        }
    ));
    let StateAction::MapInsert { key, .. } = &action else {
        panic!("fixture")
    };
    assert_eq!(
        trace.0.borrow().as_slice(),
        &[("expr", std::ptr::from_ref(key) as usize, Slot::Node)]
    );
}

#[test]
fn earlier_lookup_combined_kind_and_distinct_index_keep_original_action_slots() {
    let field = field("crate", LedgerFieldKind::Set { ty: Type::Field });
    for action in [
        StateAction::SetReset {
            field: "unknown".into(),
            index: 0,
        },
        StateAction::ListReset {
            field: "crate".into(),
            index: 0,
        },
        StateAction::SetInsert {
            field: "crate".into(),
            index: 1,
            value: missing("later"),
        },
    ] {
        let trace = Trace::default();
        assert!(matches!(
            same_error(&action, &field, &trace),
            RenderError::UnknownLedgerField(_)
        ));
        let slot = match &action {
            StateAction::SetReset { .. } => Slot::Field("field"),
            StateAction::ListReset { .. } => Slot::Node,
            StateAction::SetInsert { .. } => Slot::Field("index"),
            _ => panic!("fixture"),
        };
        assert_eq!(
            trace.0.borrow().as_slice(),
            &[("action", std::ptr::from_ref(&action) as usize, slot)]
        );
    }
}

#[test]
fn actual_stateful_junction_captures_collection_after_frame_selection_miss() {
    let field = field("crate", LedgerFieldKind::Set { ty: Type::Field });
    let ledger = HashMap::from([("crate", &field)]);
    let circuit = StatefulCircuit {
        source: None,
        name: "insert".into(),
        internal: false,
        parameters: vec![],
        actions: vec![StateAction::SetInsert {
            field: "crate".into(),
            index: 0,
            value: literal(),
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
    .expect("native refusal");
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
    .expect("native refusal");
    assert_eq!(actual, expected);
    assert!(matches!(actual, RenderError::InvalidIdentifier(ref name) if name == "crate"));
    ledger_site(&trace, &field);
}

#[test]
fn successful_all_variants_keep_statements_temporary_counter_and_witness_flag() {
    for (field, action) in collection_cases("values") {
        let trace = Trace::default();
        let actual = run(&action, &field, Some(&trace)).unwrap();
        assert_eq!(actual, run(&action, &field, None).unwrap());
        assert_eq!((actual.1, actual.2), (17, true));
        assert!(trace.0.borrow().is_empty());
    }
}
