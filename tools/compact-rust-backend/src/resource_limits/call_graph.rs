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

//! Saturating DAG expansion over the renderer's existing raw declaration keys.
//! This does not resolve or reinterpret invalid calls: duplicate, unknown and
//! cyclic inputs return to the semantic owner after local resource checks.
use super::{
    CallEdge, Contract, GraphStatus, Kind, LimitError, Metrics, Scan, add_cost, check,
    check_frames, max_cost,
};
use std::collections::HashMap;
#[derive(Clone, Copy)]
struct ResolvedEdge {
    target: usize,
    depth: usize,
    cost: [usize; 4],
}
pub(super) fn finish(contract: &Contract, mut scan: Scan<'_>) -> Result<Metrics, LimitError> {
    let count = scan.callable_count;
    let limits = scan.limits;
    let mut names = HashMap::new();
    for (i, name) in contract
        .circuits
        .iter()
        .map(|c| c.name.as_str())
        .chain(contract.stateful_circuits.iter().map(|c| c.name.as_str()))
        .enumerate()
    {
        if names.insert(name, i).is_some() {
            scan.metrics.graph_status = GraphStatus::DuplicateName;
            return Ok(scan.metrics);
        }
    }
    let mut graph = vec![Vec::new(); count];
    for (i, edges) in scan.edges.iter().enumerate() {
        for &CallEdge { name, depth, cost } in edges {
            let Some(&j) = names.get(name) else {
                scan.metrics.graph_status = GraphStatus::UnknownCallee;
                return Ok(scan.metrics);
            };
            graph[i].push(ResolvedEdge {
                target: j,
                depth,
                cost,
            });
        }
    }
    let mut color = vec![0u8; count];
    let mut order = Vec::with_capacity(count);
    for start in 0..count {
        if color[start] != 0 {
            continue;
        }
        color[start] = 1;
        let mut stack = vec![(start, 0)];
        while let Some((node, next)) = stack.last_mut() {
            if *next == graph[*node].len() {
                let i = *node;
                color[i] = 2;
                order.push(i);
                stack.pop();
                continue;
            }
            let j = graph[*node][*next].target;
            *next += 1;
            if color[j] == 1 {
                scan.metrics.graph_status = GraphStatus::Cycle;
                return Ok(scan.metrics);
            }
            if color[j] == 0 {
                color[j] = 1;
                stack.push((j, 0));
            }
        }
    }
    let mut frames = scan.path_height;
    let mut work = scan.own;
    let mut height = scan.height;
    let mut calls = vec![1usize; count];
    for i in order {
        for &ResolvedEdge {
            target: j,
            depth: d,
            cost,
        } in &graph[i]
        {
            frames[i] = max_cost(frames[i], add_cost(cost, frames[j]));
            work[i] = work[i].saturating_add(work[j]);
            height[i] = height[i].max(d.saturating_add(height[j]));
            calls[i] = calls[i].max(calls[j].saturating_add(1));
        }
        if i >= scan.pure_count {
            for phase in [2, 3] {
                scan.metrics.render_path_units[phase] =
                    scan.metrics.render_path_units[phase].max(frames[i][phase]);
            }
        }
        scan.metrics.expanded_work = scan.metrics.expanded_work.saturating_add(work[i]);
        scan.metrics.expanded_depth = scan.metrics.expanded_depth.max(height[i]);
        scan.metrics.call_depth = scan.metrics.call_depth.max(calls[i]);
    }
    for phase in [0, 1] {
        scan.metrics.render_path_units[phase] = scan.metrics.local_path_units[phase];
    }
    check_frames(&scan.metrics, limits)?;
    check(
        Kind::ExpandedWork,
        scan.metrics.expanded_work,
        limits.expanded_work,
    )?;
    check(
        Kind::ExpandedDepth,
        scan.metrics.expanded_depth,
        limits.expanded_depth,
    )?;
    check(Kind::CallDepth, scan.metrics.call_depth, limits.call_depth)?;
    Ok(scan.metrics)
}
