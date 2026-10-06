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

use crate::environment::{same_call, same_initial};
use crate::runtime::{
    CircuitZswapPlan, CompactError,
    context::{CircuitContext, CircuitResult, ConstructorContext, ConstructorResult},
    ledger::{DefaultDB, QueryContext},
    recording::RecordedCircuitResult,
};
use crate::{
    ArtifactIdentity, CallReport, Environment, LabError, ReplayReport, Snapshot, SnapshotMetadata,
};
use midnight_zswap::local::State as WalletState;

/// One plain-ledger scenario with owned, in-memory checkpoints.
/// Call adapters are trusted: final-boundary checks cannot attest transient policy
/// mutations inside arbitrary Rust closures. Use direct generated-call adapters.
/// P must clone independently or use immutable persistence. External/shared witness
/// side effects are outside the lab's rollback guarantee.
pub struct ContractLab<P> {
    snapshot: Snapshot<P>,
}

fn empty_wallet(wallet: &WalletState<DefaultDB>) -> bool {
    let empty = WalletState::default();
    wallet.coins == empty.coins
        && wallet.pending_spends == empty.pending_spends
        && wallet.pending_outputs == empty.pending_outputs
        && wallet.merkle_tree == empty.merkle_tree
        && wallet.first_free == empty.first_free
}
fn plain<P>(context: &CircuitContext<P>) -> Result<(), LabError> {
    if context.circuit_zswap() != &CircuitZswapPlan::default()
        || !empty_wallet(&context.zswap_state)
    {
        return Err(LabError::UnsupportedState);
    }
    Ok(())
}

impl<P: Clone> ContractLab<P> {
    /// Adopt a generated constructor result after the plain-ledger checks.
    /// Artifact hashes are supplied by the caller, not verified against a filesystem.
    pub fn from_constructor(
        artifacts: ArtifactIdentity,
        environment: Environment,
        initial: ConstructorResult<P>,
    ) -> Result<Self, LabError> {
        if initial.circuit_zswap() != &CircuitZswapPlan::default()
            || !empty_wallet(&initial.zswap_state)
        {
            return Err(LabError::UnsupportedState);
        }
        Ok(Self {
            snapshot: Snapshot {
                metadata: SnapshotMetadata::new(artifacts),
                environment,
                state: initial.ledger_state,
                private: initial.private_state,
            },
        })
    }
    /// Clone an owned checkpoint; no serialization or secret persistence is performed.
    pub fn snapshot(&self) -> Snapshot<P> {
        self.snapshot.clone()
    }
    /// Create an independent scenario when the private state has owned clone semantics.
    pub fn fork(&self) -> Self {
        Self {
            snapshot: self.snapshot(),
        }
    }
    pub fn environment(&self) -> &Environment {
        &self.snapshot.environment
    }
    pub fn private_state(&self) -> &P {
        &self.snapshot.private
    }
    /// Restore only a matching artifact, runtime, state mode and execution environment.
    pub fn restore(&mut self, snapshot: &Snapshot<P>) -> Result<(), LabError> {
        snapshot.metadata.validate(&self.snapshot.metadata)?;
        if !snapshot.environment.matches(&self.snapshot.environment) {
            return Err(LabError::SnapshotMismatch("environment"));
        }
        self.snapshot = snapshot.clone();
        Ok(())
    }
    fn context(&self) -> CircuitContext<P> {
        let context = ConstructorResult::new(
            ConstructorContext::new(self.snapshot.private.clone()),
            self.snapshot.state.clone(),
        )
        .into_circuit_context(self.snapshot.environment.address);
        self.snapshot.environment.apply(context)
    }
    fn validate_result<O>(
        &self,
        initial: &QueryContext<DefaultDB>,
        result: &CircuitResult<P, O>,
    ) -> Result<(), LabError> {
        plain(&result.context)?;
        let environment = &self.snapshot.environment;
        if result.context.query.address != initial.address
            || !same_call(&result.context.query.call_context, &initial.call_context)
            || result.context.own_coin_public_key().ok() != environment.coin_public_key
            || result.context.cost_model != environment.cost_model
            || result.context.gas_limit != environment.query_gas_limit
        {
            return Err(LabError::EnvironmentChanged);
        }
        Ok(())
    }
    fn commit<O>(
        &mut self,
        result: CircuitResult<P, O>,
        replay: Option<ReplayReport>,
    ) -> CallReport<O> {
        let before = self.snapshot.state.clone();
        self.snapshot.state = result.context.query.state;
        self.snapshot.private = result.context.private_state;
        CallReport {
            output: result.result,
            before,
            after: self.snapshot.state.clone(),
            effects: result.context.query.effects,
            private_outputs: result.private_transcript_outputs,
            execution_gas: result.gas_cost,
            replay,
        }
    }
    /// Execute a typed generated native call and commit only its successful result.
    /// This does not certify a public transcript or accept a ledger transaction.
    pub fn native<O>(
        &mut self,
        call: impl FnOnce(CircuitContext<P>) -> Result<CircuitResult<P, O>, CompactError>,
    ) -> Result<CallReport<O>, LabError> {
        let context = self.context();
        let initial = context.query.clone();
        let result = call(context).map_err(LabError::Execution)?;
        self.validate_result(&initial, &result)?;
        Ok(self.commit(result, None))
    }
    /// Execute a typed generated recorded call, then replay its sealed public VM
    /// program before committing. Replay failure never falls back to native success.
    /// An empty local replay is valid here, unlike proof-call preparation.
    pub fn recorded<O>(
        &mut self,
        call: impl FnOnce(CircuitContext<P>) -> Result<RecordedCircuitResult<P, O>, CompactError>,
    ) -> Result<CallReport<O>, LabError> {
        let context = self.context();
        let initial = context.query.clone();
        let recorded = call(context).map_err(LabError::Execution)?;
        if !same_initial(recorded.public.initial(), &initial) {
            return Err(LabError::InitialContextMismatch);
        }
        self.validate_result(&initial, &recorded.execution)?;
        if recorded.public.initial_intents() != &CircuitZswapPlan::default()
            || recorded.public.final_intents() != &CircuitZswapPlan::default()
        {
            return Err(LabError::UnsupportedState);
        }
        let identity = self.snapshot.environment.coin_public_key;
        if recorded.public.initial_coin_public_key() != identity
            || recorded.public.final_coin_public_key() != identity
        {
            return Err(LabError::IdentityMismatch);
        }
        let replay = recorded
            .public
            .initial()
            .query(
                recorded.public.verify_ops(),
                None,
                &self.snapshot.environment.cost_model,
            )
            .map_err(|error| LabError::Replay(format!("{error:?}")))?;
        if replay.context.effects != recorded.execution.context.query.effects {
            return Err(LabError::ReplayEffectsMismatch);
        }
        if replay.context.state.get_ref() != recorded.execution.context.query.state.get_ref() {
            return Err(LabError::ReplayStateMismatch);
        }
        let evidence = ReplayReport {
            program: recorded.public.verify_ops().to_vec(),
            gas: replay.gas_cost,
        };
        Ok(self.commit(recorded.execution, Some(evidence)))
    }
}
