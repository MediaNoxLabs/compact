// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Cached source facts and finite declaration reachability; never support authority.
use crate::{RenderError, ir::StatefulCircuit, resource_limits};
use std::collections::HashMap;

pub(in crate::recorded) struct CallGraph {
    has_calls: Vec<bool>,
    edges: Vec<Vec<usize>>,
}

impl CallGraph {
    pub(in crate::recorded) fn from_declarations(
        circuits: &[StatefulCircuit],
    ) -> Result<Self, RenderError> {
        let (declarations, targets) = resource_limits::recording_call_fact_limits();
        bound("declarations", circuits.len(), declarations)?;
        let indices: HashMap<_, _> = circuits
            .iter()
            .enumerate()
            .map(|(index, c)| (c.name.as_str(), index))
            .collect();
        let mut graph = Self {
            has_calls: Vec::with_capacity(circuits.len()),
            edges: Vec::with_capacity(circuits.len()),
        };
        let mut target_count = 0_usize;
        for circuit in circuits {
            let raw = resource_limits::recording_call_targets(circuit)?;
            target_count = target_count.saturating_add(raw.len());
            bound("nodes", target_count, targets)?;
            // A raw pure Call keeps its historical root preview, but cannot
            // become a stateful helper. Witness/builtin/Fold names aren't facts.
            graph.has_calls.push(!raw.is_empty());
            let mut edges: Vec<_> = raw
                .into_iter()
                .filter_map(|name| indices.get(name).copied())
                .collect();
            edges.sort_unstable();
            graph.edges.push(edges);
        }
        Ok(graph)
    }

    pub(in crate::recorded) fn has_calls(&self, index: usize) -> bool {
        self.has_calls[index]
    }

    /// Start with direct callees of the supplied roots, following each selected
    /// declaration once. A source cycle is finite here; checking decides refusal.
    pub(in crate::recorded) fn reachable_from(
        &self,
        roots: impl IntoIterator<Item = usize>,
    ) -> Vec<bool> {
        let mut selected = vec![false; self.edges.len()];
        let mut pending = Vec::new();
        let enqueue = |index: usize, selected: &mut [bool], pending: &mut Vec<usize>| {
            if !selected[index] {
                selected[index] = true;
                pending.push(index);
            }
        };
        for root in roots {
            for &index in &self.edges[root] {
                enqueue(index, &mut selected, &mut pending);
            }
        }
        while let Some(index) = pending.pop() {
            for &child in &self.edges[index] {
                enqueue(child, &mut selected, &mut pending);
            }
        }
        selected
    }
}

fn bound(resource: &'static str, observed: usize, limit: usize) -> Result<(), RenderError> {
    if observed > limit {
        Err(RenderError::ResourceLimit {
            resource,
            observed,
            limit,
        })
    } else {
        Ok(())
    }
}
