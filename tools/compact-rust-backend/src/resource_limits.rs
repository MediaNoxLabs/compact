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
mod borrowed_types;
mod policy;
mod pure;
mod recording_calls;
use crate::ir::Contract;
use crate::source_diagnostics::{RefusalObserver, RefusalSite};
pub(crate) use borrowed_types::{
    recording_byte_payload_limit, recording_fixed_layout_limit, recording_pure_annotations,
    recording_source_types, recording_type_nodes,
};
use policy::{LimitError, Limits, ResourceKind as Kind, check};
pub(crate) use pure::PureBodyBudget;
pub(crate) use recording_calls::{recording_call_fact_limits, recording_call_targets};

/// Resources have been checked, but an otherwise valid pure call cycle must
/// still be refused before publishing the rendered source.
#[must_use]
pub(crate) struct Preflight<'a> {
    pure_cycle: Option<usize>,
    pure_cycle_site: Option<RefusalSite<'a>>,
    original_contract: &'a Contract,
    observer: Option<&'a dyn RefusalObserver>,
}

impl Preflight<'_> {
    pub(crate) fn finish(self, contract: &Contract) -> Result<(), crate::RenderError> {
        if let Some(index) = self.pure_cycle {
            let circuit = &contract.circuits[index];
            if let Some(observer) = self.observer {
                // Private callers must finish the same input to retain a source
                // identity. A different contract keeps the old refusal/point,
                // but cannot borrow this input's closing-edge provenance.
                let site = if std::ptr::eq(self.original_contract, contract) {
                    self.pure_cycle_site.unwrap_or(RefusalSite::unavailable())
                } else {
                    RefusalSite::unavailable()
                };
                observer.refused(site);
            }
            return crate::located(circuit.source.as_ref(), || {
                Err(crate::RenderError::RecursivePureCall(circuit.name.clone()))
            });
        }
        Ok(())
    }
}

pub(crate) fn validate(contract: &Contract) -> Result<Preflight<'_>, crate::RenderError> {
    validate_with_observer(contract, None)
}
pub(crate) fn validate_with_observer<'a>(
    contract: &'a Contract,
    observer: Option<&'a dyn RefusalObserver>,
) -> Result<Preflight<'a>, crate::RenderError> {
    let mut pure_cycle_site = None;
    let measured = match observer {
        Some(observer) => measure_with_cycle_site(
            contract,
            Limits::DEFAULT,
            Some(observer),
            &mut pure_cycle_site,
        ),
        None => measure(contract, Limits::DEFAULT),
    };
    measured
        .map(|metrics| Preflight {
            pure_cycle_site,
            original_contract: contract,
            observer,
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

/// Defensive bounds for checked recording calls also entered by private tests.
/// Public generation still performs the complete borrowed-IR preflight first.
pub(crate) fn recording_call_limits() -> (usize, usize) {
    (Limits::DEFAULT.call_depth, Limits::DEFAULT.expanded_work)
}
/// Conversion expansion is independent of annotation size. These use the
/// production allowances; the test-only census envelope is not support policy.
pub(crate) fn coercion_work_limit() -> usize {
    Limits::DEFAULT.expanded_work
}
pub(crate) fn coercion_pending_limit() -> usize {
    Limits::DEFAULT.pending
}
pub(crate) fn recording_block_depth_limit() -> usize {
    Limits::DEFAULT.expanded_depth
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
    site: RefusalSite<'a>,
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
    observer: Option<&'a dyn RefusalObserver>,
    current_site: RefusalSite<'a>,
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
        self.reserve_for(n, self.current_site)
    }
    fn check_site(
        &self,
        resource: Kind,
        value: usize,
        limit: usize,
        site: RefusalSite<'_>,
    ) -> Result<(), LimitError> {
        check(resource, value, limit).inspect_err(|_| {
            if let Some(observer) = self.observer {
                observer.refused(site);
            }
        })
    }
    fn reserve_for(&self, n: usize, site: RefusalSite<'_>) -> Result<(), LimitError> {
        self.check_site(
            Kind::PendingNodes,
            self.pending.len().saturating_add(n),
            self.limits.pending,
            site,
        )?;
        self.check_site(
            Kind::Nodes,
            self.metrics
                .nodes
                .saturating_add(self.pending.len())
                .saturating_add(n),
            self.limits.nodes,
            site,
        )
    }
    fn push(
        &mut self,
        node: Node<'a>,
        depth: usize,
        owner: Option<usize>,
    ) -> Result<(), LimitError> {
        let site = node.origin_site();
        self.reserve_for(1, site)?;
        self.check_site(Kind::SyntaxDepth, depth, self.limits.syntax_depth, site)?;
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
        self.check_site(
            Kind::StringBytes,
            self.metrics.string_bytes,
            self.limits.strings,
            self.current_site,
        )
    }
    fn literal_bytes(&mut self, n: usize) -> Result<(), LimitError> {
        self.metrics.literal_bytes = self.metrics.literal_bytes.saturating_add(n);
        self.check_site(
            Kind::LiteralBytes,
            self.metrics.literal_bytes,
            self.limits.literal_bytes,
            self.current_site,
        )
    }
    fn call(&mut self, owner: Option<usize>, name: &'a str, depth: usize) {
        if let Some(i) = owner {
            self.edges[i].push(CallEdge {
                site: self.current_site,
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
    measure_observed(contract, limits, None)
}
fn measure_observed<'a>(
    contract: &'a Contract,
    limits: Limits,
    observer: Option<&'a dyn RefusalObserver>,
) -> Result<Metrics, LimitError> {
    measure_with_cycle_site(contract, limits, observer, &mut None)
}

fn measure_with_cycle_site<'a>(
    contract: &'a Contract,
    limits: Limits,
    observer: Option<&'a dyn RefusalObserver>,
    pure_cycle_site: &mut Option<RefusalSite<'a>>,
) -> Result<Metrics, LimitError> {
    // The public entry caller retains schema precedence; this standalone visitor is metrics-only.
    let count = contract
        .circuits
        .len()
        .saturating_add(contract.stateful_circuits.len())
        .saturating_add(usize::from(contract.constructor.is_some()));
    check(Kind::Declarations, count, limits.declarations)?;
    let mut scan = Scan {
        observer,
        current_site: RefusalSite::unavailable(),
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
    call_graph::finish(contract, scan, pure_cycle_site)
}

mod call_graph;
#[cfg(test)]
mod tests;

#[cfg(test)]
mod pure_cycles;

#[cfg(test)]
mod seeded_graph_tests;

#[cfg(test)]
mod pure_cycle_diagnostic_tests;
