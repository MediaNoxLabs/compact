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

//! Native intent effects keep their private Unit entries and emit no VM ops.
use super::*;

impl<Private, D: DB> RecordingFrame<Private, D> {
    pub fn create_zswap_input(mut self, coin: ledger::QualifiedCoinInfo) -> Self {
        self.context.create_zswap_input(coin);
        self.private_outputs.push(().into());
        self
    }

    pub fn create_zswap_output(
        mut self,
        coin: ledger::CoinInfo,
        recipient: ledger::CoinRecipient,
    ) -> Result<Self, CompactError> {
        self.context.create_zswap_output(coin, recipient)?;
        self.private_outputs.push(().into());
        Ok(self)
    }
}
