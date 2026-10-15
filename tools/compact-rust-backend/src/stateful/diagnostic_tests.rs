//! Private IR controls for direct native assembly guards; no frontend claim.
use super::*;
use crate::ir::Expr;
use crate::ir::LocalBinding;
use crate::source_diagnostics::Slot;
use std::cell::RefCell;

#[derive(Default)]
struct Trace(RefCell<Vec<(&'static str, usize, Slot)>>);
impl RefusalObserver for Trace {
    fn refused(&self, site: RefusalSite<'_>) {
        let Some(owner) = site.owner else {
            panic!("direct source expected")
        };
        let (category, address) = match owner {
            Owner::Expr(expr) => ("expr", std::ptr::from_ref(expr) as usize),
            Owner::StateReturn(value) => ("return", std::ptr::from_ref(value) as usize),
            _ => panic!("unexpected direct assembly owner"),
        };
        self.0.borrow_mut().push((category, address, site.slot));
    }
}
fn source(actions: Vec<StateAction>, result: Type, return_value: StateReturn) -> StatefulCircuit {
    StatefulCircuit {
        source: None,
        name: "probe".into(),
        internal: false,
        parameters: vec![],
        actions,
        result,
        return_value,
    }
}
fn unit_plan() -> Box<ReturnPlan> {
    Box::new(ReturnPlan::Value { value: Expr::Unit })
}
fn empty_action() -> Box<StateAction> {
    Box::new(StateAction::Sequence { actions: vec![] })
}
fn error(source: &StatefulCircuit, trace: &Trace) -> RenderError {
    let plain = render_stateful_circuit(
        source,
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
    )
    .err()
    .expect("expected native refusal");
    let actual = render_stateful_circuit_observed(
        source,
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        Some(trace),
    )
    .err()
    .expect("expected native refusal");
    assert_eq!(actual, plain);
    actual
}
fn assert_expr(trace: &Trace, expr: &Expr) {
    assert_eq!(
        trace.0.borrow().as_slice(),
        &[("expr", std::ptr::from_ref(expr) as usize, Slot::Node)]
    );
}

#[test]
fn effectful_result_guard_reports_original_value_before_result_type_clone() {
    let source = source(
        vec![],
        Type::Field,
        StateReturn::Effectful {
            body: ReturnPlan::Value {
                value: Expr::Boolean { value: true },
            },
        },
    );
    let StateReturn::Effectful {
        body: ReturnPlan::Value { value },
    } = &source.return_value
    else {
        panic!("fixture")
    };
    let trace = Trace::default();
    assert!(matches!(
        error(&source, &trace),
        RenderError::TypeMismatch {
            expected: Type::Field,
            actual: Type::Boolean
        }
    ));
    assert_expr(&trace, value);
}

#[test]
fn plan_and_action_conditions_capture_actual_failed_boolean_guard_operand() {
    let plan = source(
        vec![],
        Type::Unit,
        StateReturn::Effectful {
            body: ReturnPlan::Conditional {
                condition: Expr::FieldLiteral { value: "1".into() },
                then: unit_plan(),
                otherwise: unit_plan(),
            },
        },
    );
    let StateReturn::Effectful {
        body: ReturnPlan::Conditional { condition, .. },
    } = &plan.return_value
    else {
        panic!("fixture")
    };
    let trace = Trace::default();
    assert!(matches!(
        error(&plan, &trace),
        RenderError::TypeMismatch {
            expected: Type::Boolean,
            actual: Type::Field
        }
    ));
    assert_expr(&trace, condition);
    let action = source(
        vec![StateAction::If {
            condition: Expr::FieldLiteral { value: "1".into() },
            then: empty_action(),
            otherwise: empty_action(),
        }],
        Type::Unit,
        StateReturn::Unit,
    );
    let StateAction::If { condition, .. } = &action.actions[0] else {
        panic!("fixture")
    };
    let trace = Trace::default();
    assert!(matches!(
        error(&action, &trace),
        RenderError::TypeMismatch {
            expected: Type::Boolean,
            actual: Type::Field
        }
    ));
    assert_expr(&trace, condition);
}

#[test]
fn plan_and_action_binding_guards_keep_original_value_identity() {
    let binding = || LocalBinding {
        name: "local".into(),
        ty: Type::Field,
        value: Expr::Boolean { value: true },
    };
    let plan = source(
        vec![],
        Type::Unit,
        StateReturn::Effectful {
            body: ReturnPlan::Let {
                bindings: vec![binding()],
                result: unit_plan(),
            },
        },
    );
    let StateReturn::Effectful {
        body: ReturnPlan::Let { bindings, .. },
    } = &plan.return_value
    else {
        panic!("fixture")
    };
    let trace = Trace::default();
    assert!(matches!(
        error(&plan, &trace),
        RenderError::TypeMismatch {
            expected: Type::Field,
            actual: Type::Boolean
        }
    ));
    assert_expr(&trace, &bindings[0].value);
    let action = source(
        vec![StateAction::Let {
            bindings: vec![binding()],
            action: empty_action(),
        }],
        Type::Unit,
        StateReturn::Unit,
    );
    let StateAction::Let { bindings, .. } = &action.actions[0] else {
        panic!("fixture")
    };
    let trace = Trace::default();
    assert!(matches!(
        error(&action, &trace),
        RenderError::TypeMismatch {
            expected: Type::Field,
            actual: Type::Boolean
        }
    ));
    assert_expr(&trace, &bindings[0].value);
}

#[test]
fn earlier_nested_failure_is_not_replaced_by_a_parent_guard_site() {
    let source = source(
        vec![],
        Type::Boolean,
        StateReturn::Effectful {
            body: ReturnPlan::Value {
                value: Expr::Add {
                    left: Box::new(Expr::Parameter {
                        name: "missing_first".into(),
                    }),
                    right: Box::new(Expr::Parameter {
                        name: "missing_second".into(),
                    }),
                },
            },
        },
    );
    let trace = Trace::default();
    assert!(
        matches!(error(&source, &trace), RenderError::UnknownParameter(ref name) if name == "missing_first")
    );
    let StateReturn::Effectful {
        body: ReturnPlan::Value {
            value: Expr::Add { left, .. },
        },
    } = &source.return_value
    else {
        panic!("fixture")
    };
    assert_expr(&trace, left);
}

#[test]
fn malformed_plan_captures_original_return_owner_before_frame_selection() {
    let source = source(
        vec![StateAction::Expression { value: Expr::Unit }],
        Type::Unit,
        StateReturn::Effectful { body: *unit_plan() },
    );
    let trace = Trace::default();
    assert_eq!(error(&source, &trace), RenderError::MalformedReturnPlan);
    assert_eq!(
        trace.0.borrow().as_slice(),
        &[(
            "return",
            std::ptr::from_ref(&source.return_value) as usize,
            Slot::Node
        )]
    );
}

#[test]
fn observed_valid_assembly_keeps_native_output_and_emits_no_refusal() {
    let source = source(
        vec![],
        Type::Boolean,
        StateReturn::Expression {
            value: Expr::Boolean { value: true },
        },
    );
    let trace = Trace::default();
    let plain = render_stateful_circuit(
        &source,
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
    )
    .unwrap();
    let observed = render_stateful_circuit_observed(
        &source,
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        Some(&trace),
    )
    .unwrap();
    assert_eq!(
        quote::quote!(#plain).to_string(),
        quote::quote!(#observed).to_string()
    );
    assert!(trace.0.borrow().is_empty());
}
