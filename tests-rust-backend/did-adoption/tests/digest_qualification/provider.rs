// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;
use r::context::WitnessContext;
type Uint248 =
    r::WideUint<1329227995784915872903807060280344575, 340282366920938463463374607431768211455>;
#[derive(Clone)]
pub(super) struct Provider {
    fail: bool,
    pub calls: Rc<RefCell<Vec<Value>>>,
    pub private: Rc<RefCell<Vec<u64>>>,
}
impl Provider {
    pub fn new(fail: bool) -> Self {
        Self {
            fail,
            calls: Default::default(),
            private: Default::default(),
        }
    }
    pub fn reduce(
        &self,
        private: u64,
        hash: r::Field,
    ) -> Result<(u64, (r::BoundedUint<127>, Uint248)), r::CompactError> {
        self.calls
            .borrow_mut()
            .push(json!({"name":"reduction","hash":hex::encode(hash.as_le_bytes())}));
        self.private.borrow_mut().push(private);
        if self.fail {
            return Err(r::CompactError::AssertionFailed("reduction failure".into()));
        }
        let bytes = hash.as_le_bytes();
        Ok((
            private + 1,
            (
                r::BoundedUint::new(bytes[31] as u128)?,
                Uint248::from_le_bytes(&bytes[..31])?,
            ),
        ))
    }
}
impl c::TryWitnesses<u64> for Provider {
    fn getSchnorrReduction(
        &self,
        ctx: WitnessContext<'_, u64, c::LedgerView<'_>>,
        hash: r::Field,
    ) -> Result<(u64, (r::BoundedUint<127>, Uint248)), r::CompactError> {
        self.reduce(*ctx.private_state, hash)
    }
    fn currentTimestamp(
        &self,
        _: WitnessContext<'_, u64, c::LedgerView<'_>>,
    ) -> Result<(u64, r::BoundedUint<18446744073709551615>), r::CompactError> {
        panic!("digest verification must not call timestamp")
    }
    fn localControllerPublicKey(
        &self,
        _: WitnessContext<'_, u64, c::LedgerView<'_>>,
    ) -> Result<(u64, r::JubjubPoint), r::CompactError> {
        panic!("digest verification must not call controller")
    }
    fn localRecoveryAuthorityPublicKey(
        &self,
        _: WitnessContext<'_, u64, c::LedgerView<'_>>,
    ) -> Result<(u64, r::JubjubPoint), r::CompactError> {
        panic!("digest verification must not call recovery")
    }
}
