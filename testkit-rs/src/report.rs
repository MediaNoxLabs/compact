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

use crate::runtime::{
    context::RunningCost,
    fab::AlignedValue,
    ledger::{ChargedState, DefaultDB},
};
use midnight_onchain_runtime::context::Effects;
use midnight_onchain_vm::{ops::Op, result_mode::ResultModeVerify};
use std::fmt;

/// Local VM replay evidence. This is not proof verification or ledger admission.
pub struct ReplayReport {
    pub(crate) program: Vec<Op<ResultModeVerify, DefaultDB>>,
    pub(crate) gas: RunningCost,
}
impl ReplayReport {
    pub fn program(&self) -> &[Op<ResultModeVerify, DefaultDB>] {
        &self.program
    }
    pub fn gas(&self) -> RunningCost {
        self.gas
    }
}
impl fmt::Debug for ReplayReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ReplayReport")
            .field("operations", &self.program.len())
            .field("gas", &self.gas)
            .finish()
    }
}

/// Explicit accessors retain values; default diagnostics expose only counts/costs.
pub struct CallReport<O> {
    pub(crate) output: O,
    pub(crate) before: ChargedState<DefaultDB>,
    pub(crate) after: ChargedState<DefaultDB>,
    pub(crate) effects: Effects<DefaultDB>,
    pub(crate) private_outputs: Vec<AlignedValue>,
    pub(crate) execution_gas: RunningCost,
    pub(crate) replay: Option<ReplayReport>,
}
impl<O> CallReport<O> {
    pub fn output(&self) -> &O {
        &self.output
    }
    pub fn before(&self) -> &ChargedState<DefaultDB> {
        &self.before
    }
    pub fn public_state(&self) -> &ChargedState<DefaultDB> {
        &self.after
    }
    pub fn effects(&self) -> &Effects<DefaultDB> {
        &self.effects
    }
    /// Private transcript payloads require deliberate access and safe handling.
    pub fn private_outputs(&self) -> &[AlignedValue] {
        &self.private_outputs
    }
    /// Generated query-summed gas, including metered witness reads.
    pub fn execution_gas(&self) -> RunningCost {
        self.execution_gas
    }
    pub fn replay(&self) -> Option<&ReplayReport> {
        self.replay.as_ref()
    }
}
impl<O> fmt::Debug for CallReport<O> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CallReport")
            .field("private_output_count", &self.private_outputs.len())
            .field("execution_gas", &self.execution_gas)
            .field("replay", &self.replay)
            .finish_non_exhaustive()
    }
}
