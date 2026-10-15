// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Whole-old-voting-first internal helper probes. Native rendering is checked,
//! but these IR probes alone do not establish generated-crate compilation/proof.
use super::*;

fn complete_source() -> Contract {
    serde_json::from_str(include_str!(
        "../../../../tests/micro-dao-source-schema20-ir.json"
    ))
    .unwrap()
}

fn product(name: &str, ty: Type) -> Type {
    Type::Struct {
        name: name.into(),
        fields: vec![StructField {
            name: "item".into(),
            ty,
        }],
    }
}
fn sample_cases() -> Vec<(Type, Expr)> {
    let field = Expr::FieldLiteral { value: "1".into() };
    let tuple = Type::Tuple {
        elements: vec![Type::Field, Type::Boolean],
    };
    let vector = Type::Vector {
        element: Box::new(Type::Field),
        length: 2,
    };
    let tuple_value = Expr::Tuple {
        elements: vec![field.clone(), Expr::Boolean { value: true }],
    };
    let vector_value = Expr::Coerce {
        value: Box::new(Expr::Tuple {
            elements: vec![field.clone(), field],
        }),
        ty: vector.clone(),
    };
    let mut cases = vec![(tuple, tuple_value), (vector.clone(), vector_value)];
    for member in [vector, Type::OpaqueString, Type::OpaqueBytes] {
        let ty = product("RetirementProduct", member);
        cases.push((ty.clone(), Expr::Default { ty }));
    }
    for ty in [Type::OpaqueString, Type::OpaqueBytes] {
        cases.push((
            ty.clone(),
            Expr::StructField {
                value: Box::new(Expr::Default {
                    ty: product("RetirementCarrier", ty),
                }),
                field: "item".into(),
                index: 0,
            },
        ));
    }
    cases
}
fn prepend(contract: &mut Contract, ty: Type, value: Expr) {
    let send = contract
        .stateful_circuits
        .iter_mut()
        .find(|c| c.name == "sendShielded")
        .unwrap();
    let StateReturn::Expression { value: body } = &mut send.return_value else {
        unreachable!()
    };
    let original = std::mem::replace(body, Expr::Unit);
    *body = Expr::Let {
        bindings: vec![LocalBinding {
            name: "retirement_unused".into(),
            ty,
            value,
        }],
        body: Box::new(original),
    };
}
fn helper(ty: &Type, value: &Expr, mode: &str) -> (StatefulCircuit, Type, Expr) {
    let parameterized = mode != "result";
    let result_only = mode != "parameter";
    let parameters = if parameterized {
        vec![Parameter {
            name: "input".into(),
            ty: ty.clone(),
        }]
    } else {
        vec![]
    };
    let result = if result_only {
        ty.clone()
    } else {
        Type::Boolean
    };
    let body = if !result_only {
        Expr::Boolean { value: true }
    } else if parameterized {
        parameter("input")
    } else {
        value.clone()
    };
    let call = Expr::Call {
        name: "retirement_helper".into(),
        arguments: if parameterized {
            vec![value.clone()]
        } else {
            vec![]
        },
    };
    (
        StatefulCircuit {
            source: None,
            name: "retirement_helper".into(),
            internal: true,
            parameters,
            result: result.clone(),
            actions: vec![],
            return_value: StateReturn::Expression { value: body },
        },
        result,
        call,
    )
}
fn old_first_and_native_render(contract: &Contract, label: &str) {
    Environment::new(contract).old_first(label);
    let rendered = crate::render_with_capabilities(contract)
        .unwrap_or_else(|error| panic!("{label}: native/capability rendering failed: {error:?}"));
    capture(contract, label, &rendered.source);
}
// Opt-in evidence export only. Each test process requires a fresh destination;
// separate case directories avoid shared append races under the parallel runner.
// The parent qualification harness compiles these exact unmodified files.
fn capture(contract: &Contract, label: &str, generated: &str) {
    let Some(root) = std::env::var_os("COMPACT_VOTING_HELPER_PROBE_OUTPUT") else {
        return;
    };
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let index = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::path::PathBuf::from(root);
    std::fs::create_dir_all(&root).unwrap();
    let directory = root.join(format!("{index:03}"));
    std::fs::create_dir(&directory).expect("probe export requires a fresh destination");
    std::fs::write(
        directory.join("compact-rust-ir.json"),
        serde_json::to_vec_pretty(contract).unwrap(),
    )
    .unwrap();
    std::fs::write(directory.join("lib.rs"), generated).unwrap();
    let metadata = serde_json::json!({
        "index": index, "label": label,
        "old_whole_voting_accepted": true, "native_render_succeeded": true,
        "qualification": "IR/render evidence only; compilation and proof are separate",
    });
    std::fs::write(
        directory.join("case.json"),
        serde_json::to_vec_pretty(&metadata).unwrap(),
    )
    .unwrap();
}

fn whole_included(contract: &Contract, label: &str) {
    old_first_and_native_render(contract, label);
    let checked = Environment::new(contract)
        .checked("vote_commit")
        .unwrap_or_else(|gap| {
            panic!("{label}: complete old-accepted voting root refused: {gap:?}")
        });
    render(&checked).unwrap_or_else(|error| panic!("{label}: checked render failed: {error:?}"));
}

#[test]
fn unused_send_values_without_helper_boundaries_already_compose() {
    for (ty, value) in sample_cases() {
        let mut contract = complete_source();
        prepend(&mut contract, ty.clone(), value);
        let label = format!("local {ty:?}");
        old_first_and_native_render(&contract, &label);
        let checked = Environment::new(&contract)
            .checked("vote_commit")
            .unwrap_or_else(|gap| panic!("{label}: {gap:?}"));
        render(&checked).unwrap();
    }
}

#[test]
fn whole_old_voting_internal_send_helper_parameters_and_results_compose() {
    for (ty, value) in sample_cases() {
        for mode in ["parameter", "result", "both"] {
            let mut contract = complete_source();
            let (callee, result, call) = helper(&ty, &value, mode);
            contract.stateful_circuits.push(callee);
            prepend(&mut contract, result, call);
            whole_included(&contract, &format!("{mode} {ty:?}"));
        }
    }
}

#[test]
fn whole_old_voting_unselected_send_helper_calls_compose() {
    for (ty, value) in sample_cases() {
        let mut contract = complete_source();
        let (callee, result, call) = helper(&ty, &value, "result");
        contract.stateful_circuits.push(callee);
        prepend(
            &mut contract,
            result,
            Expr::If {
                condition: Box::new(Expr::Boolean { value: false }),
                then: Box::new(call),
                otherwise: Box::new(value),
            },
        );
        whole_included(&contract, &format!("unselected {ty:?}"));
    }
}
