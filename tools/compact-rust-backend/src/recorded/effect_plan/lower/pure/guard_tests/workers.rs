use super::*;
fn fixture(count: usize) -> Contract {
    let circuits = (0..count)
        .map(|i| PureCircuit {
            source: None,
            name: format!("helper{i}"),
            internal: true,
            parameters: vec![],
            result: Type::Field,
            body: if i + 1 < count {
                call(&format!("helper{}", i + 1), vec![])
            } else {
                field("1")
            },
        })
        .collect();
    Contract {
        schema_version: crate::ir::SCHEMA_VERSION,
        type_aliases: vec![],
        ledger_fields: vec![],
        constructor: None,
        witnesses: vec![],
        circuits,
        stateful_circuits: vec![root(call("helper0", vec![]), Type::Field)],
    }
}
fn render(
    source: &Contract,
    trace: Option<&Trace>,
) -> Result<crate::RenderedContract, crate::RenderError> {
    let observer = trace.map(|trace| trace as &dyn RefusalObserver);
    crate::render_with_refusals(source, None, observer, observer)
}
#[test]
fn full_native_and_recorded_pure_chain_keeps_default_two_mib_near_over_and_child() {
    let near = (1..=24)
        .take_while(|count| crate::resource_limits::validate(&fixture(*count)).is_ok())
        .last()
        .unwrap();
    assert!((2..=16).contains(&near), "actual DEFAULT near chain {near}");
    std::thread::Builder::new().stack_size(2*1024*1024).spawn(move || {
        let source = fixture(near); let trace = Trace::default(); let observed = render(&source, Some(&trace)).unwrap(); let bare = render(&source, None).unwrap(); assert_eq!(observed.source, bare.source); assert_eq!(serde_json::to_value(&observed.capabilities).unwrap(), serde_json::to_value(&bare.capabilities).unwrap()); assert!(observed.capabilities.circuits.iter().find(|c| c.name=="root").unwrap().recorded); assert!(trace.events.borrow().is_empty()); assert!(trace.related.borrow().is_empty());
        let over = fixture(near+1); let trace = Trace::default(); let observed = render(&over, Some(&trace)).err().expect("one over DEFAULT"); assert_eq!(observed, render(&over, None).err().expect("same old refusal")); assert!(matches!(observed, crate::RenderError::ResourceLimit { .. }));
        let mut malformed = fixture(near); malformed.circuits[0].body = Expr::Add { left: Box::new(call("helper1", vec![])), right: Box::new(field("bad")) }; let Expr::Add { right, .. } = &malformed.circuits[0].body else { panic!("fixture") };
        let trace = Trace::default(); let observed = render(&malformed, Some(&trace)).err().expect("original child"); assert_eq!(observed, render(&malformed, None).err().expect("same old child")); assert_eq!(observed, crate::RenderError::InvalidFieldLiteral("bad".into())); primary(&trace, expr_site(right));
        eprintln!("Full pure-call DEFAULT chain{near} accepted/recorded, chain{} refused, malformed original child preserved on2MiB", near+1);
    }).unwrap().join().unwrap();
}
