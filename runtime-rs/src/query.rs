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

//
// Purpose-named wrappers around `QueryContext::query` that force the correct
// `ResultMode` for read vs verify paths. Reading from ledger state (e.g. via
// `ledger().round()`) needs `ResultModeGather` to actually emit events; using
// `ResultModeVerify` silently produces empty events and reads come back
// missing.

use crate::{
    gather_ops_to_public_transcript, CompactError, CostModel, Op, PartialProofData, QueryContext,
    QueryResults, ResultModeGather, ResultModeVerify, RunningCost, TranscriptRejected, DB,
};

/// Run an Op program that should produce read events (e.g. `popeq`). Use this
/// for ledger view accessors. The returned `QueryResults` carries the
/// `GatherEvent::Read(...)` entries the caller decodes.
pub fn query_for_read<D: DB>(
    ctx: &QueryContext<D>,
    ops: &[Op<ResultModeGather, D>],
    gas_limit: Option<RunningCost>,
    cost_model: &CostModel,
) -> Result<QueryResults<ResultModeGather, D>, TranscriptRejected<D>> {
    ctx.query(ops, gas_limit, cost_model)
}

/// Run an Op program in verification-only mode — no events emitted, used for
/// state-mutation paths (e.g. circuits whose return value is `()`). Use this
/// for impure / provable circuit emissions where only the resulting
/// transcript / state matters.
pub fn query_for_verify<D: DB>(
    ctx: &QueryContext<D>,
    ops: &[Op<ResultModeVerify, D>],
    gas_limit: Option<RunningCost>,
    cost_model: &CostModel,
) -> Result<QueryResults<ResultModeVerify, D>, TranscriptRejected<D>> {
    ctx.query(ops, gas_limit, cost_model)
}

/// Run a verification program and append the exact ordered public operations to
/// `proof_data` after the query succeeds.
pub fn recorded_query_for_verify<D: DB>(
    proof_data: &mut PartialProofData<D>,
    ctx: &QueryContext<D>,
    ops: &[Op<ResultModeVerify, D>],
    gas_limit: Option<RunningCost>,
    cost_model: &CostModel,
) -> Result<QueryResults<ResultModeVerify, D>, TranscriptRejected<D>> {
    let results = query_for_verify(ctx, ops, gas_limit, cost_model)?;
    proof_data.push_public_ops(ops.iter().cloned());
    Ok(results)
}

/// Run a gather/read program and append the Ledger8-compatible public
/// transcript (`popeq` operations filled with their read results) to
/// `proof_data` after the query succeeds.
pub fn recorded_query_for_read<D: DB>(
    proof_data: &mut PartialProofData<D>,
    ctx: &QueryContext<D>,
    ops: &[Op<ResultModeGather, D>],
    gas_limit: Option<RunningCost>,
    cost_model: &CostModel,
) -> Result<QueryResults<ResultModeGather, D>, CompactError> {
    let results = query_for_read(ctx, ops, gas_limit, cost_model)?;
    let reads = results.events.iter().filter_map(|event| match event {
        crate::onchain_vm::result_mode::GatherEvent::Read(av) => Some(av.clone()),
        crate::onchain_vm::result_mode::GatherEvent::Log(_) => None,
    });
    let public_ops = gather_ops_to_public_transcript(ops, reads)?;
    proof_data.push_public_ops(public_ops);
    Ok(results)
}
