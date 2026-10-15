//! Shallow prior-dispatch comparisons plus explicit fuel/path/SSA invariants.
//! Child family helpers are unchanged/current; immutable-binary full continuity
//! and unchanged root near44/over45/malformed worker are parent gates.
use super::*;
mod old_dispatch;

#[derive(Debug, PartialEq, Eq)]
struct Snapshot {
    outcome: Result<Value, RecordingGap>,
    bindings: String,
    next: usize,
    spent: usize,
}
fn run(source: &Expr, scope: &Scope, budget: usize, prior: bool) -> Snapshot {
    let declarations = Declarations::default();
    let pure = HashMap::new();
    let circuits = HashMap::new();
    let mut lower = Lowerer {
        observer: None,
        declarations: &declarations,
        circuits: &circuits,
        pure: Certifier::new(&pure, &circuits),
        call_stack: vec![],
        expanded_steps: 0,
        max_call_depth: usize::MAX,
        max_expanded_steps: budget,
        next: 0,
        root_action: None,
        failure_precision: Precision::Unclassified,
        concrete_node: None,
    };
    let mut bindings = Vec::new();
    let outcome = if prior {
        lower.original_expression(source, scope, &mut bindings, "probe")
    } else {
        lower.expression(source, scope, &mut bindings, "probe")
    };
    Snapshot {
        outcome,
        bindings: format!("{bindings:?}"),
        next: lower.next,
        spent: lower.expanded_steps,
    }
}
fn same(source: &Expr, scope: &Scope) -> Snapshot {
    let actual = run(source, scope, 4096, false);
    assert_eq!(actual, run(source, scope, 4096, true));
    actual
}
fn field(value: &str) -> Expr {
    Expr::FieldLiteral {
        value: value.into(),
    }
}

#[test]
fn binary_dispatch_retains_one_spend_per_occurrence_and_exact_ssa_order() {
    let source = Expr::Add {
        left: Box::new(field("1")),
        right: Box::new(field("2")),
    };
    let result = same(&source, &Scope::new());
    assert!(result.outcome.is_ok());
    assert_eq!(result.next, 3);
    assert_eq!(result.spent, 3);
    let first = result.bindings.find("probe.left").unwrap();
    let second = result.bindings.find("probe.right").unwrap();
    assert!(first < second);
    assert!(result.bindings.contains("Add(Value { id: 0"));
    assert_eq!(result.outcome.unwrap().id, 2);
}

#[test]
fn binary_child_refusal_keeps_original_first_path_and_partial_ssa() {
    let source = Expr::Add {
        left: Box::new(Expr::Parameter {
            name: "first".into(),
        }),
        right: Box::new(Expr::Parameter {
            name: "second".into(),
        }),
    };
    let result = same(&source, &Scope::new());
    let error = result.outcome.unwrap_err();
    assert_eq!(error.path, "probe.left");
    assert_eq!(error.detail, "unbound value");
    assert_eq!(result.spent, 2);
    assert_eq!(result.next, 0);
}

#[test]
fn early_parameter_and_cast_values_do_not_gain_an_extra_binding() {
    let scope = Scope::from([(
        "value".into(),
        Value {
            id: 17,
            ty: Type::Field,
        },
    )]);
    let source = Expr::Parameter {
        name: "value".into(),
    };
    let result = same(&source, &scope);
    assert_eq!(result.outcome.unwrap().id, 17);
    assert_eq!(result.next, 0);
    assert_eq!(result.spent, 1);
    let source = Expr::Coerce {
        value: Box::new(Expr::Parameter {
            name: "value".into(),
        }),
        ty: Type::Field,
    };
    let result = same(&source, &scope);
    assert_eq!(result.outcome.unwrap().id, 17);
    assert_eq!(result.next, 0);
    assert_eq!(result.spent, 2);
}

#[test]
fn control_and_conversion_owners_preserve_paths_guards_and_child_order() {
    let cases = [
        Expr::Let {
            bindings: vec![LocalBinding {
                name: "local".into(),
                ty: Type::Field,
                value: field("1"),
            }],
            body: Box::new(Expr::Parameter {
                name: "local".into(),
            }),
        },
        Expr::Sequence {
            steps: vec![Expr::Unit],
            value: Box::new(field("2")),
        },
        Expr::If {
            condition: Box::new(Expr::Boolean { value: true }),
            then: Box::new(field("1")),
            otherwise: Box::new(field("2")),
        },
        Expr::FieldToBytes32 {
            value: Box::new(field("1")),
        },
        Expr::UnsignedCast {
            value: Box::new(Expr::UnsignedLiteral {
                value: "1".into(),
                max: "255".into(),
            }),
            max: "65535".into(),
        },
    ];
    for source in cases {
        assert!(same(&source, &Scope::new()).outcome.is_ok());
    }
    let source = Expr::Sequence {
        steps: vec![field("1")],
        value: Box::new(Expr::Parameter {
            name: "later".into(),
        }),
    };
    let result = same(&source, &Scope::new());
    assert_eq!(result.outcome.unwrap_err().path, "probe.steps[0]");
}

#[test]
fn literal_persistent_assert_and_witness_guards_keep_typed_refusals() {
    let cases = [
        field("bad"),
        Expr::UnsignedLiteral {
            value: "256".into(),
            max: "255".into(),
        },
        Expr::PersistentCommit {
            value: Box::new(Expr::Default {
                ty: Type::OpaqueString,
            }),
            opening: Box::new(Expr::BytesLiteral { bytes: vec![0; 32] }),
        },
        Expr::Assert {
            condition: Box::new(field("1")),
            message: "bad condition".into(),
        },
        Expr::WitnessCall {
            name: "missing".into(),
            arguments: vec![Expr::Parameter {
                name: "later".into(),
            }],
        },
    ];
    for source in cases {
        assert!(same(&source, &Scope::new()).outcome.is_err());
    }
    let good = Expr::PersistentHash {
        value: Box::new(Expr::BytesLiteral { bytes: vec![0; 32] }),
    };
    assert!(same(&good, &Scope::new()).outcome.is_ok());
}

#[test]
fn fuel_failure_precedes_domain_dispatch_and_unsupported_stays_refused() {
    let source = Expr::Parameter {
        name: "missing".into(),
    };
    let actual = run(&source, &Scope::new(), 0, false);
    assert_eq!(actual, run(&source, &Scope::new(), 0, true));
    let error = actual.outcome.unwrap_err();
    assert_eq!(error.path, "probe");
    assert_eq!(error.detail, "recording expansion work exceeds 0");
    assert_eq!(actual.spent, 0);
    let source = Expr::VectorFoldCall {
        name: "reducer".into(),
        initial: Box::new(field("0")),
        source: Box::new(Expr::Vector {
            element: Type::Field,
            elements: vec![field("1")],
        }),
        accumulator: Type::Field,
        element: Type::Field,
        length: 1,
    };
    let result = same(&source, &Scope::new());
    assert!(result.outcome.is_err());
    assert_eq!(result.spent, 1);
    assert_eq!(result.next, 0);
}

#[test]
fn checked_plan_render_keeps_original_ssa_rust_steps_and_result() {
    fn syntax(source: &Expr, prior: bool) -> String {
        let declarations = Declarations::default();
        let pure = HashMap::new();
        let circuits = HashMap::new();
        let mut lower = Lowerer {
            observer: None,
            declarations: &declarations,
            circuits: &circuits,
            pure: Certifier::new(&pure, &circuits),
            call_stack: vec![],
            expanded_steps: 0,
            max_call_depth: usize::MAX,
            max_expanded_steps: 4096,
            next: 0,
            root_action: None,
            failure_precision: Precision::Unclassified,
            concrete_node: None,
        };
        let mut bindings = Vec::new();
        let result = if prior {
            lower.original_expression(source, &Scope::new(), &mut bindings, "probe")
        } else {
            lower.expression(source, &Scope::new(), &mut bindings, "probe")
        }
        .unwrap();
        let result_type = result.ty.clone();
        drop(lower);
        let checked = validate::check(
            Plan {
                parameters: vec![],
                body: Block { bindings, result },
                result_type,
            },
            declarations,
        )
        .unwrap();
        let rendered = crate::recorded::effect_plan::render(&checked).unwrap();
        let steps = rendered.steps;
        let result = rendered.result;
        quote::quote!(#(#steps)* #result).to_string()
    }
    for source in [
        Expr::Add {
            left: Box::new(field("1")),
            right: Box::new(field("2")),
        },
        Expr::TupleIndex {
            value: Box::new(Expr::Tuple {
                elements: vec![field("1"), field("2")],
            }),
            index: 1,
        },
    ] {
        assert_eq!(syntax(&source, false), syntax(&source, true));
    }
}
