// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Raw keys must be checked before upstream FAB normalization can alias them.
use super::*;
use crate::ledger::DefaultDB;
use crate::{OpaqueBytes, OpaqueString};
use midnight_base_crypto::fab::{Aligned, Alignment, Value, ValueSlice};
use midnight_onchain_vm::error::OnchainProgramError;

fn invalid() -> CompactError {
    CompactError::InvalidLedgerCell("opaque ledger input has trailing zero bytes".into())
}
fn rejected(error: TranscriptRejected<DefaultDB>) {
    let TranscriptRejected::Execution(OnchainProgramError::InvalidArgs(detail)) = error else {
        panic!("expected invalid raw input: {error:?}")
    };
    assert!(detail.contains("opaque ledger input has trailing zero bytes"));
}

fn collection_inputs<T: CellValue + Clone>(canonical: T, bad: T) {
    assert!(set_insert_program::<_, DefaultDB>(&[0], canonical.clone()).is_ok());
    assert!(set_remove_program::<_, DefaultDB>(&[0], canonical.clone()).is_ok());
    assert!(
        set_member_program::<_, ResultModeGather, DefaultDB>(&[0], canonical.clone(), ()).is_ok()
    );
    assert!(
        map_insert_program::<_, _, DefaultDB>(&[0], canonical.clone(), canonical.clone()).is_ok()
    );
    assert!(
        map_lookup_program::<_, ResultModeGather, DefaultDB>(&[0], canonical.clone(), ()).is_ok()
    );
    assert!(list_push_front_program::<_, DefaultDB>(&[0][..], canonical.clone()).is_ok());
    rejected(set_insert_program::<_, DefaultDB>(&[0], bad.clone()).unwrap_err());
    rejected(set_remove_program::<_, DefaultDB>(&[0], bad.clone()).unwrap_err());
    rejected(
        map_insert_program::<_, _, DefaultDB>(&[0], bad.clone(), canonical.clone()).unwrap_err(),
    );
    rejected(map_insert_program::<_, _, DefaultDB>(&[0], canonical, bad.clone()).unwrap_err());
    rejected(list_push_front_program::<_, DefaultDB>(&[0][..], bad.clone()).unwrap_err());
    assert_eq!(
        set_member_program::<_, ResultModeGather, DefaultDB>(&[0], bad.clone(), ()).unwrap_err(),
        invalid()
    );
    assert_eq!(
        map_lookup_program::<_, ResultModeGather, DefaultDB>(&[0], bad, ()).unwrap_err(),
        invalid()
    );
}
#[test]
fn collection_programs_reject_keys_and_values_before_normalization() {
    collection_inputs(OpaqueBytes(vec![1, 0, 2]), OpaqueBytes(vec![1, 0, 2, 0]));
    collection_inputs(OpaqueString("a\0b".into()), OpaqueString("a\0b\0".into()));
}

fn held_views<T: CellValue + Clone>(canonical: T, alias: T) {
    let map = LedgerHashMap::<AlignedValue, StateValue<DefaultDB>, DefaultDB>::new().insert(
        aligned_cell_value(canonical.clone()),
        constructor_cell(true),
    );
    let state = StateValue::Array(vec![StateValue::Map(map)].into());
    let set = set_view_at_path::<T, DefaultDB>(&state, &[0]).unwrap();
    assert!(set.member(alias.clone())); // Deliberately retained compatibility API.
    assert!(set.try_member(canonical.clone()).unwrap());
    assert_eq!(set.try_member(alias.clone()).unwrap_err(), invalid());
    let map = map_view_at_path::<T, bool, DefaultDB>(&state, &[0]).unwrap();
    assert!(map.member(alias.clone())); // No signature change or hidden false result.
    assert!(map.try_member(canonical.clone()).unwrap());
    assert!(map.lookup(canonical).unwrap());
    assert_eq!(map.try_member(alias.clone()).unwrap_err(), invalid());
    assert_eq!(map.lookup(alias).unwrap_err(), invalid());
    assert_eq!(map.size().unwrap().value(), 1);
}
#[test]
fn checked_held_views_do_not_alias_existing_canonical_keys() {
    held_views(OpaqueBytes(vec![1]), OpaqueBytes(vec![1, 0]));
    held_views(OpaqueString("key".into()), OpaqueString("key\0".into()));
}

struct BadDefault(OpaqueBytes);
impl Default for BadDefault {
    fn default() -> Self {
        Self(OpaqueBytes(vec![1, 0]))
    }
}
impl Aligned for BadDefault {
    fn alignment() -> Alignment {
        OpaqueBytes::alignment()
    }
}
impl From<BadDefault> for Value {
    fn from(value: BadDefault) -> Self {
        value.0.into()
    }
}
impl CellValue for BadDefault {
    fn validate_cell_value(&self) -> Result<(), CompactError> {
        self.0.validate_cell_value()
    }
    fn decode_cell_value(value: &ValueSlice) -> Result<Self, CompactError> {
        OpaqueBytes::decode_cell_value(value).map(Self)
    }
}
#[test]
fn list_head_checks_raw_default_before_building_absent_value() {
    assert!(list_head_program::<OpaqueBytes, ResultModeGather, DefaultDB>(&[0][..], ()).is_ok());
    assert_eq!(
        list_head_program::<BadDefault, ResultModeGather, DefaultDB>(&[0][..], ()).unwrap_err(),
        invalid()
    );
}
#[test]
fn collection_path_validation_still_precedes_input_validation() {
    let bad = OpaqueBytes(vec![1, 0]);
    let error = set_insert_program::<_, DefaultDB>(&[0; 16], bad.clone()).unwrap_err();
    assert!(matches!(
        error,
        TranscriptRejected::Execution(OnchainProgramError::BoundsExceeded)
    ));
    let error =
        map_lookup_program::<_, ResultModeGather, DefaultDB>(&[0; 17], bad, ()).unwrap_err();
    assert!(
        matches!(error, CompactError::LedgerQueryRejected(detail) if detail.contains("BoundsExceeded"))
    );
}
