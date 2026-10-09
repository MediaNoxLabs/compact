// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Borrowed pure declaration checks before native typing, equality or cloning.
//! Reuses the exhaustive resource visitor; it does not certify purity or types.
use super::*;
use crate::ir::PureCircuit;

#[derive(Debug, Default)]
pub(crate) struct PureBodyBudget {
    declarations: usize,
    nodes: usize,
    strings: usize,
    literals: usize,
}
impl PureBodyBudget {
    pub(crate) fn check(&mut self, circuit: &PureCircuit) -> Result<usize, crate::RenderError> {
        self.measure(circuit)
            .map_err(|error| crate::RenderError::ResourceLimit {
                resource: error.resource.label(),
                observed: error.observed,
                limit: error.limit,
            })
    }
    fn measure(&mut self, circuit: &PureCircuit) -> Result<usize, LimitError> {
        let limits = Limits::DEFAULT;
        self.declarations = self.declarations.saturating_add(1);
        check(Kind::Declarations, self.declarations, limits.declarations)?;
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
        scan.push(Node::PureCircuit(circuit), 1, None)?;
        while let Some(visit) = scan.pending.pop() {
            scan.current_cost = visit.cost;
            scan.metrics.nodes = scan.metrics.nodes.saturating_add(1);
            scan.metrics.syntax_depth = scan.metrics.syntax_depth.max(visit.depth);
            scan.metrics.render_path_units[0] =
                scan.metrics.render_path_units[0].max(visit.cost[0]);
            children(visit, &mut scan)?;
        }
        check_frames(&scan.metrics, limits)?;
        self.nodes = self.nodes.saturating_add(scan.metrics.nodes);
        self.strings = self.strings.saturating_add(scan.metrics.string_bytes);
        self.literals = self.literals.saturating_add(scan.metrics.literal_bytes);
        check(Kind::Nodes, self.nodes, limits.nodes)?;
        check(Kind::StringBytes, self.strings, limits.strings)?;
        check(Kind::LiteralBytes, self.literals, limits.literal_bytes)?;
        Ok(scan.metrics.nodes)
    }
}
