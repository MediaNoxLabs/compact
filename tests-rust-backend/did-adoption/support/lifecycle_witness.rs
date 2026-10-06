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

use compact_rust_did_adoption_fixture::ledger_contract as c;
use midnight_compact_runtime::{
    BoundedUint, CompactError, Field, JubjubPoint, WideUint, context::WitnessContext,
};
use serde_json::{Value, json};
use std::cell::RefCell;
type Uint248 =
    WideUint<1329227995784915872903807060280344575, 340282366920938463463374607431768211455>;
pub struct Witness {
    pub options: Value,
    pub calls: RefCell<Vec<Value>>,
}
impl Witness {
    pub fn new(options: Value) -> Self {
        Self {
            options,
            calls: RefCell::default(),
        }
    }
    fn call(&self, name: &str) -> Result<(), CompactError> {
        self.calls.borrow_mut().push(json!({"name":name}));
        if self.options["fail"] == name {
            return Err(CompactError::AssertionFailed(format!("{name} failure")));
        }
        Ok(())
    }
}
impl c::TryWitnesses<u64> for Witness {
    fn getSchnorrReduction(
        &self,
        ctx: WitnessContext<'_, u64, c::LedgerView<'_>>,
        hash: Field,
    ) -> Result<(u64, (BoundedUint<127>, Uint248)), CompactError> {
        self.calls
            .borrow_mut()
            .push(json!({"name":"reduction","hash":hex::encode(hash.as_le_bytes())}));
        if self.options["fail"] == "reduction" {
            return Err(CompactError::AssertionFailed("reduction failure".into()));
        }
        let b = hash.as_le_bytes();
        Ok((
            *ctx.private_state + 1,
            (
                BoundedUint::new(b[31] as u128)?,
                Uint248::from_le_bytes(&b[..31])?,
            ),
        ))
    }
    fn currentTimestamp(
        &self,
        ctx: WitnessContext<'_, u64, c::LedgerView<'_>>,
    ) -> Result<(u64, BoundedUint<18446744073709551615>), CompactError> {
        self.call("timestamp")?;
        Ok((
            *ctx.private_state + 1,
            BoundedUint::new(
                self.options["timestamp"]
                    .as_str()
                    .unwrap_or("100")
                    .parse()
                    .unwrap(),
            )?,
        ))
    }
    fn localControllerPublicKey(
        &self,
        ctx: WitnessContext<'_, u64, c::LedgerView<'_>>,
    ) -> Result<(u64, JubjubPoint), CompactError> {
        self.call("controller")?;
        Ok((*ctx.private_state + 1, super::codec::point("1")))
    }
    fn localRecoveryAuthorityPublicKey(
        &self,
        ctx: WitnessContext<'_, u64, c::LedgerView<'_>>,
    ) -> Result<(u64, JubjubPoint), CompactError> {
        self.call("recovery")?;
        Ok((
            *ctx.private_state + 1,
            super::codec::point(if self.options["sameKeys"] == true {
                "1"
            } else {
                "3"
            }),
        ))
    }
}
