// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Borrowed source/type bounds before named collectors, cloning or equality.
//! The existing exhaustive visitor owns traversal; this is not call certification.
use super::*;
use crate::ir::{LedgerField, PureCircuit, StatefulCircuit, Type, WitnessDeclaration};
use std::collections::HashMap;

fn render_error(error: LimitError) -> crate::RenderError {
    crate::RenderError::ResourceLimit {
        resource: error.resource.label(),
        observed: error.observed,
        limit: error.limit,
    }
}

struct Types<'a> {
    scan: Scan<'a>,
    types: Vec<&'a Type>,
    defaults: Vec<&'a Type>,
}
impl<'a> Types<'a> {
    fn new() -> Self {
        Self {
            scan: Scan {
                limits: Limits::DEFAULT,
                metrics: Metrics::default(),
                pending: Vec::new(),
                pure_count: 0,
                callable_count: 0,
                own: Vec::new(),
                height: Vec::new(),
                current_cost: [0; 4],
                path_height: Vec::new(),
                edges: Vec::new(),
            },
            types: Vec::new(),
            defaults: Vec::new(),
        }
    }
    fn source(&mut self, node: Node<'a>) -> Result<(), LimitError> {
        let phases = match node {
            Node::PureCircuit(_) => [true, false, false, false],
            Node::StatefulCircuit(_) => [false, true, true, true],
            _ => [true; 4],
        };
        self.scan.push(node, 1, None)?;
        while let Some(visit) = self.scan.pending.pop() {
            self.scan.current_cost = visit.cost;
            self.scan.metrics.nodes = self.scan.metrics.nodes.saturating_add(1);
            self.scan.metrics.syntax_depth = self.scan.metrics.syntax_depth.max(visit.depth);
            for (phase, enabled) in phases.into_iter().enumerate() {
                if enabled {
                    self.scan.metrics.render_path_units[phase] =
                        self.scan.metrics.render_path_units[phase].max(visit.cost[phase]);
                }
            }
            if let Node::Type(ty) = visit.node {
                self.types.push(ty);
            }
            if let Node::Expr(crate::ir::Expr::Default { ty }) = visit.node {
                self.defaults.push(ty);
            }
            children(visit, &mut self.scan)?;
        }
        check_frames(&self.scan.metrics, self.scan.limits)
    }
}

pub(crate) fn recording_type_nodes(ty: &Type) -> Result<Vec<&Type>, crate::RenderError> {
    let mut bounded = Types::new();
    bounded.source(Node::Type(ty)).map_err(render_error)?;
    Ok(bounded.types)
}

pub(crate) fn recording_source_types<'a>(
    root: &'a StatefulCircuit,
    ledger: &HashMap<&str, &'a LedgerField>,
    witnesses: &HashMap<&str, &'a WitnessDeclaration>,
    pure: &HashMap<&str, &'a PureCircuit>,
    stateful: &HashMap<&str, &'a StatefulCircuit>,
) -> Result<Vec<&'a Type>, crate::RenderError> {
    let declarations = pure
        .len()
        .saturating_add(stateful.len())
        .saturating_add(usize::from(
            !stateful.values().any(|c| std::ptr::eq(*c, root)),
        ));
    check(
        Kind::Declarations,
        declarations,
        Limits::DEFAULT.declarations,
    )
    .map_err(render_error)?;
    let mut bounded = Types::new();
    bounded
        .source(Node::StatefulCircuit(root))
        .map_err(render_error)?;
    for field in ledger.values() {
        bounded
            .source(Node::LedgerField(field))
            .map_err(render_error)?;
    }
    for witness in witnesses.values() {
        bounded
            .source(Node::WitnessDeclaration(witness))
            .map_err(render_error)?;
    }
    for circuit in pure.values() {
        bounded
            .source(Node::PureCircuit(circuit))
            .map_err(render_error)?;
    }
    for circuit in stateful.values() {
        if !std::ptr::eq(*circuit, root) {
            bounded
                .source(Node::StatefulCircuit(circuit))
                .map_err(render_error)?;
        }
    }
    Ok(bounded.types)
}

/// Fixed byte leaves per checked aggregate type, not total runtime memory.
pub(crate) fn recording_byte_payload_limit() -> usize {
    Limits::DEFAULT.literal_bytes
}

/// Annotations from one reachable pure body, using the exhaustive preflight.
pub(crate) struct RecordingPureAnnotations<'a> {
    pub(crate) types: Vec<&'a Type>,
    pub(crate) defaults: Vec<&'a Type>,
}
pub(crate) fn recording_pure_annotations(
    circuit: &PureCircuit,
) -> Result<RecordingPureAnnotations<'_>, crate::RenderError> {
    let mut bounded = Types::new();
    bounded
        .source(Node::PureCircuit(circuit))
        .map_err(render_error)?;
    Ok(RecordingPureAnnotations {
        types: bounded.types,
        defaults: bounded.defaults,
    })
}

/// Per-used-type fixed layout expansion; dynamic opaque payloads are not counted.
pub(crate) fn recording_fixed_layout_limit() -> usize {
    Limits::DEFAULT.nodes
}
