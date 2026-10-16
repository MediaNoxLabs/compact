// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use crate::{ledger_contract as c, ledger_slots as slots, runtime as r, types};
use r::context::{CircuitContext, ConstructorContext, WitnessContext};
use r::recording::{RecordedCircuitResult, RecordingFrame};
use std::{cell::RefCell, rc::Rc};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Arguments {
    pub bytes: r::OpaqueBytes,
    pub text: r::OpaqueString,
    pub pair: (r::OpaqueBytes, r::OpaqueString),
    pub items: r::FixedVector<r::OpaqueBytes, 2>,
    pub product: types::Product,
}
pub fn arguments(text: &str, bytes: &[u8]) -> Arguments {
    let bytes = r::OpaqueBytes::from(bytes.to_vec());
    let text = r::OpaqueString::from(text);
    let pair = (bytes.clone(), text.clone());
    let items = r::FixedVector::new([bytes.clone(), bytes.clone()]);
    let product = types::Product {
        payload: types::Payload {
            text: text.clone(),
            bytes: bytes.clone(),
        },
        pair: pair.clone(),
        items: items.clone(),
    };
    Arguments {
        bytes,
        text,
        pair,
        items,
        product,
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Event {
    pub private: u64,
    pub counter: u128,
    pub arguments: Arguments,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    False,
    Error,
}
#[derive(Clone, Default)]
pub struct Witness {
    pub events: Rc<RefCell<Vec<Event>>>,
    pub refusal: Option<(u64, Refusal)>,
}
impl Witness {
    pub fn observe(
        &self,
        private: u64,
        counter: u128,
        mut args: Arguments,
    ) -> Result<(u64, bool), r::CompactError> {
        self.events.borrow_mut().push(Event {
            private,
            counter,
            arguments: args.clone(),
        });
        // Mutate every owned callback argument after capture. Later calls must
        // still receive the original values retained in generated source scope.
        args.bytes.0.push(99);
        args.text.0.push('!');
        args.pair.0.0.clear();
        args.pair.1.0.clear();
        args.items.0[0].0.clear();
        args.items.0[1].0.push(99);
        args.product.payload.text.0.clear();
        args.product.payload.bytes.0.clear();
        args.product.pair.0.0.clear();
        args.product.pair.1.0.clear();
        args.product.items.0[0].0.clear();
        args.product.items.0[1].0.push(99);
        match self
            .refusal
            .filter(|(ordinal, _)| *ordinal == private - 6)
            .map(|(_, kind)| kind)
        {
            Some(Refusal::Error) => Err(r::CompactError::AssertionFailed(format!(
                "callback {} refused",
                private - 6
            ))),
            Some(Refusal::False) => Ok((private + 1, false)),
            None => Ok((private + 1, true)),
        }
    }
}
impl c::TryWitnesses<u64> for Witness {
    fn consume(
        &self,
        ctx: WitnessContext<'_, u64, c::LedgerView<'_>>,
        bytes: r::OpaqueBytes,
        text: r::OpaqueString,
        pair: (r::OpaqueBytes, r::OpaqueString),
        items: r::FixedVector<r::OpaqueBytes, 2>,
        product: types::Product,
    ) -> Result<(u64, bool), r::CompactError> {
        self.observe(
            *ctx.private_state,
            ctx.ledger.calls()?.value(),
            Arguments {
                bytes,
                text,
                pair,
                items,
                product,
            },
        )
    }
}
pub fn context() -> CircuitContext<u64> {
    c::initial_state(ConstructorContext::new(7))
        .unwrap()
        .into_circuit_context(Default::default())
}
pub fn manual(
    context: CircuitContext<u64>,
    text: &str,
    bytes: &[u8],
    selected: bool,
    witness: &Witness,
) -> Result<RecordedCircuitResult<u64, types::Accepted>, r::CompactError> {
    let mut frame = RecordingFrame::new(context);
    // Distinct immediates make argument source order observable in the transcript.
    for amount in [1, 2, 3] {
        frame = slots::calls.record_increment(frame, amount)?;
    }
    for ordinal in 1..=if selected { 2 } else { 1 } {
        let (next, accepted) = frame.try_witness_metered(|ctx, meter| {
            let counter = slots::calls.witness_read(meter)? as u128;
            witness.observe(ctx.private_state, counter, arguments(text, bytes))
        })?;
        frame = next;
        if !accepted {
            let message = if ordinal == 1 {
                "first callback refused"
            } else {
                "second callback refused"
            };
            return Err(r::CompactError::AssertionFailed(message.into()));
        }
    }
    Ok(frame.finish(types::Accepted { value: true }))
}
