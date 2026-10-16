use super::*;

#[test]
fn lexical_prepass_schedule_stops_on_the_same_first_failure_or_witness() {
    let first = loop_action("first");
    let later = loop_action("later");
    let sources = HashMap::from([("first", &first), ("later", &later)]);
    let binding = Parameter {
        name: "item".into(),
        ty: Type::Field,
    };
    let failing = vec![
        ConstructorStep::Let {
            bindings: vec![
                LocalBinding {
                    name: "crate".into(),
                    ty: Type::Field,
                    value: call("first"),
                },
                LocalBinding {
                    name: "later".into(),
                    ty: Type::Field,
                    value: witness(),
                },
            ],
            step: Box::new(expression(call("later"))),
        },
        ConstructorStep::Sequence {
            steps: vec![expression(call("first")), expression(witness())],
        },
        ConstructorStep::MapInsert {
            field: "unused".into(),
            index: 99,
            key: call("first"),
            value: call("later"),
        },
        ConstructorStep::ForEach {
            binding: binding.clone(),
            values: vec![call("first"), witness()],
            steps: vec![expression(call("later"))],
        },
        ConstructorStep::ForEachVector {
            binding: binding.clone(),
            source: call("first"),
            length: 99,
            steps: vec![expression(call("later"))],
        },
        ConstructorStep::If {
            condition: call("first"),
            then_steps: vec![expression(witness())],
            otherwise_steps: vec![expression(call("later"))],
        },
        ConstructorStep::If {
            condition: Expr::Boolean { value: false },
            then_steps: vec![expression(call("first"))],
            otherwise_steps: vec![expression(call("later"))],
        },
        ConstructorStep::Assert {
            condition: call("first"),
            message: "unused".into(),
        },
    ];
    for step in &failing {
        let trace = Trace::default();
        assert_eq!(
            same(step, &sources, &trace),
            Err(RenderError::UnsupportedStatefulCall("first".into()))
        );
        action_site(&trace, &first.actions[0]);
    }
    let witnessed = vec![
        ConstructorStep::Sequence {
            steps: vec![expression(witness()), expression(call("later"))],
        },
        ConstructorStep::ForEach {
            binding: binding.clone(),
            values: vec![witness()],
            steps: vec![expression(call("later"))],
        },
        ConstructorStep::ForEachVector {
            binding,
            source: witness(),
            length: 1,
            steps: vec![expression(call("later"))],
        },
        ConstructorStep::If {
            condition: Expr::Boolean { value: false },
            then_steps: vec![expression(witness())],
            otherwise_steps: vec![expression(call("later"))],
        },
    ];
    for step in &witnessed {
        let trace = Trace::default();
        assert_eq!(same(step, &sources, &trace), Ok(true));
        assert!(trace.0.borrow().is_empty());
    }
}

#[test]
fn outer_constructor_accumulation_keeps_later_step_errors_after_an_earlier_witness() {
    let cycle = loop_action("loop");
    let sources = HashMap::from([("loop", &cycle)]);
    let steps = [expression(witness()), expression(call("loop"))];
    // Exact original outer |= schedule, exercising actual prepass owners; this
    // private seam does not claim a newly reachable public constructor cycle.
    let old = (|| {
        let mut required = false;
        for step in &steps {
            required |= constructor_step_uses_witness(step, &sources)?;
        }
        Ok::<_, RenderError>(required)
    })();
    let trace = Trace::default();
    let observed = (|| {
        let mut required = false;
        for step in &steps {
            required |= constructor_step_uses_witness_observed(step, &sources, Some(&trace))?;
        }
        Ok::<_, RenderError>(required)
    })();
    assert_eq!(observed, old);
    assert_eq!(
        observed,
        Err(RenderError::UnsupportedStatefulCall("loop".into()))
    );
    action_site(&trace, &cycle.actions[0]);
}

#[test]
fn public_render_keeps_stateful_graph_priority_and_successful_constructor_output() {
    let source = contract(
        vec![expression(Expr::Parameter {
            name: "later".into(),
        })],
        vec![loop_action("loop")],
    );
    let _preflight = resource_limits::validate(&source).unwrap();
    let bare = render_with_capabilities(&source)
        .err()
        .expect("original graph refusal");
    let trace = Trace::default();
    let observed = render_with_native_refusal(&source, None, Some(&trace))
        .err()
        .expect("same graph refusal");
    assert_eq!(observed, bare);
    assert_eq!(
        observed,
        RenderError::UnsupportedStatefulCall("loop".into())
    );
    action_site(&trace, &source.stateful_circuits[0].actions[0]);

    for steps in [
        vec![],
        vec![expression(Expr::Unit)],
        vec![ConstructorStep::Sequence {
            steps: vec![expression(Expr::Unit)],
        }],
    ] {
        let source = contract(steps, vec![]);
        let bare = render_with_capabilities(&source).unwrap();
        let trace = Trace::default();
        let observed = render_with_native_refusal(&source, None, Some(&trace)).unwrap();
        assert_eq!(observed.source, bare.source);
        assert_eq!(
            serde_json::to_value(&observed.capabilities).unwrap(),
            serde_json::to_value(&bare.capabilities).unwrap()
        );
        assert!(
            trace.0.borrow().is_empty(),
            "successful discovery and assembly are silent"
        );
    }
}
