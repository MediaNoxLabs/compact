//! Live test issuer + borrowed-index native callback on an unchanged 2 MiB worker.
use super::{Sandbox, address, body, diagnostic, failure, field, missing, pure, raw};
use crate::{NativeFailureCause, RenderError, RenderOptions, ir::Contract};
use serde_json::{Value, json};

fn fixture(depth: usize, missing_leaf: bool) -> Value {
    let mut expression = if missing_leaf {
        missing("leaf_missing")
    } else {
        field()
    };
    for _ in 0..depth {
        expression = json!({"kind":"subtract","left":field(),"right":expression});
    }
    pure(expression)
}

#[test]
fn issued_pure_body_callback_keeps_actual_default_boundary_on_two_mib_worker() {
    std::thread::Builder::new().stack_size(2 * 1024 * 1024).spawn(|| {
        let depth = (1..=64).take_while(|depth| {
            let contract: Contract = serde_json::from_value(fixture(*depth, false)).unwrap();
            crate::resource_limits::validate(&contract).is_ok()
        }).last().expect("small Field source admitted");
        assert!((3..48).contains(&depth), "unexpected default depth {depth}");
        let ir = fixture(depth, false);
        let contract: Contract = serde_json::from_value(ir.clone()).unwrap();
        let expected = crate::render(&contract).expect("actual public near boundary");
        let seed = Sandbox::new(&ir,None,None,0);
        let metadata = raw(&ir,&seed.source);
        let sandbox = Sandbox::new(&ir,Some(&serde_json::to_vec(&metadata).unwrap()),None,0);
        let rendered = sandbox.invoke().render(RenderOptions::default()).expect("enabled issued callback near boundary");
        assert_eq!(rendered.rendered().source, expected);
        rendered.finish_metadata().unwrap();

        let over = fixture(depth+1,false);
        let contract: Contract = serde_json::from_value(over.clone()).unwrap();
        assert!(matches!(crate::resource_limits::validate(&contract),Err(RenderError::ResourceLimit { .. })));
        let metadata = raw(&over,&seed.source);
        let sandbox = Sandbox::new(&over,Some(&serde_json::to_vec(&metadata).unwrap()),None,0);
        let refused = failure(&sandbox);
        assert!(matches!(refused.cause,NativeFailureCause::Render(RenderError::ResourceLimit { .. })));
        assert!(refused.diagnostic.is_none(), "preflight has no pure Expr hook");

        let ir = fixture(depth,true);
        let contract: Contract = serde_json::from_value(ir.clone()).unwrap();
        crate::resource_limits::validate(&contract).unwrap().finish(&contract).unwrap();
        let metadata = raw(&ir,&seed.source);
        let mut path = body();
        for _ in 0..depth { path.push(json!({"field":"right"})); }
        let leaf = address(&metadata,&path);
        let sandbox = Sandbox::new(&ir,Some(&serde_json::to_vec(&metadata).unwrap()),None,0);
        let refused = failure(&sandbox);
        assert_eq!(diagnostic(&refused).address_id,leaf);
        assert!(matches!(refused.cause,NativeFailureCause::Render(RenderError::UnknownParameter(ref name)) if name=="leaf_missing"));
        eprintln!("Issued pure callback worker: depth {depth} accepted; {} preflight refused; exact leaf diagnostic",depth+1);
    }).unwrap().join().unwrap();
}
