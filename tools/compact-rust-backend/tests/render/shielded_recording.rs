// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//  	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Qualified coins, kernel effects, and composite Zswap intent recording.

use super::support::identity;
use compact_rust_backend::ir::{
    Contract, Expr, LedgerField, LedgerFieldKind, Parameter, PureCircuit, SCHEMA_VERSION,
    StateAction, StateReturn, StatefulCircuit, StructField, Type,
};
use compact_rust_backend::{RenderError, render, render_with_capabilities};

#[test]
fn qualified_set_coin_insert_requires_exact_typed_slot_and_operands() {
    let bytes = Type::Bytes { length: 32 };
    let uint128 = Type::Unsigned {
        max: u128::MAX.to_string(),
    };
    let uint64 = Type::Unsigned {
        max: u64::MAX.to_string(),
    };
    let coin_fields = vec![
        StructField {
            name: "nonce".into(),
            ty: bytes.clone(),
        },
        StructField {
            name: "color".into(),
            ty: bytes.clone(),
        },
        StructField {
            name: "value".into(),
            ty: uint128,
        },
    ];
    let coin = Type::Struct {
        name: "ShieldedCoinInfo".into(),
        fields: coin_fields.clone(),
    };
    let mut qualified_fields = coin_fields;
    qualified_fields.push(StructField {
        name: "mt_index".into(),
        ty: uint64,
    });
    let qualified = Type::Struct {
        name: "QualifiedShieldedCoinInfo".into(),
        fields: qualified_fields,
    };
    let recipient = Type::Struct {
        name: "Either".into(),
        fields: vec![
            StructField {
                name: "is_left".into(),
                ty: Type::Boolean,
            },
            StructField {
                name: "left".into(),
                ty: Type::Struct {
                    name: "ZswapCoinPublicKey".into(),
                    fields: vec![StructField {
                        name: "bytes".into(),
                        ty: bytes.clone(),
                    }],
                },
            },
            StructField {
                name: "right".into(),
                ty: Type::Struct {
                    name: "ContractAddress".into(),
                    fields: vec![StructField {
                        name: "bytes".into(),
                        ty: bytes,
                    }],
                },
            },
        ],
    };
    let mut contract = identity(Type::Unit, Expr::Unit);
    contract.ledger_fields.push(LedgerField {
        source: None,
        id: "coins".into(),
        index: 0,
        path: vec![],
        declaration: LedgerFieldKind::Set { ty: qualified },
    });
    contract.stateful_circuits.push(StatefulCircuit {
        source: None,
        name: "insert_coin".into(),
        internal: false,
        parameters: vec![
            Parameter {
                name: "coin".into(),
                ty: coin.clone(),
            },
            Parameter {
                name: "recipient".into(),
                ty: recipient.clone(),
            },
        ],
        actions: vec![StateAction::SetInsertCoin {
            field: "coins".into(),
            index: 0,
            coin: Expr::Parameter {
                name: "coin".into(),
            },
            recipient: Expr::Parameter {
                name: "recipient".into(),
            },
        }],
        result: Type::Unit,
        return_value: StateReturn::Unit,
    });
    let rendered = render_with_capabilities(&contract).unwrap();
    assert!(rendered.source.contains(".insert_coin("));
    assert!(rendered.capabilities.circuits[0].recorded);
    assert!(rendered.capabilities.circuits[0].observed_call);

    let mut wrong_coin = contract.clone();
    wrong_coin.stateful_circuits[0].parameters[0].ty = Type::Field;
    assert!(matches!(
        render(&wrong_coin),
        Err(RenderError::TypeMismatch { .. })
    ));
    let mut wrong_recipient = contract.clone();
    wrong_recipient.stateful_circuits[0].parameters[1].ty = Type::Field;
    assert!(matches!(
        render(&wrong_recipient),
        Err(RenderError::TypeMismatch { .. })
    ));
    let mut wrong_slot = contract.clone();
    wrong_slot.ledger_fields[0].declaration = LedgerFieldKind::Set { ty: coin };
    assert!(render(&wrong_slot).is_err());
    let mut wrong_index = contract.clone();
    wrong_index.stateful_circuits[0].actions[0] = StateAction::SetInsertCoin {
        field: "coins".into(),
        index: 1,
        coin: Expr::Parameter {
            name: "coin".into(),
        },
        recipient: Expr::Parameter {
            name: "recipient".into(),
        },
    };
    assert!(render(&wrong_index).is_err());
    let mut unknown_binding = contract.clone();
    if let StateAction::SetInsertCoin { coin, .. } =
        &mut unknown_binding.stateful_circuits[0].actions[0]
    {
        *coin = Expr::Parameter {
            name: "escaped".into(),
        };
    }
    assert!(matches!(
        render(&unknown_binding),
        Err(RenderError::UnknownParameter(_))
    ));
    contract.schema_version = 13;
    assert_eq!(render(&contract), Err(RenderError::SchemaVersion(13)));
}

#[test]
fn qualified_cell_coin_write_is_typed_and_recordable() {
    let mut contract: Contract =
        serde_json::from_str(include_str!("../qualified-coin-cell-schema16-ir.json")).unwrap();
    contract.schema_version = SCHEMA_VERSION;
    let rendered = render_with_capabilities(&contract).unwrap();
    let write = rendered
        .capabilities
        .circuits
        .iter()
        .find(|c| c.name == "write_coin")
        .unwrap();
    assert!(write.recorded && write.observed_call);
    assert!(write.recording_unavailable.is_none());
    assert!(rendered.source.contains(".write_coin("));
    assert!(rendered.source.contains(".record_write_coin("));
    for mode in 0..4 {
        let mut wrong = contract.clone();
        let circuit = wrong
            .stateful_circuits
            .iter_mut()
            .find(|c| c.name == "write_coin")
            .unwrap();
        let StateAction::CellWriteCoin {
            index,
            coin,
            recipient,
            ..
        } = &mut circuit.actions[0]
        else {
            panic!("expected coin write")
        };
        match mode {
            0 => *index = 1,
            1 => *coin = Expr::Boolean { value: false },
            2 => *recipient = Expr::Boolean { value: false },
            _ => wrong.ledger_fields[0].declaration = LedgerFieldKind::Cell { ty: Type::Boolean },
        }
        assert!(
            render_with_capabilities(&wrong).is_err(),
            "invalid qualified Cell mode {mode}"
        );
    }
    let mut wrong_field = contract.clone();
    let StateAction::CellWriteCoin { field, .. } = &mut wrong_field
        .stateful_circuits
        .iter_mut()
        .find(|c| c.name == "write_coin")
        .unwrap()
        .actions[0]
    else {
        unreachable!()
    };
    *field = "missing".into();
    assert!(render_with_capabilities(&wrong_field).is_err());
    let mut escaped = contract.clone();
    let StateAction::CellWriteCoin { coin, .. } = &mut escaped
        .stateful_circuits
        .iter_mut()
        .find(|c| c.name == "write_coin")
        .unwrap()
        .actions[0]
    else {
        unreachable!()
    };
    *coin = Expr::Parameter {
        name: "escaped".into(),
    };
    assert!(render_with_capabilities(&escaped).is_err());
    let mut nested = contract;
    nested.ledger_fields[0].path = vec![1];
    assert!(render_with_capabilities(&nested).is_err());
}

#[test]
fn native_zswap_intents_validate_types_and_record_bounded_unit_helpers() {
    let mut contract: Contract =
        serde_json::from_str(include_str!("../native-zswap-intents-schema18-ir.json")).unwrap();
    contract.schema_version = SCHEMA_VERSION;
    let rendered = render_with_capabilities(&contract).unwrap();
    for name in ["produce", "consume", "flow", "witness_order", "read_coin"] {
        let circuit = rendered
            .capabilities
            .circuits
            .iter()
            .find(|c| c.name == name)
            .unwrap();
        let recorded = matches!(name, "flow" | "witness_order" | "read_coin");
        assert_eq!(circuit.recorded, recorded, "{name}");
        assert_eq!(circuit.observed_call, recorded, "{name}");
    }
    assert!(rendered.source.contains(".create_zswap_output("));
    assert!(rendered.source.contains(".create_zswap_input("));
    assert!(!rendered.source.contains(".call_local("));
    // Removing the only public query leaves input/output intents, including
    // the inlined pair helper, but cannot make a proof API by itself.
    let mut intent_only = contract.clone();
    let flow = intent_only
        .stateful_circuits
        .iter_mut()
        .find(|c| c.name == "flow")
        .unwrap();
    let StateAction::If { then, .. } = &mut flow.actions[1] else {
        panic!("expected branch")
    };
    let StateAction::Sequence { actions } = then.as_mut() else {
        panic!("expected sequence")
    };
    actions.retain(|action| !matches!(action, StateAction::CellWriteCoin { .. }));
    let native_only = render_with_capabilities(&intent_only).unwrap();
    let flow = native_only
        .capabilities
        .circuits
        .iter()
        .find(|c| c.name == "flow")
        .unwrap();
    assert!(!flow.recorded && !flow.observed_call);
    for mode in 0..3 {
        let mut wrong = contract.clone();
        let name = if mode == 0 { "consume" } else { "produce" };
        let circuit = wrong
            .stateful_circuits
            .iter_mut()
            .find(|c| c.name == name)
            .unwrap();
        let StateReturn::Expression { value } = &mut circuit.return_value else {
            panic!("expected native return");
        };
        match value {
            Expr::CreateZswapInput { coin } => **coin = Expr::Boolean { value: false },
            Expr::CreateZswapOutput { coin, recipient } => {
                if mode == 1 {
                    **coin = Expr::Boolean { value: false }
                } else {
                    **recipient = Expr::Boolean { value: false }
                }
            }
            _ => panic!("expected native expression"),
        }
        assert!(render_with_capabilities(&wrong).is_err());
    }
    let mut pure = contract.clone();
    let StateReturn::Expression { value } = pure
        .stateful_circuits
        .iter()
        .find(|c| c.name == "consume")
        .unwrap()
        .return_value
        .clone()
    else {
        panic!()
    };
    pure.circuits.push(PureCircuit {
        name: "invalid_pure".into(),
        source: None,
        internal: false,
        parameters: vec![],
        result: Type::Unit,
        body: value,
    });
    assert!(render_with_capabilities(&pure).is_err());
    for bad in [
        r#"{"kind":"create_zswap_input"}"#,
        r#"{"kind":"create_zswap_output","coin":{"kind":"unit"}}"#,
        r#"{"kind":"create_zswap_input","coin":{"kind":"unit"},"arguments":[]}"#,
    ] {
        assert!(serde_json::from_str::<Expr>(bad).is_err());
    }
}

#[test]
fn kernel_effects_have_typed_arguments_and_no_fictional_ledger_slots() {
    let mut contract: Contract =
        serde_json::from_str(include_str!("../kernel-shielded-effects-schema20-ir.json")).unwrap();
    contract.schema_version = SCHEMA_VERSION;
    assert_eq!(contract.ledger_fields.len(), 1);
    assert_eq!(contract.ledger_fields[0].id, "marker");
    let rendered = render_with_capabilities(&contract).unwrap();
    for c in &rendered.capabilities.circuits {
        assert!(c.recorded, "{}", c.name);
        assert!(c.observed_call, "{}", c.name);
    }
    assert!(rendered.source.contains(".kernel_mint_shielded("));
    assert!(rendered.source.contains(".kernel_claim_zswap_nullifier("));
    assert!(!rendered.source.contains(".call_local("));
    for mode in 0..4 {
        let mut wrong = contract.clone();
        let name = if mode == 0 { "spend" } else { "mint" };
        let circuit = wrong
            .stateful_circuits
            .iter_mut()
            .find(|c| c.name == name)
            .unwrap();
        let StateReturn::Expression { value } = &mut circuit.return_value else {
            panic!("expected Kernel return");
        };
        match value {
            Expr::KernelClaim { value, .. } => **value = Expr::Boolean { value: false },
            Expr::KernelMintShielded { domain, amount } => match mode {
                1 => **domain = Expr::Boolean { value: false },
                2 => **amount = Expr::Boolean { value: false },
                _ => {
                    **amount = Expr::UnsignedLiteral {
                        value: "1".into(),
                        max: u128::MAX.to_string(),
                    }
                }
            },
            _ => panic!("expected typed Kernel expression"),
        }
        assert!(
            render_with_capabilities(&wrong).is_err(),
            "invalid Kernel mode {mode}"
        );
    }
    let mut pure = contract.clone();
    let circuit = pure
        .stateful_circuits
        .iter()
        .find(|c| c.name == "mint")
        .unwrap()
        .clone();
    let StateReturn::Expression { value } = circuit.return_value else {
        panic!()
    };
    pure.circuits.push(PureCircuit {
        name: "invalid_pure".into(),
        source: None,
        internal: false,
        parameters: circuit.parameters,
        result: Type::Unit,
        body: value,
    });
    assert!(render_with_capabilities(&pure).is_err());
    for bad in [
        r#"{"kind":"kernel_claim","claim":"coin_spend","value":{"kind":"unit"},"path":[0]}"#,
        r#"{"kind":"kernel_claim","claim":"invented","value":{"kind":"unit"}}"#,
        r#"{"kind":"kernel_mint_shielded","domain":{"kind":"unit"}}"#,
    ] {
        assert!(serde_json::from_str::<Expr>(bad).is_err());
    }
}

#[test]
fn kernel_recording_composes_public_slots_but_rejects_escaped_scopes() {
    let mut contract: Contract =
        serde_json::from_str(include_str!("../kernel-shielded-effects-schema20-ir.json")).unwrap();
    contract.schema_version = SCHEMA_VERSION;
    let mut combined = contract.clone();
    let mint = combined
        .stateful_circuits
        .iter_mut()
        .find(|c| c.name == "mint")
        .unwrap();
    mint.actions.push(StateAction::CellWrite {
        field: "marker".into(),
        index: 0,
        value: Expr::FieldLiteral { value: "1".into() },
    });
    let rendered = render_with_capabilities(&combined).unwrap();
    let mint = rendered
        .capabilities
        .circuits
        .iter()
        .find(|c| c.name == "mint")
        .unwrap();
    assert!(mint.recorded && mint.observed_call);
    let recorded = rendered.source.split("pub mod recorded").nth(1).unwrap();
    let mint_body = recorded.split("pub fn nullifier").next().unwrap();
    assert!(
        mint_body.find(".record_write(frame,").unwrap()
            < mint_body.find(".kernel_mint_shielded(").unwrap()
    );

    let mut escaped = contract.clone();
    let circuit = escaped
        .stateful_circuits
        .iter_mut()
        .find(|c| c.name == "witness_order")
        .unwrap();
    // The first Let is followed by a sibling action: its locals cannot escape.
    circuit.actions.push(StateAction::Expression {
        value: Expr::KernelClaim {
            claim: compact_rust_backend::ir::KernelClaimKind::CoinSpend,
            value: Box::new(Expr::Parameter {
                name: "tmp_1".into(),
            }),
        },
    });
    assert!(
        render_with_capabilities(&escaped).is_err(),
        "nested Let binding escaped its Kernel action"
    );

    let mut invalid_branch = contract.clone();
    let circuit = invalid_branch
        .stateful_circuits
        .iter_mut()
        .find(|c| c.name == "selected")
        .unwrap();
    let StateAction::If { condition, .. } = &mut circuit.actions[0] else {
        panic!()
    };
    *condition = Expr::Parameter {
        name: "value".into(),
    };
    assert!(
        render_with_capabilities(&invalid_branch).is_err(),
        "Bytes32 used as Boolean branch condition"
    );

    let mut invalid_witness = contract.clone();
    invalid_witness
        .witnesses
        .iter_mut()
        .find(|w| w.name == "next_amount")
        .unwrap()
        .result = Type::Field;
    assert!(
        render_with_capabilities(&invalid_witness).is_err(),
        "Field witness used as Uint64 mint amount"
    );
}

#[test]
fn stateful_structs_validate_member_types_and_record_zswap_unit_members() {
    let mut contract: Contract =
        serde_json::from_str(include_str!("../stateful-struct-schema20-ir.json")).unwrap();
    contract.schema_version = SCHEMA_VERSION;
    let rendered = render_with_capabilities(&contract).unwrap();
    assert!(rendered.source.contains("__compact_struct_member_"));
    assert_eq!(rendered.capabilities.circuits.len(), 4);
    assert!(
        rendered
            .capabilities
            .circuits
            .iter()
            .all(|c| c.recorded && c.observed_call)
    );
    for mode in 0..4 {
        let mut wrong = contract.clone();
        let circuit = wrong
            .stateful_circuits
            .iter_mut()
            .find(|c| c.name == "reverse")
            .unwrap();
        let StateReturn::Expression {
            value: Expr::StructLiteral { ty, fields },
        } = &mut circuit.return_value
        else {
            panic!("expected literal")
        };
        match mode {
            0 => *ty = Type::Boolean,
            1 => {
                fields.pop();
            }
            2 => fields[0] = Expr::Boolean { value: false },
            3 => {
                let Expr::UnsignedCast { value, .. } = &fields[0] else {
                    panic!("expected explicit widening")
                };
                fields[0] = *value.clone();
            }
            _ => unreachable!(),
        }
        assert!(
            render_with_capabilities(&wrong).is_err(),
            "malformed literal mode {mode}"
        );
    }
    let mut pure = contract.clone();
    let circuit = pure
        .stateful_circuits
        .iter()
        .find(|c| c.name == "planned")
        .unwrap()
        .clone();
    let StateReturn::Expression { value } = circuit.return_value else {
        panic!()
    };
    pure.circuits.push(PureCircuit {
        name: "invalid_pure_struct".into(),
        source: None,
        internal: false,
        parameters: circuit.parameters,
        result: circuit.result,
        body: value,
    });
    assert!(render_with_capabilities(&pure).is_err());
}

#[test]
fn zswap_recording_does_not_adopt_query_helpers_or_escaped_bindings() {
    let mut contract: Contract =
        serde_json::from_str(include_str!("../native-zswap-intents-schema18-ir.json")).unwrap();
    contract.schema_version = SCHEMA_VERSION;
    let pair = contract
        .stateful_circuits
        .iter_mut()
        .find(|c| c.name == "pair")
        .unwrap();
    pair.actions.push(StateAction::Expression {
        value: Expr::KernelSelf {
            ty: Type::Struct {
                name: "ContractAddress".into(),
                fields: vec![compact_rust_backend::ir::StructField {
                    name: "bytes".into(),
                    ty: Type::Bytes { length: 32 },
                }],
            },
        },
    });
    let rendered = render_with_capabilities(&contract).unwrap();
    let flow = rendered
        .capabilities
        .circuits
        .iter()
        .find(|c| c.name == "flow")
        .unwrap();
    assert!(!flow.recorded && !flow.observed_call);
    assert!(!rendered.source.contains(".call_local("));
    let pair = contract
        .stateful_circuits
        .iter_mut()
        .find(|c| c.name == "pair")
        .unwrap();
    pair.actions.pop();
    pair.actions.push(StateAction::Expression {
        value: Expr::CreateZswapOutput {
            coin: Box::new(Expr::Parameter {
                name: "escaped".into(),
            }),
            recipient: Box::new(Expr::Parameter {
                name: "recipient".into(),
            }),
        },
    });
    assert!(render_with_capabilities(&contract).is_err());
}

#[test]
fn composite_intents_compose_without_public_queries_and_require_exact_typed_operands() {
    let original: Contract =
        serde_json::from_str(include_str!("../stateful-struct-schema20-ir.json")).unwrap();
    let transfer: Contract =
        serde_json::from_str(include_str!("../composite-zswap-transfer-schema20-ir.json")).unwrap();
    for source in [&original, &transfer] {
        let out = render_with_capabilities(source).unwrap();
        assert!(
            out.capabilities
                .circuits
                .iter()
                .all(|c| c.recorded && c.observed_call)
        );
        assert!(out.source.contains("create_zswap_output("));
        assert!(out.source.contains("kernel_self()?"));
        assert!(!out.source.contains(".call_local("));
    }
    for mode in 0..4 {
        let mut invalid = original.clone();
        let planned = invalid
            .stateful_circuits
            .iter_mut()
            .find(|c| c.name == "planned")
            .unwrap();
        let StateReturn::Expression {
            value: Expr::StructLiteral { fields, .. },
        } = &mut planned.return_value
        else {
            panic!("planned literal")
        };
        match mode {
            0 | 1 => {
                let Expr::CreateZswapOutput { coin, recipient } = &mut fields[1] else {
                    panic!("output member")
                };
                if mode == 0 {
                    **coin = Expr::Boolean { value: false };
                } else {
                    **recipient = Expr::Boolean { value: false };
                }
            }
            2 => {
                let Expr::CreateZswapOutput { coin, .. } = &mut fields[1] else {
                    panic!("output member")
                };
                **coin = Expr::Parameter {
                    name: "escaped".into(),
                };
            }
            _ => fields[1] = Expr::Boolean { value: true },
        }
        assert!(
            render_with_capabilities(&invalid).is_err(),
            "malformed effect mode {mode}"
        );
    }
    let mut no_query = original.clone();
    let planned = no_query
        .stateful_circuits
        .iter_mut()
        .find(|c| c.name == "planned")
        .unwrap();
    let StateReturn::Expression {
        value: Expr::StructLiteral { fields, .. },
    } = &mut planned.return_value
    else {
        panic!("planned literal")
    };
    let Expr::KernelSelf { ty } = &fields[2] else {
        panic!("Kernel.self")
    };
    fields[2] = Expr::Default { ty: ty.clone() };
    let out = render_with_capabilities(&no_query).unwrap();
    let planned = out
        .capabilities
        .circuits
        .iter()
        .find(|c| c.name == "planned")
        .unwrap();
    // A checked recording may contain only private intent events. The actual
    // source fixture separately checks EmptyTranscript at preparation time.
    assert!(planned.recorded && planned.observed_call);
    // The algebra composes typed mint and transfer effects without a profile
    // allowlist. This compiler check makes no balanced-offer/proof claim.
    let mut composed = transfer.clone();
    let StateReturn::Expression {
        value: Expr::StructLiteral { fields, .. },
    } = &mut composed.stateful_circuits[0].return_value
    else {
        panic!("transfer literal")
    };
    fields[2] = Expr::KernelMintShielded {
        domain: Box::new(Expr::Parameter {
            name: "nullifier".into(),
        }),
        amount: Box::new(Expr::UnsignedLiteral {
            value: "1".into(),
            max: u64::MAX.to_string(),
        }),
    };
    let out = render_with_capabilities(&composed).unwrap();
    assert!(out.capabilities.circuits[0].recorded);
    assert!(out.capabilities.circuits[0].observed_call);
}
