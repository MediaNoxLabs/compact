// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! A finite deterministic property corpus, not a semantics proof or parity claim.
mod corpus;
use super::model::Operation;
use super::*;
use crate::ir::*;
use corpus::*;

fn checked(contract: &Contract) -> CheckedPlan {
    crate::resource_limits::validate(contract)
        .unwrap()
        .finish(contract)
        .unwrap();
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
    check(&contract.stateful_circuits[0], &ledger, &witnesses).unwrap()
}
fn syntax(contract: &Contract) -> String {
    let plan = render(&checked(contract)).unwrap();
    let (steps, result) = (plan.steps, plan.result);
    let block: syn::Block = syn::parse_quote!({ #(#steps)* #result });
    quote::quote!(#block).to_string()
}
fn refusal(contract: &Contract) -> RecordingGap {
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
    match check(&contract.stateful_circuits[0], &ledger, &witnesses) {
        Err(PlanError::Unsupported(gap)) => gap,
        Err(error) => panic!("expected a fragment refusal, got {error:?}"),
        Ok(_) => panic!("unsupported child was admitted"),
    }
}
#[test]
fn seeded_alpha_renaming_preserves_recorded_syntax_and_admission() {
    for seed in 0..32 {
        let original = generated(seed);
        let renamed = alpha_rename(&original);
        assert_ne!(original, renamed);
        assert_eq!(syntax(&original), syntax(&renamed), "seed={seed}");
        for contract in [&original, &renamed] {
            let rendered = crate::render_with_capabilities(contract)
                .unwrap_or_else(|error| panic!("seed={seed}: {error}"));
            assert!(rendered.capabilities.circuits[0].recorded, "seed={seed}");
        }
    }
}
#[test]
fn sequence_association_preserves_ordered_plan_syntax() {
    for width in 0..=12 {
        let actions: Vec<_> = (0..width).map(|n| write(add(read(), number(n)))).collect();
        let flat = contract(ReturnPlan::Sequence {
            actions: actions.clone(),
            result: Box::new(returned(read())),
        });
        let expected = syntax(&flat);
        for split in 0..=width as usize {
            let grouped_action = contract(ReturnPlan::Sequence {
                actions: vec![
                    StateAction::Sequence {
                        actions: actions[..split].to_vec(),
                    },
                    StateAction::Sequence {
                        actions: actions[split..].to_vec(),
                    },
                ],
                result: Box::new(returned(read())),
            });
            let grouped_return = contract(ReturnPlan::Sequence {
                actions: actions[..split].to_vec(),
                result: Box::new(ReturnPlan::Sequence {
                    actions: actions[split..].to_vec(),
                    result: Box::new(returned(read())),
                }),
            });
            assert_eq!(
                syntax(&grouped_action),
                expected,
                "action association width={width},split={split}"
            );
            assert_eq!(
                syntax(&grouped_return),
                expected,
                "return association width={width},split={split}"
            );
        }
    }
}
#[test]
fn shadowing_uses_previous_binding_rhs_and_restores_outer_scope() {
    let inner = Expr::Let {
        bindings: vec![
            local("x", add(parameter("x"), number(1))),
            local("x", add(parameter("x"), number(2))),
        ],
        body: Box::new(parameter("x")),
    };
    let contract = contract(returned(add(inner, parameter("x"))));
    let checked = checked(&contract);
    let plan = checked.plan();
    let parameter_id = plan.parameters[0].id;
    let additions: Vec<_> = plan
        .body
        .bindings
        .iter()
        .filter_map(|b| match &b.operation {
            Operation::Add(a, c) => Some((b.value.id, a.id, c.id)),
            _ => None,
        })
        .collect();
    assert_eq!(additions.len(), 3);
    assert_eq!(
        additions[0].1, parameter_id,
        "first shadow RHS uses the outer parameter"
    );
    assert_eq!(
        additions[1].1, additions[0].0,
        "second shadow RHS uses the preceding local"
    );
    assert_eq!(
        additions[2].1, additions[1].0,
        "inner result uses the final shadow"
    );
    assert_eq!(
        additions[2].2, parameter_id,
        "scope exit restores the outer parameter"
    );
    assert_eq!(syntax(&contract), syntax(&alpha_rename(&contract)));
}
#[test]
fn a_different_type_can_shadow_locally_without_capturing_the_outer_field() {
    let contract = contract(returned(Expr::Sequence {
        steps: vec![Expr::Let {
            bindings: vec![LocalBinding {
                name: "x".into(),
                ty: Type::Boolean,
                value: Expr::Boolean { value: true },
            }],
            body: Box::new(Expr::Assert {
                condition: Box::new(parameter("x")),
                message: "inner Boolean".into(),
            }),
        }],
        value: Box::new(parameter("x")),
    }));
    let checked = checked(&contract);
    assert_eq!(checked.plan().body.result, checked.plan().parameters[0]);
    assert_eq!(syntax(&contract), syntax(&alpha_rename(&contract)));
    assert!(
        crate::render_with_capabilities(&contract)
            .unwrap()
            .capabilities
            .circuits[0]
            .recorded
    );
}

fn unsupported_call(name: &str) -> Expr {
    Expr::Call {
        name: name.into(),
        arguments: vec![],
    }
}
#[test]
fn unsupported_children_in_dead_branches_and_unused_bindings_keep_precise_paths() {
    for prefix in 0..=8 {
        let good: Vec<_> = (0..prefix)
            .map(|n| local(&format!("unused_{n}"), number(n)))
            .collect();
        let mut with_bad = good.clone();
        with_bad.push(local("unused_bad", unsupported_call("first")));
        with_bad.push(local("unused_later", unsupported_call("later")));
        let unused = contract(ReturnPlan::Let {
            bindings: with_bad,
            result: Box::new(returned(read())),
        });
        let dead = contract(ReturnPlan::Let {
            bindings: good,
            result: Box::new(ReturnPlan::Conditional {
                condition: Expr::Boolean { value: true },
                then: Box::new(returned(read())),
                otherwise: Box::new(returned(unsupported_call("first"))),
            }),
        });
        for (candidate, path) in [
            (
                &unused,
                format!("return_value.body.bindings[{prefix}].value"),
            ),
            (&dead, "return_value.body.result.otherwise.value".into()),
        ] {
            let first = refusal(candidate);
            assert_eq!(first.path, path);
            assert_eq!(first.ir_node, "Expr::Call");
            assert_eq!(first, refusal(candidate), "stable repeated refusal");
        }
    }
}
#[test]
fn source_order_selects_the_first_refusal_across_conditions_and_branches() {
    for selected in [false, true] {
        let candidate = contract(ReturnPlan::Conditional {
            condition: Expr::Boolean { value: selected },
            then: Box::new(returned(unsupported_call("then"))),
            otherwise: Box::new(returned(Expr::BytesLiteral { bytes: vec![0] })),
        });
        assert_eq!(refusal(&candidate).path, "return_value.body.then.value");
        let condition_first = contract(ReturnPlan::Conditional {
            condition: unsupported_call("condition"),
            then: Box::new(returned(unsupported_call("then"))),
            otherwise: Box::new(returned(unsupported_call("otherwise"))),
        });
        assert_eq!(
            refusal(&condition_first).path,
            "return_value.body.condition"
        );
    }
}
#[test]
fn helper_calls_remain_outside_the_plan_even_when_native_declarations_exist() {
    let mut candidate = contract(returned(Expr::Call {
        name: "identity".into(),
        arguments: vec![parameter("x")],
    }));
    candidate.circuits.push(PureCircuit {
        source: None,
        name: "identity".into(),
        internal: false,
        parameters: vec![Parameter {
            name: "value".into(),
            ty: Type::Field,
        }],
        result: Type::Field,
        body: parameter("value"),
    });
    let gap = refusal(&candidate);
    assert_eq!(
        (gap.ir_node.as_str(), gap.path.as_str()),
        ("Expr::Call", "return_value.body.value")
    );
    let rendered = crate::render_with_capabilities(&candidate).unwrap();
    let row = rendered
        .capabilities
        .circuits
        .iter()
        .find(|c| c.name == "sample")
        .unwrap();
    assert!(!row.recorded);
    assert_eq!(row.recording_unavailable.as_ref().unwrap(), &gap);
}
#[test]
fn legacy_fallback_can_admit_a_wide_unused_literal_after_plan_refusal() {
    let mut candidate: Contract = serde_json::from_str(include_str!(
        "../../../tests/effectful-return-schema20-ir.json"
    ))
    .unwrap();
    let StateReturn::Effectful { body } = &mut candidate.stateful_circuits[0].return_value else {
        panic!("fixture effectful return")
    };
    *body = ReturnPlan::Let {
        bindings: vec![LocalBinding {
            name: "unused_wide".into(),
            ty: Type::Unsigned {
                max: "340282366920938463463374607431768211456".into(),
            },
            value: Expr::UnsignedLiteral {
                value: "1".into(),
                max: "340282366920938463463374607431768211456".into(),
            },
        }],
        result: Box::new(body.clone()),
    };
    let gap = refusal(&candidate);
    assert_eq!(gap.path, "return_value.body.bindings[0].value");
    let ledger = candidate
        .ledger_fields
        .iter()
        .map(|f| (f.id.as_str(), f))
        .collect();
    let witnesses = candidate
        .witnesses
        .iter()
        .map(|w| (w.name.as_str(), w))
        .collect();
    let pure = candidate
        .circuits
        .iter()
        .map(|c| (c.name.as_str(), c))
        .collect();
    assert!(
        super::super::typed_plan::lower_effectful(
            &candidate.stateful_circuits[0],
            &ledger,
            &witnesses,
            &pure
        )
        .is_some()
    );
    let rendered = crate::render_with_capabilities(&candidate).unwrap();
    assert!(rendered.capabilities.circuits[0].recorded);
    assert!(
        rendered.capabilities.circuits[0]
            .recording_unavailable
            .is_none()
    );
}
