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

//! Bounded typed-IR preflight, before recursive rendering. The borrowed visitor
//! budgets pending batches; graph arithmetic saturates. Pure-cycle facts are
//! retained for semantic refusal after the renderer's existing checks.
mod policy;
use crate::ir::Contract;
use policy::{LimitError, Limits, ResourceKind as Kind, check};

/// Resources have been checked, but an otherwise valid pure call cycle must
/// still be refused before publishing the rendered source.
#[must_use]
pub(crate) struct Preflight {
    pure_cycle: Option<usize>,
}

impl Preflight {
    pub(crate) fn finish(self, contract: &Contract) -> Result<(), crate::RenderError> {
        if let Some(index) = self.pure_cycle {
            let circuit = &contract.circuits[index];
            return crate::located(circuit.source.as_ref(), || {
                Err(crate::RenderError::RecursivePureCall(circuit.name.clone()))
            });
        }
        Ok(())
    }
}

pub(crate) fn validate(contract: &Contract) -> Result<Preflight, crate::RenderError> {
    measure(contract, Limits::DEFAULT)
        .map(|metrics| Preflight {
            pure_cycle: match metrics.graph_status {
                GraphStatus::PureCycle(index) => Some(index),
                _ => None,
            },
        })
        .map_err(|error| crate::RenderError::ResourceLimit {
            resource: error.resource.label(),
            observed: error.observed,
            limit: error.limit,
        })
}
fn check_frames(metrics: &Metrics, limits: Limits) -> Result<(), LimitError> {
    for (i, kind) in [
        Kind::PurePath,
        Kind::NativePath,
        Kind::LegacyRecordedPath,
        Kind::TypedRecordedPath,
    ]
    .into_iter()
    .enumerate()
    {
        check(kind, metrics.render_path_units[i], limits.path_units)?;
    }
    Ok(())
}
#[derive(Default, Debug, PartialEq, Eq)]
enum GraphStatus {
    #[default]
    Acyclic,
    DuplicateName,
    UnknownCallee,
    Cycle,
    PureCycle(usize),
}

#[derive(Clone, Copy)]
struct CallEdge<'a> {
    name: &'a str,
    depth: usize,
    cost: [usize; 4],
}

#[derive(Default, Debug)]
struct Metrics {
    nodes: usize,
    pending_peak: usize,
    string_bytes: usize,
    literal_bytes: usize,
    syntax_depth: usize,
    call_edges: usize,
    call_depth: usize,
    expanded_depth: usize,
    expanded_work: usize,
    render_path_units: [usize; 4],
    local_path_units: [usize; 4],
    graph_status: GraphStatus,
}
#[derive(Clone, Copy)]
struct Visit<'a> {
    node: Node<'a>,
    depth: usize,
    owner: Option<usize>,
    cost: [usize; 4],
}
struct Scan<'a> {
    limits: Limits,
    metrics: Metrics,
    pending: Vec<Visit<'a>>,
    pure_count: usize,
    callable_count: usize,
    own: Vec<usize>,
    height: Vec<usize>,
    current_cost: [usize; 4],
    path_height: Vec<[usize; 4]>,
    edges: Vec<Vec<CallEdge<'a>>>,
}
impl<'a> Scan<'a> {
    fn reserve(&self, n: usize) -> Result<(), LimitError> {
        check(
            Kind::PendingNodes,
            self.pending.len().saturating_add(n),
            self.limits.pending,
        )?;
        check(
            Kind::Nodes,
            self.metrics
                .nodes
                .saturating_add(self.pending.len())
                .saturating_add(n),
            self.limits.nodes,
        )
    }
    fn push(
        &mut self,
        node: Node<'a>,
        depth: usize,
        owner: Option<usize>,
    ) -> Result<(), LimitError> {
        self.reserve(1)?;
        check(Kind::SyntaxDepth, depth, self.limits.syntax_depth)?;
        let cost = add_cost(
            if depth == 1 {
                [0; 4]
            } else {
                self.current_cost
            },
            frame_cost(node),
        );
        self.pending.push(Visit {
            node,
            depth,
            owner,
            cost,
        });
        self.metrics.pending_peak = self.metrics.pending_peak.max(self.pending.len());
        Ok(())
    }
    fn text(&mut self, s: &str) -> Result<(), LimitError> {
        self.metrics.string_bytes = self.metrics.string_bytes.saturating_add(s.len());
        check(
            Kind::StringBytes,
            self.metrics.string_bytes,
            self.limits.strings,
        )
    }
    fn literal_bytes(&mut self, n: usize) -> Result<(), LimitError> {
        self.metrics.literal_bytes = self.metrics.literal_bytes.saturating_add(n);
        check(
            Kind::LiteralBytes,
            self.metrics.literal_bytes,
            self.limits.literal_bytes,
        )
    }
    fn call(&mut self, owner: Option<usize>, name: &'a str, depth: usize) {
        if let Some(i) = owner {
            self.edges[i].push(CallEdge {
                name,
                depth,
                cost: self.current_cost,
            });
            self.metrics.call_edges += 1;
        }
    }
}
mod traversal;
use traversal::{Node, children};
mod frames;
use frames::{add_cost, frame_cost, max_cost};

fn measure(contract: &Contract, limits: Limits) -> Result<Metrics, LimitError> {
    // The public entry caller retains schema precedence; this standalone visitor is metrics-only.
    let count = contract
        .circuits
        .len()
        .saturating_add(contract.stateful_circuits.len())
        .saturating_add(usize::from(contract.constructor.is_some()));
    check(Kind::Declarations, count, limits.declarations)?;
    let mut scan = Scan {
        limits,
        metrics: Metrics {
            graph_status: GraphStatus::Acyclic,
            ..Default::default()
        },
        pending: Vec::new(),
        pure_count: contract.circuits.len(),
        callable_count: count,
        own: vec![0; count],
        height: vec![0; count],
        current_cost: [0; 4],
        path_height: vec![[0; 4]; count],
        edges: vec![Vec::new(); count],
    };
    scan.push(Node::Contract(contract), 1, None)?;
    while let Some(v) = scan.pending.pop() {
        scan.current_cost = v.cost;
        scan.metrics.nodes = scan.metrics.nodes.saturating_add(1);
        scan.metrics.syntax_depth = scan.metrics.syntax_depth.max(v.depth);
        if let Some(i) = v.owner {
            scan.own[i] += 1;
            scan.height[i] = scan.height[i].max(v.depth);
            scan.path_height[i] = max_cost(scan.path_height[i], v.cost);
        }
        children(v, &mut scan)?;
    }
    for i in 0..count {
        let phase = usize::from(i >= scan.pure_count);
        scan.metrics.local_path_units[phase] =
            scan.metrics.local_path_units[phase].max(scan.path_height[i][phase]);
    }
    for i in 0..count {
        for phase in 0..4 {
            if (phase == 0 && i < scan.pure_count) || (phase > 0 && i >= scan.pure_count) {
                scan.metrics.render_path_units[phase] =
                    scan.metrics.render_path_units[phase].max(scan.path_height[i][phase]);
            }
        }
    }
    check_frames(&scan.metrics, limits)?;
    call_graph::finish(contract, scan)
}

mod call_graph;
#[cfg(test)]
mod tests;

#[cfg(test)]
mod pure_cycles;

#[cfg(test)]
mod seeded_graph_tests;
