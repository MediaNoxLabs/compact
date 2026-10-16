use super::*;

pub(super) fn field() -> Expr {
    Expr::FieldLiteral { value: "1".into() }
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
pub(super) fn circuit(
    name: &str,
    parameters: Vec<Parameter>,
    value: Expr,
    result: Type,
) -> StatefulCircuit {
    StatefulCircuit {
        source: None,
        name: name.into(),
        internal: name != "root",
        parameters,
        actions: vec![],
        result,
        return_value: StateReturn::Expression { value },
    }
}
pub(super) fn value(source: &StatefulCircuit) -> &Expr {
    let StateReturn::Expression { value } = &source.return_value else {
        panic!("fixture")
    };
    value
}
pub(super) fn registry(sources: &[StatefulCircuit]) -> HashMap<&str, &StatefulCircuit> {
    sources
        .iter()
        .map(|source| (source.name.as_str(), source))
        .collect()
}
pub(super) fn run(
    root: &StatefulCircuit,
    snapshots: &HashMap<&str, &StatefulCircuit>,
    live: &HashMap<&str, &StatefulCircuit>,
    pure: &HashMap<&str, &PureCircuit>,
    observer: Option<&dyn RefusalObserver>,
) -> Result<(String, String), RecordingGap> {
    let ledger = HashMap::new();
    let witnesses = HashMap::new();
    let types = TypeEnvironment::from_sources(root, &ledger, &witnesses, pure, snapshots)?;
    let declarations = Declarations::with_calls(&ledger, &witnesses, pure, snapshots)
        .with_type_environment(types)
        .with_root(root);
    let (plan, certificates) = super::super::super::lower_with_pure_calls_observed(
        root,
        &declarations,
        pure,
        live,
        observer,
    )
    .map_err(LoweringRejection::into_checked_gap)?;
    let declarations = declarations.with_certificates(certificates, pure)?;
    let checked = super::super::super::super::validate::check(plan, declarations)?;
    let rendered = super::super::super::super::render(&checked)?;
    let steps = rendered.steps;
    let result = rendered.result;
    Ok((
        format!("{:?}", checked.plan()),
        quote::quote!(#(#steps)* #result).to_string(),
    ))
}
pub(super) fn same_error(
    root: &StatefulCircuit,
    snapshots: &HashMap<&str, &StatefulCircuit>,
    live: &HashMap<&str, &StatefulCircuit>,
    pure: &HashMap<&str, &PureCircuit>,
    trace: &Trace,
) -> RecordingGap {
    let expected = run(root, snapshots, live, pure, None).unwrap_err();
    let observed = run(root, snapshots, live, pure, Some(trace)).unwrap_err();
    assert_eq!(observed, expected);
    observed
}
#[derive(Clone, Copy)]
pub(super) enum Seed {
    Ordinary,
    AtDepthLimit,
    BeforeWorkLimit,
}
type Outcome = (
    Result<String, RecordingGap>,
    usize,
    usize,
    Vec<String>,
    String,
);
// Explicit malformed/private declarations are tested at the real call owner;
// complete supported runs above also pass binding/check/render downstream gates.
pub(super) fn direct(
    site: CallSite<'_>,
    name: &str,
    arguments: &[Expr],
    snapshots: &HashMap<&str, &StatefulCircuit>,
    live: &HashMap<&str, &StatefulCircuit>,
    observer: Option<&dyn RefusalObserver>,
    seed: Seed,
) -> Outcome {
    let ledger = HashMap::new();
    let witnesses = HashMap::new();
    let pure = HashMap::new();
    let declarations = Declarations::with_calls(&ledger, &witnesses, &pure, snapshots);
    let (max_call_depth, max_expanded_steps) = crate::resource_limits::recording_call_limits();
    let mut lower = Lowerer {
        observer,
        declarations: &declarations,
        circuits: live,
        pure: Certifier::new(&pure, live),
        call_stack: if matches!(seed, Seed::AtDepthLimit) {
            vec!["active".into(); max_call_depth]
        } else {
            vec!["outer".into()]
        },
        expanded_steps: if matches!(seed, Seed::BeforeWorkLimit) {
            max_expanded_steps - 1
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
    let result = lower
        .call(site, name, arguments, &Scope::new(), &mut bindings, "probe")
        .map(|value| format!("{value:?}"));
    (
        result,
        lower.next,
        lower.expanded_steps,
        lower.call_stack,
        format!("{bindings:?}"),
    )
}
pub(super) fn direct_error(
    site: CallSite<'_>,
    name: &str,
    arguments: &[Expr],
    snapshots: &HashMap<&str, &StatefulCircuit>,
    live: &HashMap<&str, &StatefulCircuit>,
    trace: &Trace,
    seed: Seed,
) -> Outcome {
    let expected = direct(site, name, arguments, snapshots, live, None, seed);
    let observed = direct(site, name, arguments, snapshots, live, Some(trace), seed);
    assert_eq!(
        observed, expected,
        "same gap/SSA/fuel/stack/bindings on refusal"
    );
    assert!(observed.0.is_err());
    observed
}
pub(super) fn detail(outcome: &Outcome, text: &str) {
    assert!(
        outcome.0.as_ref().unwrap_err().detail.contains(text),
        "expected {text:?}, got {:?}",
        outcome.0
    );
}
pub(super) fn named_unsupported() -> Type {
    Type::Struct {
        name: "WideMember".into(),
        fields: vec![StructField {
            name: "value".into(),
            ty: Type::Tuple {
                elements: vec![Type::Field; 9],
            },
        }],
    }
}
