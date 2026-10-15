use super::*;

#[test]
fn root_interface_duplicate_and_result_guards_keep_original_headers() {
    // Opaque strings are supported by the existing public interface policy.
    let supported = circuit(
        "root",
        vec![parameter("opaque", Type::OpaqueString)],
        Expr::Unit,
        Type::Unit,
    );
    let supported_sources = registry(std::slice::from_ref(&supported));
    let supported_trace = Trace::default();
    assert_eq!(
        run(
            &supported,
            &supported_sources,
            &supported_sources,
            &HashMap::new(),
            Some(&supported_trace)
        )
        .unwrap(),
        run(
            &supported,
            &supported_sources,
            &supported_sources,
            &HashMap::new(),
            None
        )
        .unwrap()
    );
    assert!(supported_trace.events.borrow().is_empty());
    assert!(supported_trace.related.borrow().is_empty());
    // Canonical local tuples may be wider than the public FAB arity limit11.
    let root = circuit(
        "root",
        vec![parameter(
            "wide",
            Type::Tuple {
                elements: vec![Type::Field; 12],
            },
        )],
        Expr::Unit,
        Type::Unit,
    );
    let sources = registry(std::slice::from_ref(&root));
    let trace = Trace::default();
    let error = same_error(&root, &sources, &sources, &HashMap::new(), &trace);
    assert!(error.detail.contains("unsupported parameter"));
    primary(
        &trace,
        RefusalSite::node(Owner::Type(&root.parameters[0].ty)),
    );

    let root = circuit(
        "root",
        vec![parameter("x", Type::Field), parameter("x", Type::Field)],
        Expr::Unit,
        Type::Unit,
    );
    let sources = registry(std::slice::from_ref(&root));
    let trace = Trace::default();
    let error = same_error(&root, &sources, &sources, &HashMap::new(), &trace);
    assert!(error.detail.contains("duplicate parameter"));
    primary(
        &trace,
        RefusalSite::field(Owner::Parameter(&root.parameters[1]), "name"),
    );

    let root = circuit("root", vec![], Expr::Unit, named_unsupported());
    let sources = registry(std::slice::from_ref(&root));
    let trace = Trace::default();
    let error = same_error(&root, &sources, &sources, &HashMap::new(), &trace);
    assert!(error.detail.contains("unsupported circuit result"));
    primary(&trace, RefusalSite::node(Owner::Type(&root.result)));
}
