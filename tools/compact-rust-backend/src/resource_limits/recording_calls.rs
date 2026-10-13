// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Bounded borrowed call facts for recorded-helper discovery. These are source
//! references, not declaration resolution or recording admission certificates.
use super::{Limits, Metrics, Node, Scan, check_frames, children};
use crate::ir::{Expr, StateAction, StatefulCircuit};
use std::collections::HashSet;

/// Existing policy bounds for the caller's cached declaration/target graph.
/// Source traversal independently bounds every node before producing facts.
pub(crate) fn recording_call_fact_limits() -> (usize, usize) {
    (Limits::DEFAULT.declarations, Limits::DEFAULT.nodes)
}

/// Direct expression/statement call targets anywhere in this declaration.
///
/// The caller resolves these raw names against the original stateful registry;
/// an expression call may instead name a pure declaration or be invalid. Pure
/// statement/fold targets and witness names are not helper facts, but all their
/// operands are visited. Callee bodies are never followed here.
pub(crate) fn recording_call_targets(
    circuit: &StatefulCircuit,
) -> Result<HashSet<&str>, crate::RenderError> {
    crate::located(circuit.source.as_ref(), || {
        measure_targets(circuit, Limits::DEFAULT).map_err(|error| {
            crate::RenderError::ResourceLimit {
                resource: error.resource.label(),
                observed: error.observed,
                limit: error.limit,
            }
        })
    })
}

fn measure_targets(
    circuit: &StatefulCircuit,
    limits: Limits,
) -> Result<HashSet<&str>, super::LimitError> {
    let mut scan = Scan {
        limits,
        metrics: Metrics::default(),
        pending: Vec::new(),
        pure_count: 0,
        callable_count: 0,
        own: Vec::new(),
        height: Vec::new(),
        current_cost: [0; 4],
        path_height: Vec::new(),
        edges: Vec::new(),
    };
    let mut targets = HashSet::new();
    scan.push(Node::StatefulCircuit(circuit), 1, None)?;
    while let Some(visit) = scan.pending.pop() {
        scan.current_cost = visit.cost;
        scan.metrics.nodes = scan.metrics.nodes.saturating_add(1);
        scan.metrics.syntax_depth = scan.metrics.syntax_depth.max(visit.depth);
        // Match the existing stateful source preflight: pure-render frames do
        // not apply to this declaration, but all stateful phases remain bounded.
        for phase in 1..4 {
            scan.metrics.render_path_units[phase] =
                scan.metrics.render_path_units[phase].max(visit.cost[phase]);
        }
        match visit.node {
            Node::StateAction(StateAction::CircuitCall { name, .. })
            | Node::Expr(Expr::Call { name, .. }) => {
                targets.insert(name.as_str());
            }
            _ => {}
        }
        // The exhaustive shared visitor owns every carrier and operand. Owner
        // None prevents its mixed pure/stateful budgeting graph from collecting
        // or following calls; only the two source producers above emit facts.
        children(visit, &mut scan)?;
    }
    check_frames(&scan.metrics, limits)?;
    // Each target is borrowed from a visited node, so cardinality cannot exceed
    // the already enforced source-node bound, even for repeated/unknown calls.
    debug_assert!(targets.len() <= scan.metrics.nodes);
    Ok(targets)
}

#[cfg(test)]
mod tests;
