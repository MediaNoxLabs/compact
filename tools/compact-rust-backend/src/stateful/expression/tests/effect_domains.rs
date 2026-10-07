// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Domain admission at the native effect dispatcher boundary.
use super::*;
use crate::ir::KernelClaimKind;
use syn::visit_mut::{self, VisitMut};

#[derive(Default)]
struct ContextCalls(Vec<String>);
impl VisitMut for ContextCalls {
    fn visit_expr_method_call_mut(&mut self, call: &mut syn::ExprMethodCall) {
        if matches!(call.receiver.as_ref(), syn::Expr::Path(p) if p.path.is_ident("context")) {
            self.0.push(call.method.to_string());
        }
        visit_mut::visit_expr_method_call_mut(self, call);
    }
}
fn context_calls(observation: &mut Observation) -> Vec<String> {
    let mut calls = ContextCalls::default();
    for statement in &mut observation.statements {
        calls.visit_stmt_mut(statement);
    }
    if let Ok((expression, _, _)) = &mut observation.result {
        calls.visit_expr_mut(expression);
    }
    calls.0
}
fn admitted(mut observation: Observation, result: Type, method: &str) {
    assert_eq!(context_calls(&mut observation), [method]);
    observation.accepted(result, false, true);
}
fn refused(mut observation: Observation, expected: Type, actual: Type) {
    assert!(context_calls(&mut observation).is_empty());
    assert!(!observation.query_effect);
    observation.refused(mismatch(expected, actual));
}
fn default_value(ty: Type) -> Box<Expr> {
    Box::new(Expr::Default { ty })
}

#[test]
fn kernel_self_requires_nominal_address_and_exact_bytes_width() {
    let declarations = Declarations::default();
    let address = Type::Struct {
        name: "ContractAddress".into(),
        fields: vec![StructField {
            name: "bytes".into(),
            ty: Type::Bytes { length: 32 },
        }],
    };
    admitted(
        declarations.lower(Expr::KernelSelf {
            ty: address.clone(),
        }),
        address.clone(),
        "kernel_self",
    );
    for bad in [
        Type::Bytes { length: 32 },
        Type::Struct {
            name: "OtherAddress".into(),
            fields: vec![StructField {
                name: "bytes".into(),
                ty: Type::Bytes { length: 32 },
            }],
        },
        Type::Struct {
            name: "ContractAddress".into(),
            fields: vec![StructField {
                name: "bytes".into(),
                ty: Type::Bytes { length: 31 },
            }],
        },
    ] {
        declarations
            .lower(Expr::KernelSelf { ty: bad.clone() })
            .refused_before_effects(mismatch(address.clone(), bad));
    }
}

#[test]
fn kernel_claims_route_each_kind_and_reject_non_digest_operands() {
    let declarations = Declarations::default();
    for (claim, method) in [
        (KernelClaimKind::Nullifier, "kernel_claim_zswap_nullifier"),
        (KernelClaimKind::CoinSpend, "kernel_claim_zswap_coin_spend"),
        (
            KernelClaimKind::CoinReceive,
            "kernel_claim_zswap_coin_receive",
        ),
    ] {
        admitted(
            declarations.lower(Expr::KernelClaim {
                claim,
                value: parameter("bytes"),
            }),
            Type::Unit,
            method,
        );
        for (operand, actual) in [
            ("short_bytes", Type::Bytes { length: 31 }),
            ("field", Type::Field),
        ] {
            refused(
                declarations.lower(Expr::KernelClaim {
                    claim,
                    value: parameter(operand),
                }),
                Type::Bytes { length: 32 },
                actual,
            );
        }
    }
}

#[test]
fn mint_requires_digest_domain_and_exact_uint64_amount() {
    let declarations = Declarations::default();
    let amount = uint(&u64::MAX.to_string());
    admitted(
        declarations.lower(Expr::KernelMintShielded {
            domain: parameter("bytes"),
            amount: default_value(amount.clone()),
        }),
        Type::Unit,
        "kernel_mint_shielded",
    );
    // Domain refusal must precede resolving a nonexistent amount operand.
    declarations
        .lower(Expr::KernelMintShielded {
            domain: parameter("short_bytes"),
            amount: parameter("absent"),
        })
        .refused_before_effects(mismatch(
            Type::Bytes { length: 32 },
            Type::Bytes { length: 31 },
        ));
    for (operand, actual) in [("field", Type::Field), ("uint", uint("255"))] {
        refused(
            declarations.lower(Expr::KernelMintShielded {
                domain: parameter("bytes"),
                amount: parameter(operand),
            }),
            amount.clone(),
            actual,
        );
    }
}

#[test]
fn coin_effects_distinguish_qualified_input_output_and_recipient_domains() {
    let declarations = Declarations::default();
    let input = qualified_coin_type();
    let output = shielded_coin_type();
    let recipient = shielded_recipient_type();
    admitted(
        declarations.lower(Expr::CreateZswapInput {
            coin: default_value(input.clone()),
        }),
        Type::Unit,
        "create_zswap_input",
    );
    refused(
        declarations.lower(Expr::CreateZswapInput {
            coin: default_value(output.clone()),
        }),
        input.clone(),
        output.clone(),
    );
    admitted(
        declarations.lower(Expr::CreateZswapOutput {
            coin: default_value(output.clone()),
            recipient: default_value(recipient.clone()),
        }),
        Type::Unit,
        "create_zswap_output",
    );
    // Invalid coin prevents even looking up the recipient.
    declarations
        .lower(Expr::CreateZswapOutput {
            coin: default_value(input.clone()),
            recipient: parameter("absent"),
        })
        .refused_before_effects(mismatch(output.clone(), input));
    refused(
        declarations.lower(Expr::CreateZswapOutput {
            coin: default_value(output),
            recipient: parameter("bytes"),
        }),
        recipient,
        Type::Bytes { length: 32 },
    );
}
