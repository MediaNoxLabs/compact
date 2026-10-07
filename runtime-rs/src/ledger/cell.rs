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

//! Cell representation, reads and canonical native/recorded VM programs.

use super::{
    CellValue, CompactError, QueryContext, QueryResults, StateValue, TranscriptRejected,
    field_at_path,
};
use midnight_base_crypto::cost_model::RunningCost;
use midnight_base_crypto::fab::AlignedValue;
use midnight_onchain_vm::cost_model::CostModel;
use midnight_onchain_vm::ops::{Key, Op};
use midnight_onchain_vm::result_mode::{
    GatherEvent, ResultMode, ResultModeGather, ResultModeVerify,
};
use midnight_storage::db::DB;

pub fn constructor_cell<T, D>(value: T) -> StateValue<D>
where
    T: CellValue,
    D: DB,
{
    StateValue::from(aligned_cell_value(value))
}

pub(crate) fn aligned_cell_value<T: CellValue>(value: T) -> AlignedValue {
    AlignedValue::new(value.into(), T::alignment()).expect("CellValue must match its alignment")
}

/// Read a Compact Cell after checking the declared type's exact alignment.
pub fn read_cell<T, D>(state: &StateValue<D>) -> Result<T, CompactError>
where
    T: CellValue,
    D: DB,
{
    let StateValue::Cell(cell) = state else {
        return Err(CompactError::InvalidLedgerCell(
            "expected Cell state".into(),
        ));
    };
    if cell.alignment != T::alignment() {
        return Err(CompactError::InvalidLedgerCell(
            "alignment differs from declared type".into(),
        ));
    }
    T::decode_cell_value(&cell.as_slice())
}

/// Decode one declared root Cell from a contract's current ledger state.
pub fn read_root_cell<T, D>(state: &StateValue<D>, index: u8) -> Result<T, CompactError>
where
    T: CellValue,
    D: DB,
{
    read_cell_at_path(state, &[index])
}

/// Read a Cell at the compiler's public ledger path, including chunked roots.
pub fn read_cell_at_path<T, D>(state: &StateValue<D>, path: &[u8]) -> Result<T, CompactError>
where
    T: CellValue,
    D: DB,
{
    read_cell(field_at_path(state, path)?)
}

/// Read a root Cell through the ledger VM and gather its typed read event.
pub fn query_cell<T: CellValue, D: DB>(
    context: &QueryContext<D>,
    field_index: u8,
    gas_limit: Option<RunningCost>,
    cost_model: &CostModel,
) -> Result<(QueryResults<ResultModeGather, D>, T), CompactError> {
    query_cell_at_path(context, &[field_index], gas_limit, cost_model)
}

pub fn query_cell_at_path<T: CellValue, D: DB>(
    context: &QueryContext<D>,
    path: &[u8],
    gas_limit: Option<RunningCost>,
    cost_model: &CostModel,
) -> Result<(QueryResults<ResultModeGather, D>, T), CompactError> {
    if path.is_empty() {
        return Err(CompactError::InvalidLedgerCell("empty ledger path".into()));
    }
    let program = cell_read_program::<ResultModeGather, D>(path, ())?;
    let result = context
        .query(&program, gas_limit, cost_model)
        .map_err(|error| CompactError::LedgerQueryRejected(format!("{error:?}")))?;
    let decoded = decode_last_read::<T, D>(&result)?;
    Ok((result, decoded))
}

pub(crate) fn cell_read_program<M: ResultMode<D>, D: DB>(
    path: &[u8],
    read_result: M::ReadResult,
) -> Result<Vec<Op<M, D>>, CompactError> {
    super::checked_index_path::<D>(path).map_err(super::query_rejected)?;
    Ok(vec![
        Op::Dup { n: 0 },
        Op::Idx {
            cached: false,
            push_path: false,
            path: path
                .iter()
                .map(|index| Key::Value(AlignedValue::from(*index)))
                .collect::<Vec<_>>()
                .into(),
        },
        Op::Popeq {
            cached: false,
            result: read_result,
        },
    ])
}

pub(crate) fn decode_last_read<T: CellValue, D: DB>(
    result: &QueryResults<ResultModeGather, D>,
) -> Result<T, CompactError> {
    let Some(GatherEvent::Read(value)) = result.events.last() else {
        return Err(CompactError::InvalidLedgerCell(
            "missing ledger read event".into(),
        ));
    };
    if value.alignment != T::alignment() {
        return Err(CompactError::InvalidLedgerCell(
            "alignment differs from declared type".into(),
        ));
    }
    T::decode_cell_value(&value.value)
}

/// Compact Counter initializes as a ledger Cell of fixed-width Uint64.
/// Replace an existing root Cell through ledger VM execution.
pub fn write_cell<T: CellValue, D: DB>(
    context: &QueryContext<D>,
    field_index: u8,
    value: T,
    gas_limit: Option<RunningCost>,
    cost_model: &CostModel,
) -> Result<QueryResults<ResultModeVerify, D>, TranscriptRejected<D>> {
    write_cell_at_path(context, &[field_index], value, gas_limit, cost_model)
}

pub fn write_cell_at_path<T: CellValue, D: DB>(
    context: &QueryContext<D>,
    path: &[u8],
    value: T,
    gas_limit: Option<RunningCost>,
    cost_model: &CostModel,
) -> Result<QueryResults<ResultModeVerify, D>, TranscriptRejected<D>> {
    context.query(&cell_write_program(path, value)?, gas_limit, cost_model)
}

pub(crate) fn cell_write_program<T: CellValue, D: DB>(
    path: &[u8],
    value: T,
) -> Result<Vec<Op<ResultModeVerify, D>>, TranscriptRejected<D>> {
    super::checked_index_path::<D>(path)?;
    if path.len() == 1 {
        // Compact emits a root Cell replacement as an insertion keyed by a
        // temporary Cell containing the ledger index. Preserve that program
        // so the ledger transcript matches the circuit's ZKIR public inputs.
        return Ok(vec![
            Op::Push {
                storage: false,
                value: constructor_cell(path[0]),
            },
            Op::Push {
                storage: true,
                value: constructor_cell(value),
            },
            Op::Ins {
                cached: false,
                n: 1,
            },
        ]);
    }
    if path.len() == 2 {
        // Compact indexes the containing array, then inserts at its final
        // key and inserts the updated array back into the root.
        return Ok(vec![
            Op::Idx {
                cached: false,
                push_path: true,
                path: vec![Key::Value(AlignedValue::from(path[0]))].into(),
            },
            Op::Push {
                storage: false,
                value: constructor_cell(path[1]),
            },
            Op::Push {
                storage: true,
                value: constructor_cell(value),
            },
            Op::Ins {
                cached: false,
                n: 1,
            },
            Op::Ins { cached: true, n: 1 },
        ]);
    }
    Ok(vec![
        Op::Idx {
            cached: false,
            push_path: true,
            path: path
                .iter()
                .map(|index| Key::Value(AlignedValue::from(*index)))
                .collect::<Vec<_>>()
                .into(),
        },
        Op::Pop,
        Op::Push {
            storage: true,
            value: constructor_cell(value),
        },
        Op::Ins { cached: true, n: 1 },
    ])
}

/// Replace a qualified-coin Cell using its transaction-allocated commitment index.
pub fn write_qualified_coin_cell<T: CellValue, D: DB>(
    context: &QueryContext<D>,
    path: impl Into<super::LedgerPath>,
    coin: super::CoinInfo,
    recipient: super::CoinRecipient,
    gas_limit: Option<RunningCost>,
    cost_model: &CostModel,
) -> Result<QueryResults<ResultModeVerify, D>, CompactError> {
    let path = path.into();
    let program = qualified_coin_cell_write_program_for_context::<T, D>(
        context,
        path.as_slice(),
        coin,
        recipient,
    )?;
    context
        .query(&program, gas_limit, cost_model)
        .map_err(|error| CompactError::LedgerQueryRejected(format!("{error:?}")))
}

pub(crate) fn qualified_coin_cell_write_program_for_context<T: CellValue, D: DB>(
    context: &QueryContext<D>,
    path: &[u8],
    coin: super::CoinInfo,
    recipient: super::CoinRecipient,
) -> Result<Vec<Op<ResultModeVerify, D>>, CompactError> {
    let commitment = super::qualified_coin_commitment::<T, D>(context, &coin, &recipient)?;
    qualified_coin_cell_write_program(path, coin, commitment)
}

fn qualified_coin_cell_write_program<D: DB>(
    path: &[u8],
    coin: super::CoinInfo,
    commitment: midnight_coin_structure::coin::Commitment,
) -> Result<Vec<Op<ResultModeVerify, D>>, CompactError> {
    let Some((&last, parents)) = path.split_last() else {
        return Err(CompactError::InvalidLedgerCell(
            "empty qualified coin Cell path".into(),
        ));
    };
    // The VM encodes dup depth in four bits. Each indexed parent adds two
    // stack entries; suppressing a root idx matches the Compact ADT program.
    let depth = 3 + 2 * parents.len();
    if depth > 15 {
        return Err(CompactError::InvalidLedgerCell(
            "qualified coin Cell path exceeds VM dup depth".into(),
        ));
    }
    let mut program = Vec::new();
    if !parents.is_empty() {
        program.push(Op::Idx {
            cached: false,
            push_path: true,
            path: super::path_keys(parents).into(),
        });
    }
    program.extend([
        Op::Push {
            storage: false,
            value: constructor_cell(last),
        },
        Op::Dup { n: depth as u8 },
        Op::Push {
            storage: false,
            value: StateValue::from(AlignedValue::from(commitment)),
        },
        Op::Idx {
            cached: true,
            push_path: false,
            path: vec![Key::Value(AlignedValue::from(1u8)), Key::Stack].into(),
        },
        Op::Push {
            storage: false,
            value: StateValue::from(AlignedValue::from(coin)),
        },
        Op::Swap { n: 0 },
        Op::Concat {
            cached: true,
            n: 91,
        },
        Op::Ins {
            cached: false,
            n: 1,
        },
    ]);
    if !parents.is_empty() {
        program.push(Op::Ins {
            cached: true,
            n: parents.len() as u8,
        });
    }
    Ok(program)
}

#[cfg(test)]
mod qualified_coin_tests {
    use super::*;
    use crate::{FixedBytes, ledger};

    #[test]
    fn native_qualified_cell_program_matches_independent_typescript() {
        let oracle: serde_json::Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/qualified-coin-cell-oracle.json"
        ))
        .unwrap();
        for (name, left, second) in [
            ("right7", false, false),
            ("left11", true, false),
            ("zero", false, false),
            ("replacement", true, true),
        ] {
            let mut nonce = [0; 32];
            nonce[..5].copy_from_slice(b"nonce");
            if second {
                nonce[5] = b'2';
            }
            let mut color = [0; 32];
            color[..5].copy_from_slice(b"color");
            let coin = ledger::coin_info_from_compact(
                FixedBytes::new(nonce),
                FixedBytes::new(color),
                if second { 43 } else { 42 },
            );
            let mut key = [0; 32];
            if left {
                key[0] = 7;
            }
            let recipient = ledger::coin_recipient_from_compact(
                left,
                FixedBytes::new(key),
                FixedBytes::new([0; 32]),
            );
            let commitment = coin.commitment(&recipient);
            let program =
                qualified_coin_cell_write_program::<ledger::DefaultDB>(&[0], coin, commitment)
                    .unwrap();
            assert_eq!(
                serde_json::to_value(program).unwrap(),
                oracle[name]["publicTranscript"]
            );
            let chunked: serde_json::Value = serde_json::from_str(include_str!(
                "../../tests/fixtures/qualified-coin-cell-chunked-oracle.json"
            ))
            .unwrap();
            let nested =
                qualified_coin_cell_write_program::<ledger::DefaultDB>(&[1, 14], coin, commitment)
                    .unwrap();
            assert_eq!(
                serde_json::to_value(nested).unwrap(),
                chunked[name]["publicTranscript"]
            );

            assert!(
                qualified_coin_cell_write_program::<ledger::DefaultDB>(&[], coin, commitment)
                    .is_err()
            );
            assert!(
                qualified_coin_cell_write_program::<ledger::DefaultDB>(&[0; 8], coin, commitment)
                    .is_err()
            );
        }
    }
}
