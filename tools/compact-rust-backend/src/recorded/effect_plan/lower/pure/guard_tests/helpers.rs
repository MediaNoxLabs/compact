use super::*;
#[derive(Clone, Copy)]
pub(super) enum Input<'a> {
    Expression(&'a Expr),
    Dispatch(&'a Expr),
    Statement(&'a StateAction),
    InvalidId(&'a Expr),
}
#[derive(Clone, Copy)]
pub(super) enum Seed {
    Plain,
    AtDepth,
    AtWork,
}
pub(super) struct Env<'a> {
    pub snapshots: &'a [PureCircuit],
    pub live: &'a [PureCircuit],
    pub stateful: &'a [StatefulCircuit],
    pub seed: Seed,
    pub canonical: bool,
}
pub(super) fn env<'a>(snapshots: &'a [PureCircuit], live: &'a [PureCircuit]) -> Env<'a> {
    Env {
        snapshots,
        live,
        stateful: &[],
        seed: Seed::Plain,
        canonical: false,
    }
}
pub(super) fn field(value: &str) -> Expr {
    Expr::FieldLiteral {
        value: value.into(),
    }
}
pub(super) fn parameter(name: &str, ty: Type) -> Parameter {
    Parameter {
        name: name.into(),
        ty,
    }
}
pub(super) fn call(name: &str, arguments: Vec<Expr>) -> Expr {
    Expr::Call {
        name: name.into(),
        arguments,
    }
}
pub(super) fn pure(parameters: Vec<Parameter>, body: Expr, result: Type) -> PureCircuit {
    PureCircuit {
        source: None,
        name: "helper".into(),
        internal: true,
        parameters,
        result,
        body,
    }
}
pub(super) fn root(value: Expr, result: Type) -> StatefulCircuit {
    StatefulCircuit {
        source: None,
        name: "root".into(),
        internal: false,
        parameters: vec![],
        actions: vec![],
        result,
        return_value: StateReturn::Expression { value },
    }
}
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Outcome {
    pub result: Result<Value, RecordingGap>,
    pub next: usize,
    pub spent: usize,
    pub stack: Vec<String>,
    pub bindings: String,
    pub certificate: Option<(usize, usize)>,
}
pub(super) fn run(
    environment: &Env<'_>,
    input: Input<'_>,
    scope: &Scope,
    observer: Option<&dyn RefusalObserver>,
) -> Outcome {
    let snapshots = environment
        .snapshots
        .iter()
        .map(|c| (c.name.as_str(), c))
        .collect();
    let live = environment
        .live
        .iter()
        .map(|c| (c.name.as_str(), c))
        .collect();
    let stateful = environment
        .stateful
        .iter()
        .map(|c| (c.name.as_str(), c))
        .collect();
    let ledger = HashMap::new();
    let witnesses = HashMap::new();
    let mut declarations = Declarations::with_calls(&ledger, &witnesses, &snapshots, &stateful);
    if environment.canonical {
        let original_root = root(Expr::Unit, Type::Unit);
        let types = super::super::super::super::type_environment::TypeEnvironment::from_sources(
            &original_root,
            &ledger,
            &witnesses,
            &snapshots,
            &stateful,
        )
        .unwrap();
        declarations = declarations.with_type_environment(types);
    }
    let (max_call_depth, max_expanded_steps) = crate::resource_limits::recording_call_limits();
    let mut lower = Lowerer {
        observer,
        declarations: &declarations,
        circuits: &stateful,
        pure: Certifier::new(&live, &stateful),
        call_stack: if matches!(environment.seed, Seed::AtDepth) {
            vec!["outer".into(); max_call_depth]
        } else {
            vec!["outer".into()]
        },
        expanded_steps: if matches!(environment.seed, Seed::AtWork) {
            max_expanded_steps
        } else {
            0
        },
        max_call_depth,
        max_expanded_steps,
        next: 11,
        root_action: None,
        failure_precision: Precision::Unclassified,
        concrete_node: None,
    };
    let mut bindings = vec![];
    let result = match input {
        Input::Expression(source) => {
            let Expr::Call { name, arguments } = source else {
                panic!("fixture")
            };
            lower.call(
                CallSite::Expression(source),
                name,
                arguments,
                scope,
                &mut bindings,
                "probe",
            )
        }
        Input::Dispatch(source) => lower.expression(source, scope, &mut bindings, "probe"),
        Input::Statement(action) => lower
            .pure_statement(action, scope, &mut bindings, "probe")
            .map(|()| Value {
                id: usize::MAX,
                ty: Type::Unit,
            }),
        Input::InvalidId(source) => lower.pure_call(
            (CallId(usize::MAX), CallSite::Expression(source)),
            "helper",
            &[],
            scope,
            &mut bindings,
            "probe",
        ),
    };
    let certificate = lower
        .pure
        .get("helper")
        .map(|c| (c.call_depth(), c.expanded_work()));
    Outcome {
        result,
        next: lower.next,
        spent: lower.expanded_steps,
        stack: lower.call_stack,
        bindings: format!("{bindings:?}"),
        certificate,
    }
}
pub(super) fn error(environment: &Env<'_>, input: Input<'_>, trace: &Trace) -> Outcome {
    let observed = run(environment, input, &Scope::new(), Some(trace));
    assert_eq!(observed, run(environment, input, &Scope::new(), None));
    assert!(observed.result.is_err());
    observed
}
pub(super) fn detail(outcome: &Outcome, text: &str) {
    assert!(
        outcome.result.as_ref().unwrap_err().detail.contains(text),
        "expected {text:?}: {:?}",
        outcome.result
    );
}
pub(super) fn expr_site(source: &Expr) -> RefusalSite<'_> {
    RefusalSite::node(Owner::Expr(source))
}
pub(super) fn action_site(source: &StateAction) -> RefusalSite<'_> {
    RefusalSite::node(Owner::Action(source))
}

pub(super) fn checked(
    root: &StatefulCircuit,
    sources: &[PureCircuit],
    observer: Option<&dyn RefusalObserver>,
) -> Result<(String, String, usize), RecordingGap> {
    let pure = sources
        .iter()
        .map(|source| (source.name.as_str(), source))
        .collect();
    let stateful = HashMap::new();
    let ledger = HashMap::new();
    let witnesses = HashMap::new();
    let types = super::super::super::super::type_environment::TypeEnvironment::from_sources(
        root, &ledger, &witnesses, &pure, &stateful,
    )?;
    let declarations = Declarations::with_calls(&ledger, &witnesses, &pure, &stateful)
        .with_type_environment(types)
        .with_root(root);
    let (plan, certificates) = super::super::super::lower_with_pure_calls_observed(
        root,
        &declarations,
        &pure,
        &stateful,
        observer,
    )
    .map_err(LoweringRejection::into_checked_gap)?;
    let count = certificates.iter().count();
    let declarations = declarations.with_certificates(certificates, &pure)?;
    let checked = super::super::super::super::validate::check(plan, declarations)?;
    let output = super::super::super::super::render(&checked)?;
    let steps = output.steps;
    let result = output.result;
    Ok((
        format!("{:?}", checked.plan()),
        quote::quote!(#(#steps)* #result).to_string(),
        count,
    ))
}
