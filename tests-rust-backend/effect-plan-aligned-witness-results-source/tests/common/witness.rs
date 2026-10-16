// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::values::{Payload, Tuple11};
use compact_rust_effect_plan_aligned_witness_results_source_fixture::{
    ledger_contract as c, ledger_slots as slots, runtime as r, types,
};
use r::context::{CircuitContext, WitnessContext};
use r::recording::{RecordedCircuitResult, RecordingFrame};
use std::{cell::RefCell, rc::Rc};
pub type Event = (&'static str, u64, u128);
pub fn provider_failure(name: &str) -> r::CompactError {
    r::CompactError::InvalidLedgerCell(format!("{name} provider refused"))
}
#[derive(Clone)]
pub struct Witness {
    pub payload: Payload,
    pub permit: bool,
    pub fail: Option<&'static str>,
    pub events: Rc<RefCell<Vec<Event>>>,
}
impl Witness {
    pub fn new(row: &serde_json::Value) -> Self {
        Self {
            payload: Payload::from_json(row["circuit"].as_str().unwrap(), &row["payload"]),
            permit: row["permit"].as_bool().unwrap(),
            fail: match row["fail"].as_str() {
                None => None,
                Some("text_value") => Some("text_value"),
                Some("permit") => Some("permit"),
                Some(n) => panic!("unknown failure {n}"),
            },
            events: Rc::default(),
        }
    }
    fn take(&self, name: &'static str, private: u64, calls: u128) -> Result<u64, r::CompactError> {
        self.events.borrow_mut().push((name, private, calls));
        if self.fail == Some(name) {
            Err(provider_failure(name))
        } else {
            Ok(private + 1)
        }
    }
}
impl c::TryWitnesses<u64> for Witness {
    fn text_value(
        &self,
        ctx: WitnessContext<'_, u64, c::LedgerView<'_>>,
    ) -> Result<(u64, r::OpaqueString), r::CompactError> {
        let next = self.take(
            "text_value",
            *ctx.private_state,
            ctx.ledger.calls()?.value(),
        )?;
        let Payload::Text(value) = &self.payload else {
            panic!("unexpected text witness")
        };
        Ok((next, value.clone()))
    }
    fn entry_value(
        &self,
        ctx: WitnessContext<'_, u64, c::LedgerView<'_>>,
    ) -> Result<(u64, types::Entry), r::CompactError> {
        let next = self.take(
            "entry_value",
            *ctx.private_state,
            ctx.ledger.calls()?.value(),
        )?;
        let Payload::Entry(value) = &self.payload else {
            panic!("unexpected Entry witness")
        };
        Ok((next, value.clone()))
    }
    fn nested_value(
        &self,
        ctx: WitnessContext<'_, u64, c::LedgerView<'_>>,
    ) -> Result<(u64, types::Nested), r::CompactError> {
        let next = self.take(
            "nested_value",
            *ctx.private_state,
            ctx.ledger.calls()?.value(),
        )?;
        let Payload::Nested(value) = &self.payload else {
            panic!("unexpected Nested witness")
        };
        Ok((next, value.clone()))
    }
    fn tuple_value(
        &self,
        ctx: WitnessContext<'_, u64, c::LedgerView<'_>>,
    ) -> Result<(u64, Tuple11), r::CompactError> {
        let next = self.take(
            "tuple_value",
            *ctx.private_state,
            ctx.ledger.calls()?.value(),
        )?;
        let Payload::Tuple(value) = &self.payload else {
            panic!("unexpected tuple witness")
        };
        Ok((next, *value.as_ref()))
    }
    fn permit(
        &self,
        ctx: WitnessContext<'_, u64, c::LedgerView<'_>>,
    ) -> Result<(u64, bool), r::CompactError> {
        Ok((
            self.take("permit", *ctx.private_state, ctx.ledger.calls()?.value())?,
            self.permit,
        ))
    }
}
/// Manual state/witness recorder; hash answers come from frozen official TS.
/// Unit helper's pure assertion is true for its explicitly retained "unit" input.
pub fn manual<T>(
    ctx: CircuitContext<u64>,
    w: &Witness,
    selected: bool,
    output: T,
) -> Result<RecordedCircuitResult<u64, T>, r::CompactError> {
    let mut frame = slots::calls.record_increment(RecordingFrame::new(ctx), 1)?;
    if selected {
        macro_rules! record {
            ($value:expr,$method:ident) => {{
                let (next, _) = frame.$method(|ctx, meter| {
                    Ok((
                        w.take(
                            w.payload.name(),
                            ctx.private_state,
                            slots::calls.witness_read(meter)? as u128,
                        )?,
                        $value,
                    ))
                })?;
                next
            }};
        }
        frame = match &w.payload {
            Payload::Text(v) => record!(v.clone(), try_witness_metered_exact),
            Payload::Entry(v) => record!(v.clone(), try_witness_metered_exact),
            Payload::Nested(v) => record!(v.clone(), try_witness_metered_exact),
            Payload::Tuple(v) => record!(*v.as_ref(), try_witness_metered),
        };
    }
    let (frame, permit) = frame.try_witness_metered(|ctx, meter| {
        Ok((
            w.take(
                "permit",
                ctx.private_state,
                slots::calls.witness_read(meter)? as u128,
            )?,
            w.permit,
        ))
    })?;
    if !permit {
        return Err(r::CompactError::AssertionFailed("denied".into()));
    }
    Ok(frame.finish(output))
}
