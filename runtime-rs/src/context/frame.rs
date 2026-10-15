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

//! Owned native circuit sequencing, witness outputs and accumulated query cost.

use midnight_base_crypto::fab::AlignedValue;

use super::{CircuitContext, CircuitResult, RunningCost, WitnessReadMeter};
use crate::CompactError;
use crate::ledger::{DB, DefaultDB};

/// Accumulates native circuit steps while leaving Compact control flow in Rust.
///
/// Each `apply` consumes the current context and adopts exactly one successor
/// result. A nested circuit can be passed to `apply` without special merging
/// code in the generated body. Public Verify operations require a separate
/// `RecordingFrame` when a replayable call is needed.
pub struct CircuitFrame<Private, D: DB = DefaultDB> {
    pub(super) context: CircuitContext<Private, D>,
    pub(super) gas_cost: RunningCost,
    pub(super) private_outputs: Vec<AlignedValue>,
}

impl<Private, D: DB> CircuitFrame<Private, D> {
    pub fn new(context: CircuitContext<Private, D>) -> Self {
        Self {
            context,
            gas_cost: RunningCost::ZERO,
            private_outputs: Vec::new(),
        }
    }

    pub fn context(&self) -> &CircuitContext<Private, D> {
        &self.context
    }

    /// Invoke a witness against the current state and append its private FAB.
    pub fn witness<T, F>(mut self, call: F) -> (Self, T)
    where
        T: Clone,
        AlignedValue: From<T>,
        F: FnOnce(&CircuitContext<Private, D>) -> (Private, T),
    {
        let (next_private, value) = call(&self.context);
        self.context.private_state = next_private;
        self.private_outputs.push(AlignedValue::from(value.clone()));
        (self, value)
    }

    /// Invoke a witness with a ledger view that meters its projected reads.
    pub fn witness_metered<T, F>(mut self, call: F) -> (Self, T)
    where
        T: Clone,
        AlignedValue: From<T>,
        F: FnOnce(&CircuitContext<Private, D>, &WitnessReadMeter<'_, D>) -> (Private, T),
    {
        let meter = WitnessReadMeter::new(&self.context);
        let (next_private, value) = call(&self.context, &meter);
        self.gas_cost += meter.gas_cost();
        self.context.private_state = next_private;
        self.private_outputs.push(AlignedValue::from(value.clone()));
        (self, value)
    }

    /// Invoke a fallible witness and adopt its private effects only on success.
    pub fn try_witness_metered<T, F>(mut self, call: F) -> Result<(Self, T), CompactError>
    where
        T: Clone,
        AlignedValue: From<T>,
        F: FnOnce(
            &CircuitContext<Private, D>,
            &WitnessReadMeter<'_, D>,
        ) -> Result<(Private, T), CompactError>,
    {
        let meter = WitnessReadMeter::new(&self.context);
        let (next_private, value) = call(&self.context, &meter)?;
        self.gas_cost += meter.gas_cost();
        self.context.private_state = next_private;
        self.private_outputs.push(AlignedValue::from(value.clone()));
        Ok((self, value))
    }

    /// Record exact opaque-bearing witness bytes, retaining ordinary callback
    /// ordering and success-only adoption. Preparation checks transport validity.
    pub fn try_witness_metered_exact<T, F>(self, call: F) -> Result<(Self, T), CompactError>
    where
        T: Clone + crate::HashInputValue,
        F: FnOnce(
            &CircuitContext<Private, D>,
            &WitnessReadMeter<'_, D>,
        ) -> Result<(Private, T), CompactError>,
    {
        let (frame, value) = self.try_witness_metered(|context, meter| {
            call(context, meter).map(|(private, value)| (private, crate::exact_hash_input(value)))
        })?;
        Ok((frame, value.into_inner()))
    }

    /// Run a native ledger step or complete nested circuit in sequence.
    pub fn apply<T, F>(self, operation: F) -> Result<(Self, T), CompactError>
    where
        F: FnOnce(CircuitContext<Private, D>) -> Result<CircuitResult<Private, T, D>, CompactError>,
    {
        let CircuitFrame {
            context,
            mut gas_cost,
            mut private_outputs,
        } = self;
        let result = operation(context)?;
        gas_cost += result.gas_cost;
        private_outputs.extend(result.private_transcript_outputs);
        Ok((
            Self {
                context: result.context,
                gas_cost,
                private_outputs,
            },
            result.result,
        ))
    }

    pub fn finish<T>(self, result: T) -> CircuitResult<Private, T, D> {
        CircuitResult {
            context: self.context,
            result,
            gas_cost: self.gas_cost,
            private_transcript_outputs: self.private_outputs,
        }
    }
}
