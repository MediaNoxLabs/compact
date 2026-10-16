//! Borrowed coin/Merkle producers with unchanged native operand scheduling.
use super::*;
use crate::ir::{Expr, LedgerFieldKind, Parameter, StateReturn};
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
            _ => panic!("unexpected coin/Merkle site"),
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
    let coin_type = crate::coin_shapes::shielded_coin_type();
    let recipient_type = crate::coin_shapes::shielded_recipient_type();
    let parameters = HashMap::from([
        ("coin", (&coin_type, syn::parse_quote!(coin))),
        ("recipient", (&recipient_type, syn::parse_quote!(recipient))),
    ]);
    let environment = NativeExpressionContext {
        witnesses: &witnesses,
        circuits: &pure,
        stateful_circuits: &stateful,
        ledger_fields: &ledger,
        observer,
    };
    let mut statements = Vec::new();
    let mut next = 17;
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

fn literal() -> Expr {
    Expr::FieldLiteral { value: "1".into() }
}
fn parameter(name: &str) -> Expr {
    Expr::Parameter { name: name.into() }
}
fn position() -> Expr {
    Expr::UnsignedLiteral {
        value: "0".into(),
        max: u64::MAX.to_string(),
    }
}
fn hash() -> Expr {
    Expr::BytesLiteral { bytes: vec![0; 32] }
}
fn first_expr(trace: &Trace, value: &Expr) {
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
fn coin_cases(id: &str) -> Vec<(LedgerField, StateAction)> {
    let ty = crate::coin_shapes::qualified_coin_type();
    vec![
        (
            field(id, LedgerFieldKind::Set { ty: ty.clone() }),
            StateAction::SetInsertCoin {
                field: id.into(),
                index: 0,
                coin: parameter("coin"),
                recipient: parameter("recipient"),
            },
        ),
        (
            field(id, LedgerFieldKind::Cell { ty }),
            StateAction::CellWriteCoin {
                field: id.into(),
                index: 0,
                coin: parameter("coin"),
                recipient: parameter("recipient"),
            },
        ),
    ]
}
fn merkle_cases(id: &str) -> Vec<(LedgerField, StateAction)> {
    let plain = || {
        field(
            id,
            LedgerFieldKind::MerkleTree {
                depth: 8,
                ty: Type::Field,
            },
        )
    };
    let historic = || {
        field(
            id,
            LedgerFieldKind::HistoricMerkleTree {
                depth: 8,
                ty: Type::Field,
            },
        )
    };
    vec![
        (
            historic(),
            StateAction::HistoricMerkleResetHistory {
                field: id.into(),
                index: 0,
            },
        ),
        (
            historic(),
            StateAction::HistoricMerkleResetToDefault {
                field: id.into(),
                index: 0,
            },
        ),
        (
            plain(),
            StateAction::MerkleResetToDefault {
                field: id.into(),
                index: 0,
            },
        ),
        (
            historic(),
            StateAction::HistoricMerkleInsert {
                field: id.into(),
                index: 0,
                value: literal(),
            },
        ),
        (
            plain(),
            StateAction::MerkleInsert {
                field: id.into(),
                index: 0,
                value: literal(),
            },
        ),
        (
            historic(),
            StateAction::HistoricMerkleInsertIndex {
                field: id.into(),
                index: 0,
                value: literal(),
                position: position(),
            },
        ),
        (
            plain(),
            StateAction::MerkleInsertIndex {
                field: id.into(),
                index: 0,
                value: literal(),
                position: position(),
            },
        ),
        (
            historic(),
            StateAction::HistoricMerkleInsertHash {
                field: id.into(),
                index: 0,
                hash: hash(),
            },
        ),
        (
            plain(),
            StateAction::MerkleInsertHash {
                field: id.into(),
                index: 0,
                hash: hash(),
            },
        ),
        (
            historic(),
            StateAction::HistoricMerkleInsertHashIndex {
                field: id.into(),
                index: 0,
                hash: hash(),
                position: position(),
            },
        ),
        (
            plain(),
            StateAction::MerkleInsertHashIndex {
                field: id.into(),
                index: 0,
                hash: hash(),
                position: position(),
            },
        ),
        (
            historic(),
            StateAction::HistoricMerkleInsertIndexDefault {
                field: id.into(),
                index: 0,
                position: position(),
            },
        ),
        (
            plain(),
            StateAction::MerkleInsertIndexDefault {
                field: id.into(),
                index: 0,
                position: position(),
            },
        ),
    ]
}

#[test]
fn all_coin_and_merkle_name_guards_keep_original_ledger_not_generated_method() {
    for (field, action) in coin_cases("crate").into_iter().chain(merkle_cases("crate")) {
        let trace = Trace::default();
        assert!(
            matches!(same_error(&action, &field, &trace), RenderError::InvalidIdentifier(ref name) if name == "crate")
        );
        ledger_site(&trace, &field);
    }
}

#[test]
fn coin_first_leaf_then_recipient_leaf_precede_invalid_ledger_name() {
    for recipient_failure in [false, true] {
        let field = field(
            "crate",
            LedgerFieldKind::Set {
                ty: crate::coin_shapes::qualified_coin_type(),
            },
        );
        let action = StateAction::SetInsertCoin {
            field: "crate".into(),
            index: 0,
            coin: if recipient_failure {
                parameter("coin")
            } else {
                Expr::Add {
                    left: Box::new(parameter("first")),
                    right: Box::new(parameter("second")),
                }
            },
            recipient: parameter("later"),
        };
        let StateAction::SetInsertCoin {
            coin, recipient, ..
        } = &action
        else {
            panic!("fixture")
        };
        let failed = if recipient_failure {
            recipient
        } else {
            let Expr::Add { left, .. } = coin else {
                panic!("fixture")
            };
            left.as_ref()
        };
        let trace = Trace::default();
        let name = if recipient_failure { "later" } else { "first" };
        assert!(
            matches!(same_error(&action, &field, &trace), RenderError::UnknownParameter(ref actual) if actual == name)
        );
        first_expr(&trace, failed);
    }
}

#[test]
fn all_three_merkle_operand_calls_keep_value_before_position_before_name() {
    let actions = [
        StateAction::MerkleInsertIndex {
            field: "crate".into(),
            index: 0,
            value: parameter("first"),
            position: parameter("later"),
        },
        StateAction::MerkleInsertIndex {
            field: "crate".into(),
            index: 0,
            value: literal(),
            position: parameter("first"),
        },
        StateAction::MerkleInsertIndexDefault {
            field: "crate".into(),
            index: 0,
            position: parameter("first"),
        },
    ];
    let field = field(
        "crate",
        LedgerFieldKind::MerkleTree {
            depth: 8,
            ty: Type::Field,
        },
    );
    for action in &actions {
        let failed = match action {
            StateAction::MerkleInsertIndex {
                value, position, ..
            } => {
                if matches!(value, Expr::Parameter { .. }) {
                    value
                } else {
                    position
                }
            }
            StateAction::MerkleInsertIndexDefault { position, .. } => position,
            _ => panic!("fixture"),
        };
        let trace = Trace::default();
        assert!(
            matches!(same_error(action, &field, &trace), RenderError::UnknownParameter(ref name) if name == "first")
        );
        first_expr(&trace, failed);
    }
}

#[test]
fn existing_operand_type_guards_still_precede_later_operands_and_keep_original_expr() {
    let pairs = [
        (
            field(
                "crate",
                LedgerFieldKind::Set {
                    ty: crate::coin_shapes::qualified_coin_type(),
                },
            ),
            StateAction::SetInsertCoin {
                field: "crate".into(),
                index: 0,
                coin: literal(),
                recipient: parameter("later"),
            },
        ),
        (
            field(
                "crate",
                LedgerFieldKind::MerkleTree {
                    depth: 8,
                    ty: Type::Field,
                },
            ),
            StateAction::MerkleInsertIndex {
                field: "crate".into(),
                index: 0,
                value: Expr::Boolean { value: true },
                position: parameter("later"),
            },
        ),
        (
            field(
                "crate",
                LedgerFieldKind::MerkleTree {
                    depth: 8,
                    ty: Type::Field,
                },
            ),
            StateAction::MerkleInsertIndexDefault {
                field: "crate".into(),
                index: 0,
                position: literal(),
            },
        ),
    ];
    for (field, action) in pairs {
        let trace = Trace::default();
        assert!(matches!(
            same_error(&action, &field, &trace),
            RenderError::TypeMismatch { .. }
        ));
        let original = match &action {
            StateAction::SetInsertCoin { coin, .. } => coin,
            StateAction::MerkleInsertIndex { value, .. } => value,
            StateAction::MerkleInsertIndexDefault { position, .. } => position,
            _ => panic!("fixture"),
        };
        assert_eq!(
            trace.0.borrow().as_slice(),
            &[("expr", std::ptr::from_ref(original) as usize, Slot::Node)]
        );
    }
}

#[test]
fn wrong_index_kind_and_missing_field_remain_before_operands_at_original_action_sites() {
    for (mut field, action) in coin_cases("crate").into_iter().chain(merkle_cases("crate")) {
        field.index = 1;
        let trace = Trace::default();
        assert!(matches!(
            same_error(&action, &field, &trace),
            RenderError::UnknownLedgerField(_)
        ));
        assert_eq!(
            trace.0.borrow().as_slice(),
            &[("action", std::ptr::from_ref(&action) as usize, Slot::Node)]
        );
    }
    let field = field("crate", LedgerFieldKind::Counter);
    for action in [
        StateAction::MerkleResetToDefault {
            field: "crate".into(),
            index: 0,
        },
        StateAction::SetInsertCoin {
            field: "absent".into(),
            index: 0,
            coin: parameter("later"),
            recipient: parameter("later"),
        },
    ] {
        let trace = Trace::default();
        assert!(matches!(
            same_error(&action, &field, &trace),
            RenderError::UnknownLedgerField(_)
        ));
        assert_eq!(
            trace.0.borrow().as_slice(),
            &[(
                "action",
                std::ptr::from_ref(&action) as usize,
                Slot::Field("field")
            )]
        );
    }
}

#[test]
fn actual_stateful_junction_captures_coin_and_merkle_after_frame_miss() {
    let pairs = vec![
        coin_cases("crate").remove(0),
        merkle_cases("crate").remove(2),
    ];
    for (field, action) in pairs {
        let ledger = HashMap::from([("crate", &field)]);
        let circuit = StatefulCircuit {
            source: None,
            name: "mutate".into(),
            internal: false,
            parameters: vec![
                Parameter {
                    name: "coin".into(),
                    ty: crate::coin_shapes::shielded_coin_type(),
                },
                Parameter {
                    name: "recipient".into(),
                    ty: crate::coin_shapes::shielded_recipient_type(),
                },
            ],
            actions: vec![action],
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
}

#[test]
fn all_successful_coin_and_merkle_variants_keep_output_and_effect_state() {
    for (field, action) in coin_cases("slot").into_iter().chain(merkle_cases("slot")) {
        let trace = Trace::default();
        let actual = run(&action, &field, Some(&trace)).unwrap();
        assert_eq!(actual, run(&action, &field, None).unwrap());
        assert!(!actual.2);
        let expected_next = if matches!(
            action,
            StateAction::SetInsertCoin { .. } | StateAction::CellWriteCoin { .. }
        ) {
            19
        } else {
            17
        };
        assert_eq!(actual.1, expected_next);
        assert!(trace.0.borrow().is_empty());
    }
}
