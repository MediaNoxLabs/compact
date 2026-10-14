//! Original stateful lexical names, before their initializer checks.
use super::*;
use crate::ir::{Expr, LocalBinding, Parameter};
use crate::source_diagnostics::Slot;
use std::cell::RefCell;

#[derive(Default)]
struct Trace(RefCell<Vec<(&'static str, usize, Slot)>>);
impl RefusalObserver for Trace {
    fn refused(&self, site: RefusalSite<'_>) {
        let (kind, pointer) = match site.owner.expect("original source") {
            Owner::Binding(binding) => ("binding", std::ptr::from_ref(binding) as usize),
            Owner::Expr(value) => ("expr", std::ptr::from_ref(value) as usize),
            _ => panic!("unexpected lexical source"),
        };
        self.0.borrow_mut().push((kind, pointer, site.slot));
    }
}
fn binding(name: &str, value: Expr) -> LocalBinding {
    LocalBinding {
        name: name.into(),
        ty: Type::Field,
        value,
    }
}
fn parameter(name: &str) -> Expr {
    Expr::Parameter { name: name.into() }
}
fn literal() -> Expr {
    Expr::FieldLiteral { value: "1".into() }
}
fn empty_action() -> Box<StateAction> {
    Box::new(StateAction::Sequence { actions: vec![] })
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
fn plans(bindings: Vec<LocalBinding>) -> StatefulCircuit {
    source(
        vec![],
        Type::Field,
        StateReturn::Effectful {
            body: ReturnPlan::Let {
                bindings,
                result: Box::new(ReturnPlan::Value { value: literal() }),
            },
        },
    )
}
fn actions(bindings: Vec<LocalBinding>) -> StatefulCircuit {
    source(
        vec![StateAction::Let {
            bindings,
            action: empty_action(),
        }],
        Type::Unit,
        StateReturn::Unit,
    )
}
fn run(
    source: &StatefulCircuit,
    observer: Option<&dyn RefusalObserver>,
) -> Result<syn::Item, RenderError> {
    if observer.is_some() {
        render_stateful_circuit_observed(
            source,
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            observer,
        )
    } else {
        render_stateful_circuit(
            source,
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
        )
    }
}
fn same_error(source: &StatefulCircuit, trace: &Trace) -> RenderError {
    let expected = run(source, None).err().expect("native refusal");
    let actual = run(source, Some(trace)).err().expect("native refusal");
    assert_eq!(actual, expected);
    actual
}
fn first_binding(source: &StatefulCircuit) -> &LocalBinding {
    match &source.return_value {
        StateReturn::Effectful {
            body: ReturnPlan::Let { bindings, .. },
        } => &bindings[0],
        _ => {
            let StateAction::Let { bindings, .. } = &source.actions[0] else {
                panic!("fixture")
            };
            &bindings[0]
        }
    }
}

#[test]
fn both_name_producers_report_actual_binding_before_competing_bad_initializer() {
    for source in [
        plans(vec![binding("crate", parameter("later_initializer"))]),
        actions(vec![binding("crate", parameter("later_initializer"))]),
    ] {
        let original = first_binding(&source);
        let trace = Trace::default();
        assert!(
            matches!(same_error(&source, &trace), RenderError::InvalidIdentifier(ref name) if name == "crate")
        );
        assert_eq!(
            trace.0.borrow().as_slice(),
            &[(
                "binding",
                std::ptr::from_ref(original) as usize,
                Slot::Field("name")
            )]
        );
    }
}

#[test]
fn earlier_initializer_failure_keeps_first_leaf_and_never_observes_later_name() {
    for source in [
        plans(vec![
            binding("first", parameter("missing_first")),
            binding("crate", literal()),
        ]),
        actions(vec![
            binding("first", parameter("missing_first")),
            binding("crate", literal()),
        ]),
    ] {
        let original = &first_binding(&source).value;
        let trace = Trace::default();
        assert!(
            matches!(same_error(&source, &trace), RenderError::UnknownParameter(ref name) if name == "missing_first")
        );
        assert_eq!(
            trace.0.borrow().first(),
            Some(&("expr", std::ptr::from_ref(original) as usize, Slot::Node))
        );
        assert!(
            trace
                .0
                .borrow()
                .iter()
                .all(|(kind, _, _)| *kind != "binding")
        );
    }
}

#[test]
fn action_local_scope_does_not_escape_to_a_later_sibling() {
    let mut source = actions(vec![binding("local_only", literal())]);
    source.actions.push(StateAction::Expression {
        value: parameter("local_only"),
    });
    let trace = Trace::default();
    assert!(
        matches!(same_error(&source, &trace), RenderError::UnknownParameter(ref name) if name == "local_only")
    );
    assert!(
        trace
            .0
            .borrow()
            .iter()
            .all(|(kind, _, _)| *kind != "binding")
    );
}

#[test]
fn successful_original_shadow_scope_keeps_complete_item_and_has_no_phantom_site() {
    let mut action = actions(vec![binding("seed", parameter("seed"))]);
    let StateAction::Let { action: inner, .. } = &mut action.actions[0] else {
        panic!("fixture")
    };
    **inner = StateAction::Expression {
        value: parameter("seed"),
    };
    let mut plan = plans(vec![binding("seed", parameter("seed"))]);
    let StateReturn::Effectful {
        body: ReturnPlan::Let { result, .. },
    } = &mut plan.return_value
    else {
        panic!("fixture")
    };
    **result = ReturnPlan::Value {
        value: parameter("seed"),
    };
    for mut source in [action, plan] {
        source.parameters = vec![Parameter {
            name: "seed".into(),
            ty: Type::Field,
        }];
        let expected = run(&source, None).unwrap();
        let trace = Trace::default();
        let actual = run(&source, Some(&trace)).unwrap();
        assert_eq!(
            quote::quote!(#actual).to_string(),
            quote::quote!(#expected).to_string()
        );
        assert!(trace.0.borrow().is_empty());
    }
}
