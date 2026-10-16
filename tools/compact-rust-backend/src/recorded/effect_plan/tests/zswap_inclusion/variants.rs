// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;

#[test]
fn whole_old_zswap_parameter_domain_and_identity_operands_compose() {
    let mut c = source();
    circuit_mut(&mut c, "flow").parameters.extend([
        Parameter {
            name: "domain".into(),
            ty: Type::Bytes { length: 32 },
        },
        Parameter {
            name: "amount".into(),
            ty: Type::Unsigned {
                max: u64::MAX.to_string(),
            },
        },
    ]);
    for p in root(&c).parameters.clone() {
        replace_uses(
            &mut c,
            "flow",
            &p.name,
            Expr::Coerce {
                value: Box::new(parameter(&p.name)),
                ty: p.ty,
            },
        );
    }
    included(
        &c,
        "flow",
        "all six canonical parameter classes and identity coercions",
    );
}

#[test]
fn whole_old_zswap_all_effects_and_effect_returns_compose() {
    for return_effect in [false, true] {
        let mut c = source();
        let r = circuit_mut(&mut c, "flow");
        r.parameters.extend([
            Parameter {
                name: "domain".into(),
                ty: Type::Bytes { length: 32 },
            },
            Parameter {
                name: "amount".into(),
                ty: Type::Unsigned {
                    max: u64::MAX.to_string(),
                },
            },
        ]);
        for kind in [
            KernelClaimKind::Nullifier,
            KernelClaimKind::CoinSpend,
            KernelClaimKind::CoinReceive,
        ] {
            r.actions.push(expression(Expr::KernelClaim {
                claim: kind,
                value: Box::new(parameter("domain")),
            }));
        }
        let mint = Expr::KernelMintShielded {
            domain: Box::new(parameter("domain")),
            amount: Box::new(parameter("amount")),
        };
        if return_effect {
            r.return_value = StateReturn::Expression { value: mint };
        } else {
            r.actions.push(expression(mint));
        }
        included(
            &c,
            "flow",
            &format!("all kernel claim/mint leaves, return={return_effect}"),
        );
    }
}

#[test]
fn whole_old_zswap_witness_operand_classes_compose() {
    let mut c = source();
    let root_parameters = root(&c).parameters.clone();
    for p in root_parameters {
        let witness = format!("probe_{}", p.name);
        c.witnesses.push(WitnessDeclaration {
            source: None,
            name: witness.clone(),
            parameters: vec![],
            result: p.ty,
        });
        replace_uses(
            &mut c,
            "flow",
            &p.name,
            Expr::WitnessCall {
                name: witness,
                arguments: vec![],
            },
        );
    }
    included(
        &c,
        "flow",
        "qualified coin, coin, recipient and Boolean witness operands",
    );
    let r = circuit_mut(&mut c, "flow");
    r.actions.push(expression(Expr::KernelMintShielded {
        domain: Box::new(Expr::WitnessCall {
            name: "probe_domain".into(),
            arguments: vec![],
        }),
        amount: Box::new(Expr::WitnessCall {
            name: "probe_amount".into(),
            arguments: vec![],
        }),
    }));
    for (name, result) in [
        ("probe_domain", Type::Bytes { length: 32 }),
        (
            "probe_amount",
            Type::Unsigned {
                max: u64::MAX.to_string(),
            },
        ),
    ] {
        c.witnesses.push(WitnessDeclaration {
            source: None,
            name: name.into(),
            parameters: vec![],
            result,
        });
    }
    included(
        &c,
        "flow",
        "all six canonical zero-argument witness classes",
    );
}

#[test]
fn whole_old_zswap_sequences_and_ordered_shadowing_compose() {
    for mode in 0..3 {
        let mut c = source();
        let r = circuit_mut(&mut c, "flow");
        let original = StateAction::Sequence {
            actions: std::mem::take(&mut r.actions),
        };
        r.actions = vec![match mode {
            0 => StateAction::Sequence {
                actions: vec![original],
            },
            1 => StateAction::Let {
                bindings: vec![],
                action: Box::new(original),
            },
            _ => StateAction::Let {
                bindings: vec![
                    LocalBinding {
                        name: "alias".into(),
                        ty: crate::coin_shapes::shielded_coin_type(),
                        value: parameter("coin"),
                    },
                    LocalBinding {
                        name: "coin".into(),
                        ty: crate::coin_shapes::shielded_coin_type(),
                        value: parameter("alias"),
                    },
                    LocalBinding {
                        name: "alias".into(),
                        ty: crate::coin_shapes::shielded_recipient_type(),
                        value: parameter("recipient"),
                    },
                ],
                action: Box::new(original),
            },
        }];
        included(&c, "flow", &format!("sequence/let/shadowing {mode}"));
    }
}

#[test]
fn whole_old_zswap_helper_unit_effect_returns_and_actual_scopes_compose() {
    for mode in 0..4 {
        let mut c = source();
        let mut helper = root(&c).clone();
        helper.name = "forward_effects".into();
        helper.internal = true;
        if mode == 1 {
            helper.return_value = StateReturn::Expression { value: output() };
        }
        let actuals = helper
            .parameters
            .iter()
            .map(|p| parameter(&p.name))
            .collect::<Vec<_>>();
        circuit_mut(&mut c, "flow").actions = vec![StateAction::CircuitCall {
            name: helper.name.clone(),
            arguments: actuals.clone(),
        }];
        c.stateful_circuits.push(helper);
        if mode == 2 {
            let parameters = circuit_mut(&mut c, "forward_effects").parameters.clone();
            for p in parameters {
                let renamed = format!("formal_{}", p.name);
                replace_uses(&mut c, "forward_effects", &p.name, parameter(&renamed));
                circuit_mut(&mut c, "forward_effects")
                    .parameters
                    .iter_mut()
                    .find(|f| f.name == p.name)
                    .unwrap()
                    .name = renamed;
            }
        }
        if mode == 3 {
            circuit_mut(&mut c, "flow")
                .actions
                .push(StateAction::CircuitCall {
                    name: "forward_effects".into(),
                    arguments: actuals,
                });
        }
        included(
            &c,
            "flow",
            &format!("Unit helper return/renamed formal/sibling reuse {mode}"),
        );
    }
}

#[test]
fn whole_old_zswap_each_branch_and_empty_selected_path_are_structural_cases() {
    for mode in 0..3 {
        let mut c = source();
        let r = circuit_mut(&mut c, "flow");
        let body = StateAction::Sequence {
            actions: std::mem::take(&mut r.actions),
        };
        let empty = StateAction::Sequence { actions: vec![] };
        let (then, otherwise) = match mode {
            0 => (body, empty),
            1 => (empty, body),
            _ => (body.clone(), body),
        };
        r.actions = vec![StateAction::If {
            condition: parameter("selected"),
            then: Box::new(then),
            otherwise: Box::new(otherwise),
        }];
        included(
            &c,
            "flow",
            &format!("outer branch placement {mode}; empty path may fail preparation"),
        );
    }
}

#[test]
fn whole_old_zswap_declaration_names_are_data() {
    fn rename(value: &mut Json) {
        match value {
            Json::Object(fields) => {
                for key in ["name", "id", "field"] {
                    if let Some(Json::String(name)) = fields.get_mut(key)
                        && [
                            "flow",
                            "pair",
                            "pot",
                            "coin",
                            "input",
                            "recipient",
                            "selected",
                        ]
                        .contains(&name.as_str())
                    {
                        *name = format!("zswap_probe_{name}");
                    }
                }
                fields.values_mut().for_each(rename);
            }
            Json::Array(values) => values.iter_mut().for_each(rename),
            _ => {}
        }
    }
    let mut value = serde_json::to_value(source()).unwrap();
    rename(&mut value);
    let c: Contract = serde_json::from_value(value).unwrap();
    included(
        &c,
        "zswap_probe_flow",
        "renamed root/helper/ledger/formals and references",
    );
}
