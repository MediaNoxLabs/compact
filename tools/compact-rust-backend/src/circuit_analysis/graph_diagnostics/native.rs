use super::*;

#[test]
fn actual_native_root_cycle_precedes_arity_without_guessing_caller() {
    let root = circuit(
        "loop",
        vec![StateAction::CircuitCall {
            name: "loop".into(),
            arguments: vec![Expr::Parameter {
                name: "later".into(),
            }],
        }],
    );
    let sources = HashMap::from([("loop", &root)]);
    let bare = crate::stateful::render_stateful_circuit_observed(
        &root,
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &sources,
        None,
    )
    .err()
    .expect("old graph failure");
    let trace = Trace::default();
    let observed = crate::stateful::render_stateful_circuit_observed(
        &root,
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &sources,
        Some(&trace),
    )
    .err()
    .expect("actual graph failure");
    assert_eq!(observed, bare);
    assert_eq!(
        observed,
        RenderError::UnsupportedStatefulCall("loop".into())
    );
    action_site(&trace, &root.actions[0]);
}

#[test]
fn full_native_graph_discovery_keeps_default_two_mib_source_boundary() {
    fn fixture(depth: usize) -> Contract {
        let mut value = call("loop");
        for _ in 0..depth {
            value = Expr::Tuple {
                elements: vec![value],
            };
        }
        Contract {
            schema_version: SCHEMA_VERSION,
            type_aliases: vec![],
            constructor: None,
            witnesses: vec![],
            ledger_fields: vec![],
            circuits: vec![],
            stateful_circuits: vec![circuit("loop", vec![StateAction::Expression { value }])],
        }
    }
    fn original_leaf(source: &Contract) -> &Expr {
        let StateAction::Expression { value } = &source.stateful_circuits[0].actions[0] else {
            panic!("fixture")
        };
        let mut value = value;
        while let Expr::Tuple { elements } = value {
            value = &elements[0];
        }
        value
    }
    let depth = (1..=64)
        .take_while(|depth| crate::resource_limits::validate(&fixture(*depth)).is_ok())
        .last()
        .expect("bounded graph source admitted");
    assert!((3..48).contains(&depth));
    std::thread::Builder::new().stack_size(2 * 1024 * 1024).spawn(move || {
        let near = fixture(depth);
        let _preflight = crate::resource_limits::validate(&near).unwrap();
        let trace = Trace::default();
        let observed = crate::render_with_native_refusal(&near, None, Some(&trace)).err().expect("cycle failure");
        let bare = crate::render_with_capabilities(&near).err().expect("same old cycle failure");
        assert_eq!(observed, bare);
        assert_eq!(observed, RenderError::UnsupportedStatefulCall("loop".into()));
        expr_site(&trace, original_leaf(&near));
        let over = fixture(depth + 1);
        let expected = crate::resource_limits::validate(&over).err().expect("source one over refuses");
        assert!(matches!(expected, RenderError::ResourceLimit { .. }));
        assert_eq!(crate::render_with_capabilities(&over).err().unwrap(), expected);
        eprintln!("Full native graph source depth{depth} admitted before same cycle refusal; depth{} source-refused on ordinary2MiB", depth + 1);
    }).unwrap().join().unwrap();
}
