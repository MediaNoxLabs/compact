// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;
use std::{cell::RefCell, rc::Rc};
#[derive(Clone, Copy, Default)]
pub(super) enum Behavior {
    #[default]
    Honest,
    Refuse,
    BadReduction,
    QuotientBound,
}
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Event {
    pub private: u64,
    pub challenge: Field,
    pub count: Option<u64>,
}
#[derive(Clone, Default)]
pub(super) struct Provider {
    pub events: Rc<RefCell<Vec<Event>>>,
    pub behavior: Behavior,
    pub inspect_counter: bool,
}
impl Provider {
    pub fn reduce(
        &self,
        private: u64,
        challenge: Field,
        count: Option<u64>,
    ) -> Result<(u64, (BoundedUint<127>, Uint248)), r::CompactError> {
        self.events.borrow_mut().push(Event {
            private,
            challenge,
            count,
        });
        let bytes = challenge.as_le_bytes();
        let honest = (
            BoundedUint::new(u128::from(bytes[31])).unwrap(),
            Uint248::from_le_bytes(&bytes[..31]).unwrap(),
        );
        let result = match self.behavior {
            Behavior::Honest => honest,
            Behavior::Refuse => {
                return Err(r::CompactError::InvalidLedgerCell(
                    "reduction provider refused".into(),
                ));
            }
            Behavior::BadReduction => (
                BoundedUint::new(0).unwrap(),
                Uint248::from_le_bytes(&[0; 31]).unwrap(),
            ),
            Behavior::QuotientBound => (BoundedUint::new(116).unwrap(), honest.1),
        };
        Ok((private + 1, result))
    }
}
impl c::TryWitnesses<u64> for Provider {
    fn getSchnorrReduction(
        &self,
        ctx: WitnessContext<'_, u64, LedgerView<'_>>,
        challenge: Field,
    ) -> Result<(u64, (BoundedUint<127>, Uint248)), r::CompactError> {
        let count = if self.inspect_counter {
            Some(ctx.ledger.acceptedCount()?.value() as u64)
        } else {
            None
        };
        self.reduce(*ctx.private_state, challenge, count)
    }
    fn localAttestorKey(
        &self,
        _ctx: WitnessContext<'_, u64, LedgerView<'_>>,
    ) -> Result<(u64, JubjubPoint), r::CompactError> {
        Err(r::CompactError::InvalidLedgerCell(
            "unexpected constructor callback".into(),
        ))
    }
}
