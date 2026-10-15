// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Typed leaves are checked before hashing; already-hashed fixed bytes are unchanged.
use super::*;
use crate::ledger::DefaultDB;
use crate::{OpaqueBytes, OpaqueString};
use midnight_onchain_vm::error::OnchainProgramError;

fn invalid() -> CompactError {
    CompactError::InvalidLedgerCell("opaque ledger input has trailing zero bytes".into())
}
fn typed_inputs<T: CellValue + Clone>(canonical: T, bad: T) {
    assert!(leaf_hash_for(canonical.clone()).is_ok());
    assert_eq!(leaf_hash_for(bad.clone()).unwrap_err(), invalid());
    assert!(merkle_insert_index_program::<_, DefaultDB>(&[0][..], canonical.clone(), 0).is_ok());
    assert!(
        historic_merkle_insert_index_program::<_, DefaultDB>(&[0][..], canonical.clone(), 0)
            .is_ok()
    );
    assert!(merkle_insert_program::<_, DefaultDB>(&[0][..], canonical.clone()).is_ok());
    assert!(historic_merkle_insert_program::<_, DefaultDB>(&[0][..], canonical).is_ok());
    for error in [
        merkle_insert_index_program::<_, DefaultDB>(&[0][..], bad.clone(), 0).unwrap_err(),
        historic_merkle_insert_index_program::<_, DefaultDB>(&[0][..], bad.clone(), 0).unwrap_err(),
        merkle_insert_program::<_, DefaultDB>(&[0][..], bad.clone()).unwrap_err(),
        historic_merkle_insert_program::<_, DefaultDB>(&[0][..], bad.clone()).unwrap_err(),
    ] {
        assert!(
            matches!(error, TranscriptRejected::Execution(OnchainProgramError::InvalidArgs(detail))
            if detail.contains("opaque ledger input has trailing zero bytes"))
        );
    }
    for history in [MerkleHistory::CurrentOnly, MerkleHistory::Historic] {
        assert_eq!(
            check_root_program::<_, ResultModeGather, DefaultDB>(
                (&[0]).into(),
                bad.clone(),
                history,
                ()
            )
            .unwrap_err(),
            invalid()
        );
    }
}
#[test]
fn typed_merkle_programs_and_generic_root_queries_reject_raw_noncanonical_inputs() {
    typed_inputs(OpaqueBytes(vec![1, 0, 2]), OpaqueBytes(vec![1, 0, 2, 0]));
    typed_inputs(OpaqueString("a\0b".into()), OpaqueString("a\0b\0".into()));
}
#[test]
fn fixed_hash_programs_keep_trailing_zero_bytes_valid() {
    let hash = FixedBytes::<32>::new([0; 32]);
    assert!(merkle_insert_hash_index_program::<DefaultDB>(&[0][..], hash, 0).is_ok());
    assert!(historic_merkle_insert_hash_index_program::<DefaultDB>(&[0][..], hash, 0).is_ok());
    assert!(merkle_insert_hash_program::<DefaultDB>(&[0][..], hash).is_ok());
    assert!(historic_merkle_insert_hash_program::<DefaultDB>(&[0][..], hash).is_ok());
    assert!(
        check_root_program::<_, ResultModeGather, DefaultDB>(
            (&[0]).into(),
            hash,
            MerkleHistory::CurrentOnly,
            ()
        )
        .is_ok()
    );
}
