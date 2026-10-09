// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

use compact_rust_effect_plan_kernel_source_fixture::{
    ledger_contract as c, ledger_slots, runtime as r,
};
use r::context::{CircuitContext, ConstructorContext, WitnessContext};
use r::ledger::{CoinCommitment, CoinNullifier, HashOutput, KernelClaim};
use r::recording::{RecordedCircuitResult, RecordingFrame};
use r::{CompactError, Field, FixedBytes};
use std::cell::RefCell;
type Amount = r::BoundedUint<18446744073709551615>;
fn context() -> CircuitContext<u64> {
    c::initial_state(ConstructorContext::new(9))
        .unwrap()
        .into_circuit_context(Default::default())
}
#[derive(Default)]
struct Witness {
    calls: RefCell<Vec<(&'static str, u64)>>,
    fail_at: Option<&'static str>,
    amount: u64,
}
impl Witness {
    fn call(&self, name: &'static str, private: u64) -> Result<u64, CompactError> {
        self.calls.borrow_mut().push((name, private));
        if self.fail_at == Some(name) {
            return Err(CompactError::AssertionFailed(format!("{name} refused")));
        }
        Ok(private + 1)
    }
    fn domain_value(&self, private: u64) -> Result<(u64, FixedBytes<32>), CompactError> {
        Ok((self.call("domain", private)?, FixedBytes::new([3; 32])))
    }
    fn amount_value(&self, private: u64) -> Result<(u64, Amount), CompactError> {
        Ok((
            self.call("amount", private)?,
            Amount::new(self.amount as u128)?,
        ))
    }
}
impl c::TryWitnesses<u64> for Witness {
    fn domain(
        &self,
        ctx: WitnessContext<'_, u64, c::LedgerView<'_>>,
    ) -> Result<(u64, FixedBytes<32>), CompactError> {
        self.domain_value(*ctx.private_state)
    }
    fn amount(
        &self,
        ctx: WitnessContext<'_, u64, c::LedgerView<'_>>,
    ) -> Result<(u64, Amount), CompactError> {
        self.amount_value(*ctx.private_state)
    }
}
fn reference(
    selected: bool,
    value: FixedBytes<32>,
    witness: &Witness,
) -> Result<RecordedCircuitResult<u64, ()>, CompactError> {
    let frame =
        ledger_slots::marker.record_write(RecordingFrame::new(context()), Field::from(1_u64))?;
    let frame = frame.kernel_claim(KernelClaim::Nullifier(CoinNullifier(HashOutput(value.0))))?;
    let frame = if selected {
        let (frame, domain) =
            frame.try_witness_metered(|ctx, _| witness.domain_value(ctx.private_state))?;
        let (frame, amount) =
            frame.try_witness_metered(|ctx, _| witness.amount_value(ctx.private_state))?;
        frame.kernel_mint_shielded(HashOutput(domain.0), amount.value() as u64)?
    } else {
        frame.kernel_claim(KernelClaim::CoinReceive(CoinCommitment(HashOutput(
            value.0,
        ))))?
    };
    Ok(frame.finish(()))
}
#[test]
fn composed_cell_claim_branch_and_witnessed_mint_match_official_sequence() {
    for selected in [false, true] {
        for amount in [0, 13, u64::MAX] {
            let a = Witness {
                amount,
                ..Default::default()
            };
            let b = Witness {
                amount,
                ..Default::default()
            };
            let m = Witness {
                amount,
                ..Default::default()
            };
            let value = FixedBytes::new([7; 32]);
            let native = c::compose(context(), &a, selected, value).unwrap();
            let recorded = c::recorded::compose(context(), &b, selected, value).unwrap();
            let manual = reference(selected, value, &m).unwrap();
            let expected_calls = if selected {
                vec![("domain", 9), ("amount", 10)]
            } else {
                vec![]
            };
            assert_eq!(*a.calls.borrow(), expected_calls);
            assert_eq!(*b.calls.borrow(), expected_calls);
            assert_eq!(*m.calls.borrow(), expected_calls);
            assert_eq!(
                ledger_slots::marker
                    .inspect(native.context.query.state.get_ref())
                    .unwrap(),
                Field::from(1_u64)
            );
            for execution in [&recorded.execution, &manual.execution] {
                assert_eq!(execution.context.query.state, native.context.query.state);
                assert_eq!(
                    execution.context.query.effects,
                    native.context.query.effects
                );
                assert_eq!(execution.gas_cost, native.gas_cost);
                assert_eq!(
                    execution.context.private_state,
                    if selected { 11 } else { 9 }
                );
                assert_eq!(
                    execution.private_transcript_outputs,
                    native.private_transcript_outputs
                );
            }
            assert_eq!(
                native.private_transcript_outputs.len(),
                if selected { 2 } else { 0 }
            );
            assert_eq!(
                serde_json::to_value(recorded.public.verify_ops()).unwrap(),
                serde_json::to_value(manual.public.verify_ops()).unwrap()
            );
            let replay = recorded
                .public
                .initial()
                .query(
                    recorded.public.verify_ops(),
                    None,
                    &recorded.execution.context.cost_model,
                )
                .unwrap();
            let expected = manual
                .public
                .initial()
                .query(
                    manual.public.verify_ops(),
                    None,
                    &manual.execution.context.cost_model,
                )
                .unwrap();
            assert_eq!(replay.context.state, native.context.query.state);
            assert_eq!(replay.context.effects, native.context.query.effects);
            assert_eq!(replay.gas_cost, expected.gas_cost);
        }
    }
}
#[test]
fn witness_failures_keep_once_only_left_to_right_calls_after_prior_effects() {
    for fail_at in ["domain", "amount"] {
        let a = Witness {
            fail_at: Some(fail_at),
            ..Default::default()
        };
        let b = Witness {
            fail_at: Some(fail_at),
            ..Default::default()
        };
        let m = Witness {
            fail_at: Some(fail_at),
            ..Default::default()
        };
        let value = FixedBytes::new([7; 32]);
        let expected = CompactError::AssertionFailed(format!("{fail_at} refused"));
        assert_eq!(
            c::compose(context(), &a, true, value).err().unwrap(),
            expected
        );
        assert_eq!(
            c::recorded::compose(context(), &b, true, value)
                .err()
                .unwrap(),
            expected
        );
        assert_eq!(reference(true, value, &m).err().unwrap(), expected);
        let calls = if fail_at == "domain" {
            vec![("domain", 9)]
        } else {
            vec![("domain", 9), ("amount", 10)]
        };
        assert_eq!(*a.calls.borrow(), calls);
        assert_eq!(*b.calls.borrow(), calls);
        assert_eq!(*m.calls.borrow(), calls);
        // Err returns no partial public frame; this asserts witness prefix/error only.
    }
}
