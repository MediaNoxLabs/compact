// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use compact_rust_effect_plan_optional_send_source_fixture::{
    ledger_contract as c, runtime as r, types,
};
use midnight_coin_structure::transfer::SenderEvidence;
use r::context::{CircuitContext, ConstructorContext};
use r::ledger::{CoinRecipient, KernelClaim};
use r::recording::{RecordedCircuitResult, RecordingFrame};
pub fn bytes(first: u8) -> r::FixedBytes<32> {
    let mut bytes = [0; 32];
    bytes[0] = first;
    r::FixedBytes::new(bytes)
}
pub fn address() -> r::ledger::ContractAddress {
    r::ledger::ContractAddress(r::ledger::HashOutput(bytes(9).into_array()))
}
pub fn coin() -> types::QualifiedShieldedCoinInfo {
    types::QualifiedShieldedCoinInfo {
        nonce: bytes(1),
        color: bytes(2),
        value: r::BoundedUint::new(42).unwrap(),
        mt_index: r::BoundedUint::new(0).unwrap(),
    }
}
pub fn recipient(kind: u8) -> types::Either {
    types::Either {
        is_left: kind == 7,
        left: types::ZswapCoinPublicKey {
            bytes: bytes(if kind == 7 { 7 } else { 0 }),
        },
        right: types::ContractAddress {
            bytes: bytes(if kind == 7 { 0 } else { kind }),
        },
    }
}
pub fn context() -> CircuitContext<()> {
    let mut ctx = c::initial_state(ConstructorContext::new(()))
        .unwrap()
        .into_circuit_context(address())
        .with_coin_public_key_bytes(bytes(7).into_array());
    ctx.set_zswap_output_start(1).unwrap();
    ctx
}
fn nonce(domain: &[u8]) -> r::FixedBytes<32> {
    let mut tag = [0; 32];
    tag[..domain.len()].copy_from_slice(domain);
    let tag = r::Field::from_le_bytes(&tag).unwrap();
    let hash = midnight_transient_crypto::hash::transient_hash(&[
        tag,
        midnight_transient_crypto::hash::degrade_to_transient(r::ledger::HashOutput(
            bytes(1).into_array(),
        )),
    ]);
    r::FixedBytes::new(midnight_transient_crypto::hash::upgrade_from_transient(hash).0)
}
fn native_coin(coin: &types::ShieldedCoinInfo) -> r::ledger::CoinInfo {
    r::ledger::coin_info_from_compact(coin.nonce, coin.color, coin.value.value())
}
pub fn manual(
    ctx: CircuitContext<()>,
    selected: bool,
    kind: u8,
    amount: u128,
) -> Result<RecordedCircuitResult<(), types::ShieldedSendResult>, r::CompactError> {
    let frame = RecordingFrame::new(ctx);
    if !selected {
        return Ok(frame.finish(types::ShieldedSendResult::default()));
    }
    let (mut frame, self_address) = frame.kernel_self()?;
    let input = r::ledger::coin_info_from_compact(bytes(1), bytes(2), 42);
    frame = frame.create_zswap_input(input.qualify(0));
    frame = frame.kernel_claim(KernelClaim::Nullifier(
        input.nullifier(&SenderEvidence::Contract(self_address)),
    ))?;
    let remainder = 42_u128
        .checked_sub(amount)
        .ok_or(r::CompactError::UnsignedUnderflow)?;
    let destination = if kind == 7 {
        CoinRecipient::User(r::ledger::CoinPublicKey(r::ledger::HashOutput(
            bytes(7).into_array(),
        )))
    } else {
        CoinRecipient::Contract(r::ledger::ContractAddress(r::ledger::HashOutput(
            bytes(kind).into_array(),
        )))
    };
    let sent = types::ShieldedCoinInfo {
        nonce: nonce(b"midnight:kernel:nonce_evolve"),
        color: bytes(2),
        value: r::BoundedUint::new(amount)?,
    };
    let sent_info = native_coin(&sent);
    let commitment = sent_info.commitment(&destination);
    frame = frame.create_zswap_output(sent_info, destination)?;
    frame = frame.kernel_claim(KernelClaim::CoinSpend(commitment))?;
    if kind == 9 {
        frame = frame.kernel_claim(KernelClaim::CoinReceive(commitment))?;
    }
    let mut change = types::Maybe::default();
    if remainder != 0 {
        let value = types::ShieldedCoinInfo {
            nonce: nonce(b"midnight:kernel:nonce_evolve/2"),
            color: bytes(2),
            value: r::BoundedUint::new(remainder)?,
        };
        let info = native_coin(&value);
        let destination = CoinRecipient::Contract(self_address);
        let commitment = info.commitment(&destination);
        frame = frame
            .create_zswap_output(info, destination)?
            .kernel_claim(KernelClaim::CoinSpend(commitment))?
            .kernel_claim(KernelClaim::CoinReceive(commitment))?;
        change = types::Maybe {
            is_some: true,
            value,
        };
    }
    Ok(frame.finish(types::ShieldedSendResult { change, sent }))
}
pub fn compare(
    native: &r::context::CircuitResult<(), types::ShieldedSendResult>,
    recorded: &RecordedCircuitResult<(), types::ShieldedSendResult>,
    manual: &RecordedCircuitResult<(), types::ShieldedSendResult>,
) {
    for actual in [&recorded.execution, &manual.execution] {
        assert_eq!(actual.result, native.result);
        assert_eq!(actual.context.query.state, native.context.query.state);
        assert_eq!(actual.context.query.effects, native.context.query.effects);
        assert_eq!(
            actual.context.circuit_zswap(),
            native.context.circuit_zswap()
        );
        assert_eq!(actual.gas_cost, native.gas_cost);
        assert_eq!(
            actual.private_transcript_outputs,
            native.private_transcript_outputs
        );
    }
    assert_eq!(recorded.public.verify_ops(), manual.public.verify_ops());
    let replay = recorded
        .public
        .initial()
        .query(
            recorded.public.verify_ops(),
            None,
            &native.context.cost_model,
        )
        .unwrap();
    let reference = manual
        .public
        .initial()
        .query(manual.public.verify_ops(), None, &native.context.cost_model)
        .unwrap();
    assert_eq!(replay.context.state, native.context.query.state);
    assert_eq!(replay.context.effects, native.context.query.effects);
    assert_eq!(replay.gas_cost, reference.gas_cost);
}
