// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use crate::{ledger_contract as c, ledger_slots as slots, runtime as r, types};
use r::context::{CircuitContext, ConstructorContext, WitnessContext};
use r::fab::{Alignment, AlignmentAtom};
use r::recording::{RecordedCircuitResult, RecordingFrame};
use std::{cell::RefCell, rc::Rc};
pub type Wide = r::WideUint<1, { u128::MAX }>;
pub fn wide_bytes() -> Vec<u8> {
    let mut bytes = vec![0; 17];
    bytes[0] = 7;
    bytes[16] = 1;
    bytes
}
pub fn input(text: &str, bytes: &[u8]) -> types::Product {
    types::Product {
        pair: (text.into(), bytes.to_vec().into()),
        items: r::FixedVector::new([vec![2].into(), vec![3].into()]),
        nested: types::Nested {
            tag: r::Field::from(11_u64),
            text: "nested".into(),
        },
        wide: Wide::from_le_bytes(&wide_bytes()).unwrap(),
        empty: types::Empty {},
    }
}
/// Exact known test value layout, independent of the generated product encoder.
pub fn product_atoms(value: &types::Product) -> Vec<Vec<u8>> {
    assert_eq!(value.nested.tag, r::Field::from(11_u64));
    assert_eq!(value.wide.as_le_bytes(), wide_bytes());
    vec![
        value.pair.0.0.as_bytes().to_vec(),
        value.pair.1.0.clone(),
        value.items.0[0].0.clone(),
        value.items.0[1].0.clone(),
        vec![11],
        value.nested.text.0.as_bytes().to_vec(),
        wide_bytes(),
    ]
}
pub fn product_alignment() -> Alignment {
    let opaque = Alignment::singleton(AlignmentAtom::Compress);
    let field = Alignment::singleton(AlignmentAtom::Field);
    let wide = Alignment::singleton(AlignmentAtom::Bytes { length: 17 });
    Alignment::concat([&opaque, &opaque, &opaque, &opaque, &field, &opaque, &wide])
}
#[derive(Clone)]
pub struct Witness {
    pub bytes: r::OpaqueBytes,
    pub calls: Rc<RefCell<Vec<(u64, u128)>>>,
}
impl Witness {
    pub fn new(bytes: &[u8]) -> Self {
        Self {
            bytes: bytes.to_vec().into(),
            calls: Rc::default(),
        }
    }
}
impl c::TryWitnesses<u64> for Witness {
    fn replacement(
        &self,
        ctx: WitnessContext<'_, u64, c::LedgerView<'_>>,
    ) -> Result<(u64, r::OpaqueBytes), r::CompactError> {
        self.calls
            .borrow_mut()
            .push((*ctx.private_state, ctx.ledger.calls()?.value()));
        Ok((*ctx.private_state + 1, self.bytes.clone()))
    }
}
pub fn context() -> CircuitContext<u64> {
    c::initial_state(ConstructorContext::new(5))
        .unwrap()
        .into_circuit_context(Default::default())
}
pub fn expected(
    input: &types::Product,
    selected: bool,
    replacement: &r::OpaqueBytes,
) -> types::Result {
    let mut chosen = input.clone();
    if selected {
        chosen.items = r::FixedVector::new([input.pair.1.clone(), replacement.clone()]);
    }
    types::Result {
        chosen,
        original: input.clone(),
    }
}
pub fn manual(
    context: CircuitContext<u64>,
    input: &types::Product,
    selected: bool,
    witness: &Witness,
) -> Result<RecordedCircuitResult<u64, types::Result>, r::CompactError> {
    let mut frame = slots::calls.record_increment(RecordingFrame::new(context), 1)?;
    let mut bytes = r::OpaqueBytes::default();
    if selected {
        let (next, value) = frame.try_witness_metered(|ctx, meter| {
            let counter = slots::calls.witness_read(meter)? as u128;
            witness
                .calls
                .borrow_mut()
                .push((ctx.private_state, counter));
            Ok((ctx.private_state + 1, witness.bytes.clone()))
        })?;
        frame = next;
        bytes = value;
    }
    Ok(frame.finish(expected(input, selected, &bytes)))
}
