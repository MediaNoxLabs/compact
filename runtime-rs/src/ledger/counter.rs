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

//! Counter reads and the shared increment/decrement VM program.

use super::{
    CompactError, LedgerPath, QueryContext, QueryResults, StateValue, TranscriptRejected,
    constructor_cell, path_keys, read_cell,
};
use midnight_base_crypto::cost_model::RunningCost;
use midnight_onchain_vm::cost_model::CostModel;
use midnight_onchain_vm::ops::Op;
use midnight_onchain_vm::result_mode::ResultModeVerify;
use midnight_storage::db::DB;

pub fn constructor_counter<D: DB>() -> StateValue<D> {
    constructor_cell::<u64, D>(0)
}

pub fn read_counter<D: DB>(state: &StateValue<D>) -> Result<u64, CompactError> {
    read_cell::<u64, D>(state)
}

/// Run Compact Counter's increment program through ledger query execution.
pub fn increment_counter<D: DB>(
    context: &QueryContext<D>,
    path: impl Into<LedgerPath>,
    amount: u16,
    gas_limit: Option<RunningCost>,
    cost_model: &CostModel,
) -> Result<QueryResults<ResultModeVerify, D>, TranscriptRejected<D>> {
    let path = path.into();
    update_counter(
        context,
        path.as_slice(),
        amount,
        false,
        gas_limit,
        cost_model,
    )
}

/// Run Compact Counter's decrement program through ledger query execution.
pub fn decrement_counter<D: DB>(
    context: &QueryContext<D>,
    path: impl Into<LedgerPath>,
    amount: u16,
    gas_limit: Option<RunningCost>,
    cost_model: &CostModel,
) -> Result<QueryResults<ResultModeVerify, D>, TranscriptRejected<D>> {
    let path = path.into();
    update_counter(
        context,
        path.as_slice(),
        amount,
        true,
        gas_limit,
        cost_model,
    )
}

fn update_counter<D: DB>(
    context: &QueryContext<D>,
    path: &[u8],
    amount: u16,
    subtract: bool,
    gas_limit: Option<RunningCost>,
    cost_model: &CostModel,
) -> Result<QueryResults<ResultModeVerify, D>, TranscriptRejected<D>> {
    context.query(
        &counter_program(path, amount, subtract),
        gas_limit,
        cost_model,
    )
}

pub(crate) fn counter_program<D: DB>(
    path: &[u8],
    amount: u16,
    subtract: bool,
) -> Vec<Op<ResultModeVerify, D>> {
    let arithmetic = if subtract {
        Op::Subi {
            immediate: amount.into(),
        }
    } else {
        Op::Addi {
            immediate: amount.into(),
        }
    };
    vec![
        Op::Idx {
            cached: false,
            push_path: true,
            path: path_keys(path).into(),
        },
        arithmetic,
        Op::Ins {
            cached: true,
            n: path.len() as u8,
        },
    ]
}
