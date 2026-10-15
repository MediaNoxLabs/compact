// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Existing Cell adapters backed by the official ledger; manual source program.
use crate::{ledger_slots, runtime as r, types};
use r::context::{CircuitContext, ConstructorContext, ConstructorResult};
use r::ledger::{ChargedState, DefaultDB, StateValue, constructor_cell};
use r::recording::{RecordedCircuitResult, RecordingFrame};
use r::{CompactError, Field, OpaqueBytes, OpaqueString};
pub fn envelope(populated: bool) -> types::Envelope {
    types::Envelope {
        entry: types::OptionalText {
            is_some: populated,
            value: if populated {
                OpaqueString::from("stored ✓")
            } else {
                OpaqueString::default()
            },
        },
        bytes: if populated {
            OpaqueBytes::from(vec![0, 1, 255])
        } else {
            OpaqueBytes::default()
        },
        marker: Field::from(if populated { 41_u64 } else { 0 }),
    }
}
pub fn context(populated: bool, malformed: bool) -> CircuitContext<u64> {
    let text = if malformed {
        // Same official opaque FAB representation, deliberately invalid UTF-8.
        constructor_cell::<_, DefaultDB>(OpaqueBytes::from(vec![0xff]))
    } else {
        constructor_cell::<_, DefaultDB>(envelope(populated).entry.value)
    };
    let mut rest = vec![
        constructor_cell::<_, DefaultDB>(envelope(populated).bytes),
        constructor_cell::<_, DefaultDB>(envelope(populated)),
        constructor_cell::<_, DefaultDB>(Field::from(0_u64)),
    ];
    rest.extend((0..12).map(|i| {
        constructor_cell::<_, DefaultDB>(Field::from(if populated { 100_u64 + i } else { 0 }))
    }));
    let fields = vec![
        StateValue::Array(vec![text].into()),
        StateValue::Array(rest.into()),
    ];
    ConstructorResult::new(
        ConstructorContext::new(7),
        ChargedState::new(StateValue::Array(fields.into())),
    )
    .into_circuit_context(Default::default())
}
pub fn reference(
    ctx: CircuitContext<u64>,
    selected: bool,
) -> Result<RecordedCircuitResult<u64, types::Receipt>, CompactError> {
    let (frame, prior) = ledger_slots::envelope.record_read(RecordingFrame::new(ctx))?;
    let (frame, _saved) = ledger_slots::text.record_read(frame)?;
    let (frame, _body) = ledger_slots::blob.record_read(frame)?;
    let chosen = if selected {
        prior
    } else {
        types::Envelope {
            entry: types::OptionalText {
                is_some: true,
                value: OpaqueString::default(),
            },
            bytes: OpaqueBytes::default(),
            marker: Field::from(9_u64),
        }
    };
    let frame = ledger_slots::text.record_write(frame, chosen.entry.value.clone())?;
    let frame = ledger_slots::blob.record_write(frame, chosen.bytes.clone())?;
    let frame = ledger_slots::envelope.record_write(frame, chosen)?;
    let (frame, readback) = ledger_slots::envelope.record_read(frame)?;
    let (frame, _readtext) = ledger_slots::text.record_read(frame)?;
    let (frame, _readbytes) = ledger_slots::blob.record_read(frame)?;
    let frame = ledger_slots::outcome.record_write(frame, readback.marker)?;
    let (frame, marker) = ledger_slots::outcome.record_read(frame)?;
    Ok(frame.finish(types::Receipt {
        matched: true,
        marker,
    }))
}
