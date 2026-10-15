use super::*;

// Whole-contract budget preflight deliberately has no occurrence at this edge.
// Keep the VM semantic Trace strict; authenticate this separate unavailable site.
#[derive(Default)]
struct UnavailablePreflight(RefCell<Vec<Slot>>);
impl RefusalObserver for UnavailablePreflight {
    fn refused(&self, site: RefusalSite<'_>) {
        assert!(
            site.owner.is_none(),
            "budget edge must remain explicitly unavailable"
        );
        self.0.borrow_mut().push(site.slot);
    }
}

fn fixture(depth: usize, malformed: bool) -> Contract {
    let mut value = field(if malformed { "bad" } else { "1" });
    for _ in 0..depth {
        value = Expr::Add {
            left: Box::new(value),
            right: Box::new(field("1")),
        };
    }
    Contract {
        schema_version: SCHEMA_VERSION,
        type_aliases: vec![],
        witnesses: vec![],
        circuits: vec![],
        stateful_circuits: vec![],
        ledger_fields: vec![ledger("stored", LedgerFieldKind::Cell { ty: Type::Field })],
        constructor: Some(Constructor {
            source: None,
            parameters: vec![],
            steps: vec![ConstructorStep::Sequence {
                steps: vec![ConstructorStep::CellWrite {
                    field: "stored".into(),
                    index: 0,
                    value,
                }],
            }],
        }),
    }
}
fn leaf(source: &Contract) -> &Expr {
    let ConstructorStep::Sequence { steps } = &source.constructor.as_ref().unwrap().steps[0] else {
        panic!("fixture Sequence")
    };
    let ConstructorStep::CellWrite { value, .. } = &steps[0] else {
        panic!("fixture Cell")
    };
    let mut value = value;
    while let Expr::Add { left, .. } = value {
        value = left;
    }
    value
}

#[test]
fn full_vm_constructor_observer_keeps_actual_default_two_mib_boundary() {
    let depth = (1..=64)
        .take_while(|depth| resource_limits::validate(&fixture(*depth, false)).is_ok())
        .last()
        .expect("small VM constructor source admitted");
    assert!(
        (3..48).contains(&depth),
        "unexpected DEFAULT VM source depth {depth}"
    );
    std::thread::Builder::new().stack_size(2 * 1024 * 1024).spawn(move || {
        let near = fixture(depth, false);
        let _preflight = resource_limits::validate(&near).unwrap();
        let trace = Trace::default();
        let observed = render_with_native_refusal(&near, None, Some(&trace)).unwrap();
        let bare = render_with_capabilities(&near).unwrap();
        assert_eq!(observed.source, bare.source);
        assert_eq!(serde_json::to_value(&observed.capabilities).unwrap(),
            serde_json::to_value(&bare.capabilities).unwrap());
        assert!(trace.0.borrow().is_empty());

        let over = fixture(depth + 1, false);
        let expected = resource_limits::validate(&over).err().expect("one-over source refused");
        let unavailable = UnavailablePreflight::default();
        assert_eq!(render_with_native_refusal(&over, None, Some(&unavailable)).err().unwrap(), expected);
        assert_eq!(unavailable.0.borrow().as_slice(), &[Slot::Node]);
        assert_eq!(render_with_capabilities(&over).err().unwrap(), expected);

        let malformed = fixture(depth, true);
        let _preflight = resource_limits::validate(&malformed).unwrap();
        let original = leaf(&malformed);
        let trace = Trace::default();
        let actual = render_with_native_refusal(&malformed, None, Some(&trace)).err().unwrap();
        assert_eq!(actual, render_with_capabilities(&malformed).err().unwrap());
        assert_eq!(actual, RenderError::InvalidFieldLiteral("bad".into()));
        first_expr(&trace, original);
        eprintln!("Full VM constructor Some/None DEFAULT depth{depth} admitted, depth{} refused, malformed original leaf captured on2MiB", depth + 1);
    }).unwrap().join().unwrap();
}
