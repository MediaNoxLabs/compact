// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//  http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Canonical immediate values at the official ledger8 VM boundary.
//!
//! Stored contract state remains the official unchecked representation. A VM
//! program's constants must use its checked representation, preventing Field
//! encodings from silently reducing modulo the scalar field.

use super::{CellValue, CompactError, StateValue};
use midnight_base_crypto::fab::AlignedValue;
use midnight_onchain_state::state::AlignedValueChecked;
use midnight_storage::arena::Sp;
use midnight_storage::db::DB;

pub(super) type Immediate<D> = StateValue<D, AlignedValueChecked>;

pub(super) fn key(value: AlignedValue) -> Result<AlignedValueChecked, CompactError> {
    AlignedValueChecked::try_from(value).map_err(|()| {
        CompactError::InvalidLedgerCell("VM value contains a noncanonical Field encoding".into())
    })
}

pub(super) fn aligned<D: DB>(value: AlignedValue) -> Result<Immediate<D>, CompactError> {
    key(value).map(|value| StateValue::Cell(Sp::new(value)))
}

pub(super) fn cell<T: CellValue, D: DB>(value: T) -> Result<Immediate<D>, CompactError> {
    aligned(super::try_aligned_cell_value(value)?)
}

/// Only fixed-width native scalar constants belong here. Caller-defined
/// CellValue implementations must go through the fallible `cell` boundary.
pub(super) fn literal<T: Into<AlignedValue>, D: DB>(value: T) -> Immediate<D> {
    StateValue::Cell(Sp::new(midnight_onchain_vm::ops::expect_checked(value)))
}

/// Convert a newly assembled, small reset constant, never existing contract
/// state. Upstream's recursive conversion can expand shared state trees.
pub(super) fn state<D: DB>(value: StateValue<D>) -> Result<Immediate<D>, CompactError> {
    value.try_into_checked().map_err(|()| {
        CompactError::InvalidLedgerCell("VM constant is not a canonical checked state".into())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Field;
    use crate::ledger::{self, ChargedState, ContractAddress, DefaultDB, QueryContext};
    use midnight_base_crypto::fab::{Aligned, Alignment, Value, ValueAtom, ValueSlice};

    struct NoncanonicalField;

    impl Aligned for NoncanonicalField {
        fn alignment() -> Alignment {
            Field::alignment()
        }
    }

    impl From<NoncanonicalField> for Value {
        fn from(_: NoncanonicalField) -> Self {
            Self(vec![ValueAtom(vec![0xff; 32])])
        }
    }

    impl CellValue for NoncanonicalField {
        fn decode_cell_value(_: &ValueSlice) -> Result<Self, CompactError> {
            Ok(Self)
        }
    }

    #[test]
    fn canonical_immediate_preserves_the_stored_value() {
        let checked = cell::<_, DefaultDB>(Field::from(17u64)).unwrap();
        let stored: StateValue<DefaultDB> = checked.into();
        assert_eq!(stored, ledger::constructor_cell(Field::from(17u64)));
    }

    #[test]
    fn alignment_alone_does_not_authorize_a_vm_field() {
        let aligned = ledger::try_aligned_cell_value(NoncanonicalField).unwrap();
        assert!(AlignedValue::new(aligned.value.clone(), aligned.alignment.clone()).is_some());
        assert!(matches!(
            key(aligned),
            Err(CompactError::InvalidLedgerCell(_))
        ));
        assert!(matches!(
            cell::<_, DefaultDB>(NoncanonicalField),
            Err(CompactError::InvalidLedgerCell(_))
        ));
    }

    #[test]
    fn invalid_manual_cell_is_rejected_before_a_ledger_write() {
        let query = QueryContext::new(
            ChargedState::new(StateValue::<DefaultDB>::Array(
                vec![ledger::constructor_cell(false)].into(),
            )),
            ContractAddress::default(),
        );
        let before = query.state.clone();
        let error = ledger::write_cell_at_path(
            &query,
            &[0],
            NoncanonicalField,
            None,
            &midnight_onchain_vm::cost_model::INITIAL_COST_MODEL,
        )
        .err()
        .expect("noncanonical immediate must not reach the VM");
        assert!(format!("{error:?}").contains("noncanonical Field"));
        assert_eq!(query.state, before);
    }

    #[test]
    fn reset_constants_are_checked_without_scanning_existing_state() {
        assert!(state::<DefaultDB>(ledger::constructor_list()).is_ok());
        assert!(state::<DefaultDB>(ledger::constructor_merkle_tree(4)).is_ok());
        let malformed = StateValue::Array(
            vec![ledger::constructor_cell::<_, DefaultDB>(NoncanonicalField)].into(),
        );
        assert!(matches!(
            state(malformed),
            Err(CompactError::InvalidLedgerCell(_))
        ));
    }
}
