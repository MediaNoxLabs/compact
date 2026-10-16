use super::*;

fn fixture(depth: usize) -> Contract {
    let mut ty = product("Leaf", vec![member("field", Type::Field)]);
    for _ in 0..depth {
        ty = Type::Vector {
            length: 1,
            element: Box::new(ty),
        };
    }
    let mut source = contract();
    source.circuits.push(PureCircuit {
        source: None,
        name: "identity".into(),
        internal: false,
        parameters: vec![Parameter {
            name: "value".into(),
            ty: ty.clone(),
        }],
        result: ty,
        body: Expr::Parameter {
            name: "value".into(),
        },
    });
    source
}

#[test]
fn original_collection_keeps_actual_default_two_mib_near_and_over_boundaries() {
    let depth = (1..=64)
        .take_while(|depth| resource_limits::validate(&fixture(*depth)).is_ok())
        .last()
        .expect("bounded named source admitted");
    assert!((3..48).contains(&depth));
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(move || {
            let near = fixture(depth);
            let _preflight = resource_limits::validate(&near).unwrap();
            let bare = render_with_capabilities(&near).unwrap();
            let trace = Trace::default();
            let observed = render_with_native_refusal(&near, None, Some(&trace)).unwrap();
            assert_eq!(observed.source, bare.source);
            assert_eq!(
                serde_json::to_value(&observed.capabilities).unwrap(),
                serde_json::to_value(&bare.capabilities).unwrap()
            );
            assert!(trace.0.borrow().is_empty());
            let over = fixture(depth + 1);
            let expected = resource_limits::validate(&over)
                .err()
                .expect("one over public default");
            assert!(matches!(expected, RenderError::ResourceLimit { .. }));
            let trace = Trace::default();
            assert_eq!(same_public_error(&over, &trace), expected);
            assert!(
                !trace.0.borrow().is_empty(),
                "actual preflight source refusal"
            );
            eprintln!(
                "Named collection default depth{depth} rendered; depth{} refused on unchanged2MiB",
                depth + 1
            );
        })
        .unwrap()
        .join()
        .unwrap();
}
