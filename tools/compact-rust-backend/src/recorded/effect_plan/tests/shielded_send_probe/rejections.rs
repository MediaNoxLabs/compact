// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Malformed source remains refused; it is never counted as old inclusion.
use super::*;
fn rejected(contract: &Contract, label: &str) {
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
    let root = circuits["send_to_self"];
    assert!(
        matches!(
            crate::recorded::typed_plan::lower_shielded_send_checked(
                root, &ledger, &witnesses, &pure, &circuits
            ),
            ProfileAttempt::Rejected(_)
        ),
        "{label}: old malformed-source refusal"
    );
    assert!(
        matches!(
            check_with_calls(root, &ledger, &witnesses, &pure, &circuits),
            Err(PlanError::Unsupported(_))
        ),
        "{label}: source refusal must not become an invariant failure"
    );
}
#[test]
fn complete_old_send_and_checked_lowering_refuse_malformed_calls_and_claims() {
    let mut contract = source();
    let Expr::Call { arguments, .. } = expression(root_mut(&mut contract, "send_to_self")) else {
        unreachable!()
    };
    arguments.pop();
    rejected(&contract, "wrong-arity");
    let mut contract = source();
    *expression(root_mut(&mut contract, "send_to_self")) =
        call("send_to_self", vec![reference("input"), reference("value")]);
    rejected(&contract, "recursive-root");
    let mut contract = source();
    let root = root_mut(&mut contract, "send_to_self");
    let value = expression(root).clone();
    *expression(root) = Expr::Sequence {
        steps: vec![Expr::KernelClaim {
            claim: KernelClaimKind::Nullifier,
            value: Box::new(Expr::BytesLiteral { bytes: vec![1; 31] }),
        }],
        value: Box::new(value),
    };
    rejected(&contract, "wrong-claim-carrier");
    let mut contract = source();
    prepend(
        &mut contract,
        vec![local("bad", Type::Field, reference("escaped_local"))],
    );
    rejected(&contract, "escaped-source-binding");
    let mut contract = source();
    let helper = root_mut(&mut contract, "sendShielded");
    helper.parameters[1].name = helper.parameters[0].name.clone();
    rejected(&contract, "duplicate-helper-formals");
}
