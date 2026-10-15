//! Actual first graph edges, compared to the literal H136 graph owner.
use super::*;
use crate::ir::{Contract, SCHEMA_VERSION};
use crate::source_diagnostics::Slot;
use std::cell::RefCell;

mod edges;
mod native;
mod old_reference;

#[derive(Clone, Copy)]
enum Kind {
    Witness,
    Output,
}
type CapturedGraphSite = (Option<(&'static str, usize)>, Slot);
#[derive(Default)]
struct Trace(RefCell<Vec<CapturedGraphSite>>);
impl RefusalObserver for Trace {
    fn refused(&self, site: RefusalSite<'_>) {
        let owner = match site.owner {
            Some(Owner::Expr(source)) => Some(("expr", std::ptr::from_ref(source) as usize)),
            Some(Owner::Action(source)) => Some(("action", std::ptr::from_ref(source) as usize)),
            None => None,
            _ => panic!("no invented graph source owner"),
        };
        self.0.borrow_mut().push((owner, site.slot));
    }
    fn related(&self, _: RefusalSite<'_>) {
        panic!("this slice does not invent related edges");
    }
}
fn circuit(name: &str, actions: Vec<StateAction>) -> StatefulCircuit {
    StatefulCircuit {
        source: None,
        name: name.into(),
        internal: false,
        parameters: vec![],
        actions,
        result: crate::ir::Type::Unit,
        return_value: StateReturn::Unit,
    }
}
fn call(name: &str) -> Expr {
    Expr::Call {
        name: name.into(),
        arguments: vec![],
    }
}
fn action(name: &str) -> StateAction {
    StateAction::CircuitCall {
        name: name.into(),
        arguments: vec![],
    }
}
fn same<'source>(
    kind: Kind,
    root: &'source StatefulCircuit,
    sources: &HashMap<&str, &'source StatefulCircuit>,
    seed: &HashSet<String>,
    trace: &Trace,
) -> Result<bool, RenderError> {
    let mut old_visiting = seed.clone();
    let old = match kind {
        Kind::Witness => old_reference::circuit_uses_witness(root, sources, &mut old_visiting),
        Kind::Output => {
            old_reference::circuit_emits_native_private_output(root, sources, &mut old_visiting)
        }
    };
    let mut bare_visiting = seed.clone();
    let bare = match kind {
        Kind::Witness => circuit_uses_witness(root, sources, &mut bare_visiting),
        Kind::Output => circuit_emits_native_private_output(root, sources, &mut bare_visiting),
    };
    let mut observed_visiting = seed.clone();
    let observed = match kind {
        Kind::Witness => {
            circuit_uses_witness_observed(root, sources, &mut observed_visiting, None, Some(trace))
        }
        Kind::Output => circuit_emits_native_private_output_observed(
            root,
            sources,
            &mut observed_visiting,
            None,
            Some(trace),
        ),
    };
    assert_eq!(bare, old);
    assert_eq!(observed, old);
    assert_eq!(bare_visiting, old_visiting);
    assert_eq!(observed_visiting, old_visiting);
    observed
}
fn action_site(trace: &Trace, source: &StateAction) {
    assert_eq!(
        trace.0.borrow().as_slice(),
        &[(
            Some(("action", std::ptr::from_ref(source) as usize)),
            Slot::Field("name")
        )]
    );
}
fn expr_site(trace: &Trace, source: &Expr) {
    assert_eq!(
        trace.0.borrow().as_slice(),
        &[(
            Some(("expr", std::ptr::from_ref(source) as usize)),
            Slot::Field("name")
        )]
    );
}
