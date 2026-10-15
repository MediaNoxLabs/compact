// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! A default leaf is constructed once and checked before conversion or VM access.
use midnight_compact_runtime::context::{CircuitContext, ConstructorContext, ConstructorResult};
use midnight_compact_runtime::fab::{Aligned, Alignment, Value, ValueSlice};
use midnight_compact_runtime::ledger::{CellValue, ChargedState, StateValue};
use midnight_compact_runtime::recording::RecordingFrame;
use midnight_compact_runtime::{BoundedUint, CompactError, OpaqueBytes};
use std::cell::Cell;

thread_local! {
    static DEFAULTS: Cell<usize> = const { Cell::new(0) };
    static ENCODINGS: Cell<usize> = const { Cell::new(0) };
}
struct InvalidDefault(OpaqueBytes);
impl Default for InvalidDefault {
    fn default() -> Self {
        DEFAULTS.set(DEFAULTS.get() + 1);
        Self(OpaqueBytes(vec![7, 0]))
    }
}
impl Aligned for InvalidDefault {
    fn alignment() -> Alignment {
        OpaqueBytes::alignment()
    }
}
impl From<InvalidDefault> for Value {
    fn from(value: InvalidDefault) -> Self {
        ENCODINGS.set(ENCODINGS.get() + 1);
        value.0.into()
    }
}
impl CellValue for InvalidDefault {
    fn validate_cell_value(&self) -> Result<(), CompactError> {
        self.0.validate_cell_value()
    }
    fn decode_cell_value(value: &ValueSlice) -> Result<Self, CompactError> {
        OpaqueBytes::decode_cell_value(value).map(Self)
    }
}
fn absent() -> CircuitContext<()> {
    // Any actual tree query here would fail; raw validation must fail first.
    ConstructorResult::new(
        ConstructorContext::new(()),
        ChargedState::new(StateValue::Null),
    )
    .into_circuit_context(Default::default())
}
#[test]
fn invalid_default_leaf_is_created_once_and_never_encoded_in_native_or_recorded_trees() {
    for operation in ["native", "recorded", "native_historic", "recorded_historic"] {
        DEFAULTS.set(0);
        ENCODINGS.set(0);
        let position = BoundedUint::new(0).unwrap();
        let error = match operation {
            "native" => absent()
                .merkle_insert_index_default::<InvalidDefault>(&[0], position)
                .err(),
            "recorded" => RecordingFrame::new(absent())
                .insert_merkle_index_default::<InvalidDefault>(&[0], position)
                .err(),
            "native_historic" => absent()
                .historic_insert_index_default::<InvalidDefault>(&[0], position)
                .err(),
            "recorded_historic" => RecordingFrame::new(absent())
                .insert_historic_merkle_index_default::<InvalidDefault>(&[0], position)
                .err(),
            _ => unreachable!(),
        };
        assert_eq!(
            error,
            Some(CompactError::InvalidLedgerCell(
                "opaque ledger input has trailing zero bytes".into()
            )),
            "{operation}"
        );
        assert_eq!(DEFAULTS.get(), 1, "{operation}: default must run once");
        assert_eq!(ENCODINGS.get(), 0, "{operation}: no normalizing conversion");
    }
}
