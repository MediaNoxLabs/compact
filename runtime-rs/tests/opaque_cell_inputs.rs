// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Raw opaque validation precedes consuming FAB conversions and composes through derives.
use midnight_compact_runtime::fab::{Aligned, Alignment, Value, ValueSlice};
use midnight_compact_runtime::ledger::{self, CellValue, DefaultDB};
use midnight_compact_runtime::{
    CompactCellValue, CompactError, FixedBytes, FixedVector, OpaqueBytes, OpaqueString,
};

#[derive(Clone, Debug, PartialEq, Eq, CompactCellValue)]
struct Payload {
    text: OpaqueString,
    bytes: OpaqueBytes,
}
#[derive(Clone, Debug, PartialEq, Eq, CompactCellValue)]
struct Nested {
    payloads: FixedVector<(Payload, OpaqueBytes), 2>,
}
#[derive(Clone, Debug, PartialEq, Eq, CompactCellValue)]
struct Empty {}
#[derive(Clone, Copy, Debug, PartialEq, Eq, CompactCellValue)]
enum Choice {
    First,
    Second,
}
fn payload(text: &str, bytes: &[u8]) -> Payload {
    Payload {
        text: text.into(),
        bytes: bytes.to_vec().into(),
    }
}
fn rejected<T: CellValue>(value: T) {
    let result = ledger::try_constructor_cell::<_, DefaultDB>(value);
    assert!(
        matches!(result, Err(CompactError::InvalidLedgerCell(ref message))
        if message == "opaque ledger input has trailing zero bytes")
    );
}
fn round_trip<T: CellValue + Clone + std::fmt::Debug + PartialEq>(value: T) {
    let state = ledger::try_constructor_cell::<_, DefaultDB>(value.clone()).unwrap();
    assert_eq!(ledger::read_cell::<T, DefaultDB>(&state).unwrap(), value);
}

#[test]
fn opaque_raw_matrix_rejects_only_terminal_zero_and_retains_canonical_inputs() {
    for bytes in [vec![], vec![1], vec![0, 1], vec![1, 0, 2], vec![255]] {
        round_trip(OpaqueBytes::from(bytes));
    }
    for bytes in [vec![0], vec![1, 0], vec![1, 0, 0], vec![0, 1, 255, 0]] {
        rejected(OpaqueBytes::from(bytes));
    }
    for text in ["", "hello", "\0hello", "hello\0world", "é\0文"] {
        round_trip(OpaqueString::from(text));
    }
    for text in ["\0", "hello\0", "hello\0\0", "é文\0"] {
        rejected(OpaqueString::from(text));
    }
}

#[test]
fn recursive_struct_vector_and_tuple_members_cannot_hide_noncanonical_values() {
    let good = payload("text", &[0, 2]);
    round_trip(good.clone());
    rejected(payload("text\0", &[2]));
    rejected(payload("text", &[2, 0]));
    let nested = |second| Nested {
        payloads: FixedVector::new([
            (good.clone(), OpaqueBytes::from(vec![1])),
            (second, OpaqueBytes::from(vec![2])),
        ]),
    };
    round_trip(nested(good.clone()));
    rejected(nested(payload("bad\0", &[3])));
    rejected((good.clone(), OpaqueBytes::from(vec![1, 0])));
    rejected(FixedVector::new([good, payload("fine", &[0])]));
}

#[test]
fn fixed_width_and_empty_products_keep_their_existing_encoding() {
    round_trip(FixedBytes::new([1, 0, 0]));
    round_trip([1_u8, 0, 0]);
    round_trip(FixedBytes::new([]));
    round_trip(());
    round_trip(Empty {});
    round_trip(FixedVector::<OpaqueBytes, 0>::new([]));
    round_trip(Choice::First);
    round_trip(Choice::Second);
}

#[test]
fn legacy_constructor_and_into_value_remain_explicitly_normalizing() {
    let state = ledger::constructor_cell::<_, DefaultDB>(OpaqueBytes::from(vec![1, 0]));
    assert_eq!(
        ledger::read_cell::<OpaqueBytes, DefaultDB>(&state).unwrap(),
        OpaqueBytes::from(vec![1])
    );
    let value: Value = OpaqueBytes::from(vec![1, 0]).into();
    assert_eq!(value.0[0].0, vec![1]);
    rejected(OpaqueBytes::from(vec![1, 0]));
}

struct Reject(&'static str);
impl Aligned for Reject {
    fn alignment() -> Alignment {
        <() as Aligned>::alignment()
    }
}
impl From<Reject> for Value {
    fn from(_: Reject) -> Self {
        panic!("validation must run before consuming conversion")
    }
}
impl CellValue for Reject {
    fn decode_cell_value(_: &ValueSlice) -> Result<Self, CompactError> {
        Err(CompactError::InvalidLedgerCell("test-only decode".into()))
    }
    fn validate_cell_value(&self) -> Result<(), CompactError> {
        Err(CompactError::InvalidLedgerCell(self.0.into()))
    }
}
#[derive(CompactCellValue)]
struct Ordered {
    first: Reject,
    second: Reject,
}

#[test]
fn borrowing_validation_preserves_first_member_error_before_any_conversion() {
    for error in [
        ledger::try_constructor_cell::<_, DefaultDB>(Reject("direct")).unwrap_err(),
        ledger::try_constructor_cell::<_, DefaultDB>(Ordered {
            first: Reject("direct"),
            second: Reject("later"),
        })
        .unwrap_err(),
        ledger::try_constructor_cell::<_, DefaultDB>((Reject("direct"), Reject("later")))
            .unwrap_err(),
        ledger::try_constructor_cell::<_, DefaultDB>(FixedVector::new([
            Reject("direct"),
            Reject("later"),
        ]))
        .unwrap_err(),
    ] {
        assert!(
            matches!(error, CompactError::InvalidLedgerCell(ref message) if message == "direct")
        );
    }
}

struct Misaligned;
impl Aligned for Misaligned {
    fn alignment() -> Alignment {
        <() as Aligned>::alignment()
    }
}
impl From<Misaligned> for Value {
    fn from(_: Misaligned) -> Self {
        true.into()
    }
}
impl CellValue for Misaligned {
    fn decode_cell_value(_: &ValueSlice) -> Result<Self, CompactError> {
        Ok(Self)
    }
}
#[test]
fn safe_constructor_returns_alignment_error_instead_of_panicking() {
    let result = ledger::try_constructor_cell::<_, DefaultDB>(Misaligned);
    assert!(
        matches!(result, Err(CompactError::InvalidLedgerCell(ref message))
        if message == "CellValue does not match its declared alignment")
    );
}

#[path = "opaque_cell_inputs/compound_slots.rs"]
mod compound_slots;
#[path = "opaque_cell_inputs/zero_width_slots.rs"]
mod zero_width_slots;
