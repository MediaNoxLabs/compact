use super::*;

#[test]
fn prepass_cycle_keeps_actual_callee_closing_action_or_expression() {
    let action_loop = loop_action("loop");
    let sources = HashMap::from([("loop", &action_loop)]);
    let step = expression(Expr::Call {
        name: "loop".into(),
        arguments: vec![Expr::FieldLiteral {
            value: "bad".into(),
        }],
    });
    let trace = Trace::default();
    assert_eq!(
        same(&step, &sources, &trace),
        Err(RenderError::UnsupportedStatefulCall("loop".into()))
    );
    action_site(&trace, &action_loop.actions[0]);

    let original = call("loop");
    let expr_loop = circuit("loop", vec![], StateReturn::Expression { value: original });
    let sources = HashMap::from([("loop", &expr_loop)]);
    let trace = Trace::default();
    assert_eq!(
        same(&step, &sources, &trace),
        Err(RenderError::UnsupportedStatefulCall("loop".into()))
    );
    let StateReturn::Expression { value } = &expr_loop.return_value else {
        panic!("fixture")
    };
    expr_site(&trace, value);
    let ConstructorStep::Expression { value: incoming } = &step else {
        panic!("fixture")
    };
    assert_ne!(std::ptr::from_ref(value), std::ptr::from_ref(incoming));
}

#[test]
fn intrinsic_witness_precedes_graph_even_when_cycle_occurs_first_in_expression() {
    let cycle = loop_action("loop");
    let sources = HashMap::from([("loop", &cycle)]);
    let step = expression(Expr::Tuple {
        elements: vec![call("loop"), witness()],
    });
    let trace = Trace::default();
    assert_eq!(same(&step, &sources, &trace), Ok(true));
    assert!(
        trace.0.borrow().is_empty(),
        "old intrinsic-witness shortcut never enters cyclic graph"
    );

    let step = ConstructorStep::MapInsert {
        field: "not_checked_in_prepass".into(),
        index: 99,
        key: witness(),
        value: call("loop"),
    };
    let trace = Trace::default();
    assert_eq!(same(&step, &sources, &trace), Ok(true));
    assert!(
        trace.0.borrow().is_empty(),
        "key witness short-circuits value exactly as before"
    );
}

#[test]
fn transitive_success_and_unknown_calls_are_silent_and_equal_occurrences_stay_distinct() {
    let witnessed = circuit(
        "witnessed",
        vec![StateAction::Expression { value: witness() }],
        StateReturn::Unit,
    );
    let helper = circuit(
        "helper",
        vec![StateAction::CircuitCall {
            name: "witnessed".into(),
            arguments: vec![],
        }],
        StateReturn::Unit,
    );
    let sources = HashMap::from([("helper", &helper), ("witnessed", &witnessed)]);
    let trace = Trace::default();
    assert_eq!(
        same(&expression(call("helper")), &sources, &trace),
        Ok(true)
    );
    assert_eq!(
        same(&expression(call("unknown")), &sources, &trace),
        Ok(false)
    );
    assert!(trace.0.borrow().is_empty());

    let original = loop_action("loop");
    let equal_sources = [original.clone(), original];
    let mut ids = Vec::new();
    for source in &equal_sources {
        let trace = Trace::default();
        assert_eq!(
            same(
                &expression(call("loop")),
                &HashMap::from([("loop", source)]),
                &trace
            ),
            Err(RenderError::UnsupportedStatefulCall("loop".into()))
        );
        action_site(&trace, &source.actions[0]);
        ids.push(std::ptr::from_ref(&source.actions[0]));
    }
    assert_ne!(ids[0], ids[1]);
}
