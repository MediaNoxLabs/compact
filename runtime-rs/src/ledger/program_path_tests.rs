// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//  	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Program boundary tests use the pinned ledger-8 opcode layout as oracle.
use super::*;
use midnight_onchain_vm::error::OnchainProgramError;
use midnight_onchain_vm::result_mode::ResultModeVerify;
use midnight_transient_crypto::repr::FieldRepr;

type Program = Vec<Op<ResultModeVerify, DefaultDB>>;
type MutationBuilder = fn(&[u8]) -> Result<Program, TranscriptRejected<DefaultDB>>;
type ReadBuilder = fn(&[u8]) -> Result<Program, CompactError>;

#[test]
#[expect(
    clippy::result_large_err,
    reason = "Test builders preserve upstream errors for exact variant assertions"
)]
fn mutation_operands_preserve_exact_encoding_boundaries() {
    let cases: &[(&str, usize, MutationBuilder)] = &[
        ("plain append", 14, |p| merkle_insert_program(p, true)),
        ("plain hash append", 14, |p| {
            merkle_insert_hash_program(p, FixedBytes([0; 32]))
        }),
        ("historic append", 14, |p| {
            historic_merkle_insert_program(p, true)
        }),
        ("historic hash append", 14, |p| {
            historic_merkle_insert_hash_program(p, FixedBytes([0; 32]))
        }),
        ("plain indexed", 15, |p| {
            merkle_insert_index_program(p, true, 0)
        }),
        ("plain hash indexed", 15, |p| {
            merkle_insert_hash_index_program(p, FixedBytes([0; 32]), 0)
        }),
        ("plain default indexed", 15, |p| {
            merkle_insert_index_default_program::<bool, _>(p, 0)
        }),
        ("historic indexed", 14, |p| {
            historic_merkle_insert_index_program(p, true, 0)
        }),
        ("historic hash indexed", 14, |p| {
            historic_merkle_insert_hash_index_program(p, FixedBytes([0; 32]), 0)
        }),
        ("historic default indexed", 14, |p| {
            historic_merkle_insert_index_default_program::<bool, _>(p, 0)
        }),
        ("reset history", 13, |p| {
            historic_reset_history_program(p.into())
        }),
        ("plain reset", 16, |p| merkle_reset_program(p, 3)),
        ("historic reset", 16, |p| {
            historic_reset_to_default_program(p, 3)
        }),
        ("list push", 14, |p| list_push_front_program(p, true)),
        ("list pop", 15, |p| list_pop_front_program(p)),
        ("list reset", 16, |p| list_reset_program(p)),
        ("set insert", 15, |p| set_insert_program(p, true)),
        ("set remove", 15, |p| set_remove_program(p, true)),
        ("set reset", 16, |p| set_reset_program(p)),
        ("map insert", 15, |p| map_insert_program(p, true, false)),
        ("counter add", 15, |p| counter_program(p, 1, false)),
        ("counter subtract", 15, |p| counter_program(p, 1, true)),
    ];
    for &(name, maximum, build) in cases {
        let program = build(&vec![0; maximum]).unwrap_or_else(|e| panic!("{name}: {e:?}"));
        // Each maximum reaches n=15 in the final write-back operation.
        let last = program.last().unwrap();
        assert!(
            matches!(
                last,
                Op::Ins {
                    cached: true,
                    n: 15
                }
            ),
            "{name}: {last:?}"
        );
        assert_eq!(last.field_vec(), vec![Field::from(0xaf_u64)], "{name}");
        for length in [maximum + 1, 255, 256, 257] {
            assert!(
                matches!(
                    build(&vec![0; length]),
                    Err(TranscriptRejected::Execution(
                        OnchainProgramError::BoundsExceeded
                    ))
                ),
                "{name}: length {length}"
            );
        }
    }
}

#[test]
fn read_and_cell_programs_preserve_sixteen_key_index_boundary() {
    let cases: &[(&str, ReadBuilder)] = &[
        ("cell", |p| cell_read_program(p, AlignedValue::from(false))),
        ("counter", |p| {
            counter_read_program(p, AlignedValue::from(false))
        }),
        ("counter comparison", |p| {
            counter_less_than_program(p, 1, AlignedValue::from(false))
        }),
        ("list length", |p| {
            list_length_program(p, AlignedValue::from(false))
        }),
        ("list empty", |p| {
            list_is_empty_program(p, AlignedValue::from(false))
        }),
        ("list head", |p| {
            list_head_program::<bool, _, _>(p, AlignedValue::from(false))
        }),
        ("map lookup", |p| {
            map_lookup_program(p, true, AlignedValue::from(false))
        }),
        ("set member", |p| {
            set_member_program(p, true, AlignedValue::from(false))
        }),
        ("set size", |p| {
            set_size_program(p, AlignedValue::from(false))
        }),
        ("set empty", |p| {
            set_is_empty_program(p, AlignedValue::from(false))
        }),
        ("merkle full", |p| {
            is_full_program(p.into(), 3, AlignedValue::from(false))
        }),
        ("merkle root", |p| {
            merkle_check_root_verify_program(p.into(), 0_u8, AlignedValue::from(false))
        }),
        ("historic root", |p| {
            historic_check_root_verify_program(p.into(), 0_u8, AlignedValue::from(false))
        }),
    ];
    for &(name, build) in cases {
        let program = build(&[0; 16]).unwrap();
        let index = &program[1];
        assert!(
            matches!(index, Op::Idx { path, push_path: false, .. } if path.len() == 16),
            "{name}"
        );
        assert_eq!(index.field_vec()[0], Field::from(0x5f_u64), "{name}");
        for length in [17, 255, 256] {
            assert!(
                matches!(build(&vec![0; length]), Err(CompactError::LedgerQueryRejected(message))
                if message == "Execution(BoundsExceeded)"),
                "{name}: length {length}"
            );
        }
        // Root selection remains supported by the internal VM builders.
        assert!(build(&[]).is_ok(), "{name}: empty root");
    }
    let program = cell_write_program::<_, DefaultDB>(&[0; 16], false).unwrap();
    assert_eq!(program[0].field_vec()[0], Field::from(0x7f_u64));
    for length in [17, 255, 256] {
        assert!(matches!(
            cell_write_program::<_, DefaultDB>(&vec![0; length], false),
            Err(TranscriptRejected::Execution(
                OnchainProgramError::BoundsExceeded
            ))
        ));
    }
}

#[test]
#[expect(
    clippy::result_large_err,
    reason = "Test builders preserve upstream errors for exact variant assertions"
)]
fn empty_paths_distinguish_field_replacement_from_root_selection() {
    for build in [
        (|p: &[u8]| merkle_reset_program::<DefaultDB>(p, 3)) as MutationBuilder,
        |p| historic_reset_to_default_program(p, 3),
        |p| list_reset_program(p),
    ] {
        assert!(
            matches!(build(&[]), Err(TranscriptRejected::Execution(OnchainProgramError::InvalidArgs(message)))
            if message == "ledger path contains no field index")
        );
    }
    assert!(merkle_insert_program::<_, DefaultDB>(&[][..], true).is_ok());
    assert!(historic_merkle_insert_program::<_, DefaultDB>(&[][..], true).is_ok());
    assert!(historic_reset_history_program::<DefaultDB>((&[][..]).into()).is_ok());
    assert!(set_reset_program::<DefaultDB>(&[]).is_ok());
    assert!(set_insert_program::<_, DefaultDB>(&[], true).is_ok());
    assert!(list_push_front_program::<_, DefaultDB>(&[][..], true).is_ok());
    assert!(counter_program::<DefaultDB>(&[], 1, false).is_ok());
}

#[test]
fn qualified_coin_set_checks_operand_before_commitment_lookup() {
    type QualifiedCell = (FixedBytes<32>, FixedBytes<32>, u128, u64);
    let mut query = empty_query_context();
    let coin = coin_info_from_compact(FixedBytes([1; 32]), FixedBytes([2; 32]), 3);
    let recipient = coin_recipient_from_compact(true, FixedBytes([3; 32]), FixedBytes([0; 32]));
    for length in [7, 16, 255, 256] {
        assert!(
            matches!(qualified_coin_set_insert_program::<QualifiedCell, DefaultDB>(
            &query, &vec![0; length], coin, recipient.clone()),
            Err(CompactError::LedgerQueryRejected(message)) if message == "Execution(BoundsExceeded)")
        );
    }
    let commitment = coin.commitment(&recipient);
    query.call_context.com_indices = query.call_context.com_indices.insert(commitment, 0);
    let program = qualified_coin_set_insert_program::<QualifiedCell, DefaultDB>(
        &query, &[0; 6], coin, recipient,
    )
    .unwrap();
    assert!(matches!(program[1], Op::Dup { n: 14 }));
    assert_eq!(program[1].field_vec(), vec![Field::from(0x3e_u64)]);
    assert!(matches!(
        program.last(),
        Some(Op::Ins { cached: true, n: 6 })
    ));
    assert_eq!(
        program.last().unwrap().field_vec(),
        vec![Field::from(0xa6_u64)]
    );
}
