// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//  http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Recording uses the native Kernel's canonical upstream VM programs.

use super::*;

impl<Private, D: DB> RecordingFrame<Private, D> {
    /// Claim a nullifier or coin commitment in the actual Kernel effects frame.
    /// Ledger transaction validation still requires a matching offer.
    pub fn kernel_claim(self, claim: ledger::KernelClaim) -> Result<Self, CompactError> {
        let program = ledger::kernel_claim_program::<ResultModeVerify, D>(claim);
        self.apply_verify_program(program)
    }

    /// Record a shielded mint effect, without fabricating a coin or a balance.
    pub fn kernel_mint_shielded(
        self,
        domain: ledger::HashOutput,
        amount: u64,
    ) -> Result<Self, CompactError> {
        let program = ledger::kernel_mint_shielded_program::<ResultModeVerify, D>(domain, amount);
        self.apply_verify_program(program)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::{ConstructorContext, ConstructorResult};
    use crate::ledger::{ChargedState, ContractAddress, DefaultDB, StateValue};

    fn context() -> CircuitContext<()> {
        ConstructorResult::new(
            ConstructorContext::new(()),
            ChargedState::new(StateValue::<DefaultDB>::Array(
                vec![ledger::constructor_cell(false)].into(),
            )),
        )
        .into_circuit_context(ContractAddress::default())
    }

    #[test]
    fn recording_keeps_duplicate_claim_ops_and_overflow_prefix() {
        let claim =
            ledger::KernelClaim::Nullifier(ledger::CoinNullifier(ledger::HashOutput([7; 32])));
        let once = RecordingFrame::new(context())
            .kernel_claim(claim)
            .unwrap()
            .finish(());
        let twice = RecordingFrame::new(context())
            .kernel_claim(claim)
            .unwrap()
            .kernel_claim(claim)
            .unwrap()
            .finish(());
        assert_eq!(
            once.execution.context.query.effects,
            twice.execution.context.query.effects
        );
        assert_eq!(
            twice.public.verify_ops().len(),
            2 * once.public.verify_ops().len()
        );
        let domain = ledger::HashOutput([1; 32]);
        let first = RecordingFrame::new(context())
            .kernel_mint_shielded(domain, u64::MAX)
            .unwrap()
            .finish(());
        let prior = first.execution.context.query.effects.clone();
        let mut next = context();
        next.query = first.execution.context.query.clone();
        let error = RecordingFrame::new(next)
            .kernel_mint_shielded(domain, 1)
            .err()
            .unwrap();
        assert!(error.to_string().to_lowercase().contains("overflow"));
        assert_eq!(first.execution.context.query.effects, prior);
        assert_eq!(*prior.shielded_mints.get(&domain).unwrap(), u64::MAX);
        assert_eq!(
            first.public.verify_ops().len(),
            ledger::kernel_mint_shielded_program::<ResultModeVerify, DefaultDB>(domain, u64::MAX)
                .len()
        );
    }

    #[test]
    fn recording_kernel_obeys_zero_budget() {
        let mut ctx = context();
        ctx.gas_limit = Some(RunningCost::ZERO);
        assert!(matches!(
            RecordingFrame::new(ctx).kernel_mint_shielded(ledger::HashOutput([1; 32]), 7),
            Err(CompactError::LedgerQueryRejected(_))
        ));
    }
}
