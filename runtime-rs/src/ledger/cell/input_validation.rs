// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Low-level error adaptation happens before a write program can be executed.
use super::*;
use crate::OpaqueBytes;
use crate::ledger::DefaultDB;
use midnight_onchain_vm::error::OnchainProgramError;

#[test]
fn every_cell_write_path_rejects_raw_opaque_input_before_building_a_program() {
    for path in [vec![], vec![0], vec![1, 0], vec![1, 2, 3]] {
        let error =
            cell_write_program::<_, DefaultDB>(&path, OpaqueBytes::from(vec![1, 0])).unwrap_err();
        assert!(
            matches!(error, TranscriptRejected::Execution(OnchainProgramError::InvalidArgs(ref message))
            if message == "invalid Compact ledger cell: opaque ledger input has trailing zero bytes")
        );
    }
}
#[test]
fn cell_write_retains_path_validation_precedence() {
    let error =
        cell_write_program::<_, DefaultDB>(&[0; 17], OpaqueBytes::from(vec![1, 0])).unwrap_err();
    assert!(matches!(
        error,
        TranscriptRejected::Execution(OnchainProgramError::BoundsExceeded)
    ));
}
#[test]
fn canonical_cell_write_keeps_existing_official_state_encoding() {
    for path in [vec![0], vec![1, 0], vec![1, 2, 3]] {
        let value = OpaqueBytes::from(vec![0, 1]);
        let expected = constructor_cell::<_, DefaultDB>(value.clone());
        let program = cell_write_program::<_, DefaultDB>(&path, value).unwrap();
        let stored: Vec<_> = program
            .iter()
            .filter_map(|op| match op {
                Op::Push {
                    storage: true,
                    value,
                } => Some(value),
                _ => None,
            })
            .collect();
        assert_eq!(stored.len(), 1);
        let stored: StateValue<DefaultDB> = (*stored[0]).clone().into();
        assert_eq!(stored, expected);
    }
}
