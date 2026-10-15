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

//! Discovery, stable naming and emission planning for shared recorded helpers.
use super::*;

pub(super) mod call_graph;
#[cfg(test)]
mod naming_tests;
mod reachability;
use call_graph::CallGraph;
pub(crate) use reachability::retain_reachable_helpers;

pub(super) fn helper_ident(
    name: &str,
    circuits: &HashMap<&str, &StatefulCircuit>,
) -> Result<syn::Ident, RenderError> {
    crate::naming::recorded_helper_ident(name, circuits)
}

/// Plan optional sharing from bounded semantic call facts. Original declarations
/// and complete checked helper rendering own admission, independently of shape.
pub(crate) fn plan_recorded_helpers(
    ordered_circuits: &[StatefulCircuit],
    ledger_fields: &HashMap<&str, &LedgerField>,
    witnesses: &HashMap<&str, &WitnessDeclaration>,
    pure_circuits: &HashMap<&str, &PureCircuit>,
    circuits: &HashMap<&str, &StatefulCircuit>,
) -> Result<(HashSet<String>, Vec<syn::Item>), RenderError> {
    #[cfg(test)]
    let _phase = admission_probe::trace::HelperPlanning::begin();
    let graph = CallGraph::from_declarations(ordered_circuits)?;
    let roots: Vec<_> = ordered_circuits
        .iter()
        .enumerate()
        .filter_map(|(index, circuit)| {
            (!circuit.internal && graph.has_calls(index)).then_some(index)
        })
        .collect();
    if roots.is_empty() {
        return Ok((HashSet::new(), Vec::new()));
    }

    // Preserve provisional helper validation before complete root previews.
    // Original visibility remains intact: sharing never relaxes a public ABI.
    let prospective = graph.reachable_from(roots.iter().copied());
    let mut provisional_names = HashSet::new();
    for (index, circuit) in ordered_circuits.iter().enumerate() {
        if !prospective[index] {
            continue;
        }
        if crate::located(circuit.source.as_ref(), || {
            render_recorded_helper(
                circuit,
                ledger_fields,
                witnesses,
                pure_circuits,
                circuits,
                &HashSet::new(),
            )
        })?
        .is_supported()
        {
            provisional_names.insert(circuit.name.clone());
        }
    }

    // Only supported exported roots seed final optional sharing. Checked calls
    // themselves inline and cannot depend on membership in this name set.
    let mut supported_roots = Vec::new();
    for index in roots {
        let circuit = &ordered_circuits[index];
        let recorded = crate::located(circuit.source.as_ref(), || {
            render_recorded_circuit(
                circuit,
                ledger_fields,
                witnesses,
                pure_circuits,
                circuits,
                &provisional_names,
            )
        })?;
        if recorded.is_supported() {
            supported_roots.push(index);
        }
    }
    let candidates = graph.reachable_from(supported_roots);
    let mut names = HashSet::new();
    for (index, circuit) in ordered_circuits.iter().enumerate() {
        if !candidates[index] {
            continue;
        }
        if crate::located(circuit.source.as_ref(), || {
            render_recorded_helper(
                circuit,
                ledger_fields,
                witnesses,
                pure_circuits,
                circuits,
                &HashSet::new(),
            )
        })?
        .is_supported()
        {
            names.insert(circuit.name.clone());
        }
    }
    let mut items = Vec::new();
    for circuit in ordered_circuits {
        if !names.contains(&circuit.name) {
            continue;
        }
        let item = crate::located(circuit.source.as_ref(), || {
            render_recorded_helper(
                circuit,
                ledger_fields,
                witnesses,
                pure_circuits,
                circuits,
                &names,
            )
        })?;
        if let RecordingOutcome::Supported(item) = item {
            items.push(item);
        }
    }
    Ok((names, items))
}
