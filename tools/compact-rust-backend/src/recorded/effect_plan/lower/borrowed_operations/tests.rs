//! Private lowerer controls: unchanged gaps, partial SSA, fuel and lexical inputs.
use super::*;
use crate::source_diagnostics::Slot;
use std::cell::RefCell;
mod identity;
mod old_reference;
mod parity;

type Event = (Option<(&'static str, usize)>, Slot);
#[derive(Default)]
struct Trace {
    primary: RefCell<Option<Event>>,
    events: RefCell<Vec<Event>>,
}
fn event(site: RefusalSite<'_>) -> Event {
    let owner = site.owner.map(|owner| match owner {
        Owner::Expr(value) => ("expr", std::ptr::from_ref(value) as usize),
        Owner::Action(value) => ("action", std::ptr::from_ref(value) as usize),
        Owner::StateReturn(value) => ("return", std::ptr::from_ref(value) as usize),
        Owner::Type(value) => ("type", std::ptr::from_ref(value) as usize),
        _ => panic!("unexpected synthetic-operation source owner"),
    });
    (owner, site.slot)
}
impl RefusalObserver for Trace {
    fn refused(&self, site: RefusalSite<'_>) {
        let event = event(site);
        if self.primary.borrow().is_none() {
            *self.primary.borrow_mut() = Some(event);
        }
        self.events.borrow_mut().push(event);
    }
}
type Outcome = (Result<String, RecordingGap>, usize, usize, String);
enum Input<'s> {
    Action(&'s StateAction),
    Return(&'s StateReturn),
}
fn ledger() -> Vec<LedgerField> {
    [
        ("cell", LedgerFieldKind::Cell { ty: Type::Field }),
        ("counter", LedgerFieldKind::Counter),
        ("set", LedgerFieldKind::Set { ty: Type::Field }),
    ]
    .into_iter()
    .enumerate()
    .map(|(index, (id, declaration))| LedgerField {
        id: id.into(),
        index: index as u8,
        path: vec![],
        source: None,
        declaration,
    })
    .collect()
}
fn run(
    input: Input<'_>,
    fields: &[LedgerField],
    observer: Option<&dyn RefusalObserver>,
    old: bool,
    before_limit: Option<usize>,
) -> Outcome {
    let registry = fields.iter().map(|f| (f.id.as_str(), f)).collect();
    let witnesses = HashMap::new();
    let pure = HashMap::new();
    let circuits = HashMap::new();
    let declarations = Declarations::with_calls(&registry, &witnesses, &pure, &circuits);
    let (max_call_depth, max_expanded_steps) = crate::resource_limits::recording_call_limits();
    let mut lower = Lowerer {
        observer,
        declarations: &declarations,
        circuits: &circuits,
        pure: Certifier::new(&pure, &circuits),
        call_stack: vec!["root".into()],
        expanded_steps: before_limit.map_or(0, |remaining| max_expanded_steps - remaining),
        max_call_depth,
        max_expanded_steps,
        next: 17,
        root_action: None,
        failure_precision: Precision::Unclassified,
        concrete_node: None,
    };
    let mut bindings = vec![];
    let scope = Scope::new();
    let result = match input {
        Input::Action(action) if old => lower.spend("probe").and_then(|()| {
            let StateAction::Assert { condition, message } = action else {
                panic!("fixture")
            };
            lower
                .old_synthetic_expression(
                    &Expr::Assert {
                        condition: Box::new(condition.clone()),
                        message: message.clone(),
                    },
                    &scope,
                    &mut bindings,
                    "probe",
                )
                .map(|_| "unit".into())
        }),
        Input::Action(action) => lower
            .action(action, &scope, &mut bindings, "probe")
            .map(|()| "unit".into()),
        Input::Return(returned) if old => {
            let expression = match returned {
                StateReturn::CellRead { field, index } => Expr::CellRead {
                    field: field.clone(),
                    index: *index,
                },
                StateReturn::CounterRead { field, index } => Expr::CounterRead {
                    field: field.clone(),
                    index: *index,
                },
                StateReturn::SetSize { field, index } => Expr::SetSize {
                    field: field.clone(),
                    index: *index,
                },
                StateReturn::SetIsEmpty { field, index } => Expr::SetIsEmpty {
                    field: field.clone(),
                    index: *index,
                },
                _ => panic!("fixture"),
            };
            lower
                .old_synthetic_expression(&expression, &scope, &mut bindings, "probe.return_value")
                .map(|value| format!("{value:?}"))
        }
        Input::Return(returned) => lower
            .ordinary(&[], returned, &Type::Unit, &scope, &mut bindings, "probe")
            .map(|value| format!("{value:?}")),
    };
    (
        result,
        lower.next,
        lower.expanded_steps,
        format!("{bindings:?}"),
    )
}
fn assertion(condition: Expr) -> StateAction {
    StateAction::Assert {
        condition,
        message: "original message".into(),
    }
}
fn returns(field: &str, index: u8) -> [StateReturn; 4] {
    [
        StateReturn::CellRead {
            field: field.into(),
            index,
        },
        StateReturn::CounterRead {
            field: field.into(),
            index,
        },
        StateReturn::SetSize {
            field: field.into(),
            index,
        },
        StateReturn::SetIsEmpty {
            field: field.into(),
            index,
        },
    ]
}

mod workers;
