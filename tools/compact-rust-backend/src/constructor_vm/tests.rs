//! Direct original-IR controls, supplemented by the complete old-owner inverse.
use crate::ir::{Constructor, LedgerField, LocalBinding, Parameter};
use crate::source_diagnostics::Slot;
use crate::*;
use std::cell::RefCell;

mod lexical_control_guards;
mod operands;
mod scope;
mod workers;

#[derive(Default)]
struct Trace(RefCell<Vec<(&'static str, usize, Slot)>>);
impl RefusalObserver for Trace {
    fn refused(&self, site: RefusalSite<'_>) {
        let (kind, pointer) = match site.owner {
            Some(Owner::Expr(value)) => ("expr", std::ptr::from_ref(value) as usize),
            Some(Owner::Binding(value)) => ("binding", std::ptr::from_ref(value) as usize),
            Some(Owner::ConstructorStep(value)) => ("step", std::ptr::from_ref(value) as usize),
            Some(Owner::CounterAmount(value)) => ("amount", std::ptr::from_ref(value) as usize),
            _ => panic!("invented VM constructor source owner"),
        };
        self.0.borrow_mut().push((kind, pointer, site.slot));
    }
}
fn field(value: &str) -> Expr {
    Expr::FieldLiteral {
        value: value.into(),
    }
}
fn parameter(name: &str) -> Expr {
    Expr::Parameter { name: name.into() }
}
fn ledger(id: &str, declaration: LedgerFieldKind) -> LedgerField {
    LedgerField {
        source: None,
        id: id.into(),
        index: 0,
        path: vec![0],
        declaration,
    }
}
fn first_expr(trace: &Trace, expected: &Expr) {
    assert_eq!(
        trace.0.borrow().first(),
        Some(&("expr", std::ptr::from_ref(expected) as usize, Slot::Node))
    );
}
fn run<'a>(
    steps: &'a [ConstructorStep],
    fields: &HashMap<&str, &LedgerField>,
    parameters: &HashMap<&'a str, (&'a Type, syn::Ident)>,
    circuits: &HashMap<&str, &PureCircuit>,
    observer: Option<&dyn RefusalObserver>,
) -> Result<(String, usize, usize), RenderError> {
    let witnesses = HashMap::new();
    let stateful = HashMap::new();
    let mut next_loop = 7;
    let mut next_temp = 9;
    let statements = if observer.is_some() {
        let environment = stateful::NativeExpressionContext {
            witnesses: &witnesses,
            circuits,
            stateful_circuits: &stateful,
            ledger_fields: fields,
            observer,
        };
        render_constructor_vm_steps_observed(
            steps,
            parameters,
            &environment,
            super::Output {
                next_loop: &mut next_loop,
                next_temp: &mut next_temp,
            },
        )?
    } else {
        render_constructor_vm_steps(
            steps,
            fields,
            parameters,
            &witnesses,
            circuits,
            &stateful,
            &mut next_loop,
            &mut next_temp,
        )?
    };
    Ok((
        quote::quote!(#(#statements)*).to_string(),
        next_loop,
        next_temp,
    ))
}
fn same_error(
    steps: &[ConstructorStep],
    fields: &HashMap<&str, &LedgerField>,
    trace: &Trace,
) -> RenderError {
    let bare = run(steps, fields, &HashMap::new(), &HashMap::new(), None).unwrap_err();
    let observed = run(steps, fields, &HashMap::new(), &HashMap::new(), Some(trace)).unwrap_err();
    assert_eq!(observed, bare);
    observed
}
