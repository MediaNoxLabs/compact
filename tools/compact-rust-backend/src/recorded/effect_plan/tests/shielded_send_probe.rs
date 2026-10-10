// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Whole-old-send controls: canonical wide casts compose; tuple12 remains a probe.
//! Native compilation and runtime/proof qualification remain separate.
use super::*;
use crate::recorded::{admission_probe::trace, profile_attempt::ProfileAttempt};
use serde_json::json;

fn source() -> Contract {
    serde_json::from_str(include_str!(
        "../../../../tests/shielded-send-schema20-ir.json"
    ))
    .unwrap()
}
fn root_mut<'a>(contract: &'a mut Contract, name: &str) -> &'a mut StatefulCircuit {
    contract
        .stateful_circuits
        .iter_mut()
        .find(|c| c.name == name)
        .unwrap()
}
fn probe(contract: &Contract, name: &str, label: &str, require_checked: bool) {
    let ledger = contract
        .ledger_fields
        .iter()
        .map(|f| (f.id.as_str(), f))
        .collect();
    let witnesses = contract
        .witnesses
        .iter()
        .map(|w| (w.name.as_str(), w))
        .collect();
    let pure = contract
        .circuits
        .iter()
        .map(|c| (c.name.as_str(), c))
        .collect();
    let circuits: HashMap<_, _> = contract
        .stateful_circuits
        .iter()
        .map(|c| (c.name.as_str(), c))
        .collect();
    let root = circuits[name];
    assert!(
        matches!(
            crate::recorded::typed_plan::lower_shielded_send_checked(
                root, &ledger, &witnesses, &pure, &circuits
            ),
            ProfileAttempt::Admitted(_)
        ),
        "{label}: whole old checked-send acceptance is mandatory"
    );
    let checked = match check_with_calls(root, &ledger, &witnesses, &pure, &circuits) {
        Ok(plan) => {
            render(&plan).unwrap();
            json!({"status":"supported"})
        }
        Err(PlanError::Unsupported(gap)) => {
            json!({"status":"unsupported","path":gap.path,"detail":gap.detail,"ir_node":gap.ir_node})
        }
        Err(PlanError::Invalid(gap)) => panic!("{label}: invariant failure: {gap:?}"),
    };
    let (output, rows) = trace::capture_json(|| crate::render_with_capabilities(contract));
    let output = output.unwrap_or_else(|e| panic!("{label}: complete native render: {e:?}"));
    let helper = if root.internal {
        let (item, rows) = trace::capture_json(|| {
            let _phase = trace::HelperPlanning::begin();
            crate::recorded::render_recorded_helper(
                root,
                &ledger,
                &witnesses,
                &pure,
                &circuits,
                &std::collections::HashSet::new(),
            )
        });
        let item = item.unwrap();
        json!({"supported":item.is_supported(),"invocations":rows})
    } else {
        serde_json::Value::Null
    };
    let metadata = json!({"label":label,"root":name,"internal":root.internal,"whole_old_send_admitted":true,"checked":checked,"full_render":true,"capabilities":output.capabilities,"invocations":rows,"helper":helper,"claim":"Whole typed IR rendering only; raw crate compilation and runtime/proof results are separate. Probe mutations do not assert in advance that a gap exists."});
    eprintln!("SEND_PROBE {}", serde_json::to_string(&metadata).unwrap());
    if let Some(base) = std::env::var_os("COMPACT_SEND_PROBE_OUTPUT") {
        let dir = std::path::PathBuf::from(base).join(label);
        std::fs::create_dir_all(dir.parent().unwrap()).unwrap();
        std::fs::create_dir(&dir).expect("use a fresh probe output directory");
        std::fs::write(
            dir.join("compact-rust-ir.json"),
            serde_json::to_vec_pretty(contract).unwrap(),
        )
        .unwrap();
        std::fs::write(dir.join("lib.rs"), output.source).unwrap();
        std::fs::write(
            dir.join("case.json"),
            serde_json::to_vec_pretty(&metadata).unwrap(),
        )
        .unwrap();
    }
    if require_checked {
        assert_eq!(
            checked["status"], "supported",
            "{label}: reviewed old-first control must compose"
        );
    }
}
#[test]
fn complete_old_send_baselines_compose_and_render() {
    let contract = source();
    for name in [
        "sendShielded",
        "send_to_self",
        "send_to_user",
        "send_to_contract",
        "send_forward",
    ] {
        probe(&contract, name, &format!("baseline-{name}"), true);
    }
}
#[test]
fn whole_old_send_with_unused_wide_cast_composes_and_renders() {
    let mut contract = source();
    let root = root_mut(&mut contract, "send_to_self");
    let StateReturn::Expression { value } = &mut root.return_value else {
        unreachable!()
    };
    let original = value.clone();
    // Existing `value` formal is canonical Uint128. Conversion changes no
    // required send effect counts; it is deliberately a retained unused local.
    let max = "680564733841876926926749214863536422910".to_owned();
    *value = Expr::Let {
        bindings: vec![LocalBinding {
            name: "unused_wide".into(),
            ty: Type::Unsigned { max: max.clone() },
            value: Expr::UnsignedCast {
                value: Box::new(Expr::Parameter {
                    name: "value".into(),
                }),
                max,
            },
        }],
        body: Box::new(original),
    };
    probe(&contract, "send_to_self", "unused-wide-cast", true);
}
#[test]
fn whole_old_internal_send_with_tuple12_reports_checked_outcome() {
    let mut contract = source();
    let root = root_mut(&mut contract, "send_to_self");
    let mut caller = root.clone();
    root.internal = true;
    root.parameters.push(Parameter {
        name: "unused_tuple".into(),
        ty: Type::Tuple {
            elements: vec![Type::Field; 12],
        },
    });
    // Keep the internal mutant reachable from a public caller whose ABI remains
    // the original two parameters; tuple12 is a local argument, not public FAB.
    caller.name = "tuple12_entry".into();
    caller.return_value = StateReturn::Expression {
        value: Expr::Call {
            name: root.name.clone(),
            arguments: vec![
                Expr::Parameter {
                    name: "input".into(),
                },
                Expr::Parameter {
                    name: "value".into(),
                },
                Expr::Tuple {
                    elements: vec![Expr::FieldLiteral { value: "1".into() }; 12],
                },
            ],
        },
    };
    contract.stateful_circuits.push(caller);
    probe(&contract, "send_to_self", "internal-tuple12", false);
}
