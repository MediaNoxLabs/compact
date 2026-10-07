// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Finite generated-input properties for ordinary Unit call DAGs.
//! The oracle unfolds occurrences, independently of the production graph DP.
//! These private-IR cases do not claim Compact source or arbitrary-input safety.

use super::{GraphStatus, Kind, Limits, measure};
use crate::ir::{Contract, Expr, SCHEMA_VERSION, StateAction, StateReturn, StatefulCircuit, Type};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

const SEEDS: [u64; 4] = [0x0301_7001, 0x0301_7002, 0x0301_7003, 0x0301_7004];
const CASES_PER_SEED: usize = 16;
const MAX_DECLARATIONS: usize = 8;
const MAX_OUTGOING: usize = 3;
const MAX_INPUT_BYTES: usize = 32 * 1024;
// At most eight roots and a ternary tree of depth eight per root.
const MAX_UNFOLDED_OCCURRENCES: usize = 8 * 3280;

struct Sequence(u64);

impl Sequence {
    fn below(&mut self, bound: usize) -> usize {
        assert!(bound > 0);
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((self.0 >> 32) as usize) % bound
    }
}

#[derive(Debug)]
struct Case {
    seed: u64,
    ordinal: usize,
    // Indices are declaration identities; duplicate targets are distinct calls.
    edges: Vec<Vec<usize>>,
}

impl Case {
    fn contract(&self, order: &[usize]) -> Contract {
        let mut names = vec![String::new(); order.len()];
        for (position, &identity) in order.iter().enumerate() {
            names[identity] = format!("c{position:02}");
        }
        let stateful_circuits = order
            .iter()
            .map(|&identity| {
                let mut actions: Vec<_> = self.edges[identity]
                    .iter()
                    .map(|&target| StateAction::CircuitCall {
                        name: names[target].clone(),
                        arguments: vec![],
                    })
                    .collect();
                actions.push(StateAction::Assert {
                    condition: Expr::Boolean { value: true },
                    message: "ok".into(),
                });
                StatefulCircuit {
                    source: None,
                    internal: identity != 0,
                    name: names[identity].clone(),
                    parameters: vec![],
                    actions,
                    result: Type::Unit,
                    return_value: StateReturn::Unit,
                }
            })
            .collect();
        Contract {
            schema_version: SCHEMA_VERSION,
            type_aliases: vec![],
            constructor: None,
            witnesses: vec![],
            ledger_fields: vec![],
            circuits: vec![],
            stateful_circuits,
        }
    }

    fn ordinary_contract(&self) -> Contract {
        self.contract(&(0..self.edges.len()).collect::<Vec<_>>())
    }

    fn permuted_contract(&self) -> Contract {
        let mut order: Vec<_> = (0..self.edges.len()).collect();
        let mut sequence = Sequence(self.seed ^ ((self.ordinal as u64) << 32));
        for end in (1..order.len()).rev() {
            let other = sequence.below(end + 1);
            order.swap(end, other);
        }
        self.contract(&order)
    }

    fn diagnostic(&self, contract: &Contract) -> String {
        format!(
            "seed={:#x} case={} edges={:?} input={}",
            self.seed,
            self.ordinal,
            self.edges,
            serde_json::to_string(contract).unwrap()
        )
    }
}

fn cases() -> Vec<Case> {
    let mut cases = Vec::new();
    for seed in SEEDS {
        let mut sequence = Sequence(seed);
        for ordinal in 0..CASES_PER_SEED {
            let edges = match ordinal {
                0 => vec![vec![]],
                1 => vec![vec![1, 2], vec![3], vec![3], vec![]],
                2 => vec![vec![1, 1, 1], vec![2, 2], vec![]],
                3 => vec![vec![1], vec![], vec![3], vec![]],
                _ => {
                    let count = 2 + sequence.below(MAX_DECLARATIONS - 1);
                    (0..count)
                        .map(|from| {
                            if from + 1 == count {
                                vec![]
                            } else {
                                (0..sequence.below(MAX_OUTGOING + 1))
                                    .map(|_| from + 1 + sequence.below(count - from - 1))
                                    .collect()
                            }
                        })
                        .collect()
                }
            };
            cases.push(Case {
                seed,
                ordinal,
                edges,
            });
        }
    }
    cases
}

#[derive(Debug, PartialEq, Eq)]
struct Expected {
    nodes: usize,
    call_edges: usize,
    call_depth: usize,
    expanded_work: usize,
    occurrences: usize,
}

fn expected(case: &Case) -> Expected {
    // Four nodes: declaration, name, Unit result type, Unit return.
    // Three more: Assert action, Boolean condition, message text.
    // Each call occurrence contributes its action and target-name text.
    let local_work: Vec<_> = case.edges.iter().map(|edges| 7 + 2 * edges.len()).collect();
    let mut expected = Expected {
        nodes: 1 + local_work.iter().sum::<usize>(),
        call_edges: case.edges.iter().map(Vec::len).sum(),
        call_depth: 0,
        expanded_work: 0,
        occurrences: 0,
    };
    // Explicit occurrence unfolding, no memoization/topological graph reduction.
    // Every declaration is a root, including unreachable/internal components.
    let mut pending: Vec<_> = (0..case.edges.len()).map(|root| (root, 1)).collect();
    while let Some((node, depth)) = pending.pop() {
        expected.occurrences += 1;
        assert!(expected.occurrences <= MAX_UNFOLDED_OCCURRENCES);
        expected.expanded_work += local_work[node];
        expected.call_depth = expected.call_depth.max(depth);
        pending.extend(case.edges[node].iter().map(|&target| (target, depth + 1)));
    }
    expected
}

#[test]
fn literal_unit_examples_pin_the_independent_work_model() {
    for (edges, nodes, calls, depth, work, occurrences) in [
        (vec![vec![]], 8, 0, 1, 7, 1),
        (vec![vec![1], vec![]], 17, 1, 2, 23, 3),
        (vec![vec![1, 1], vec![]], 19, 2, 2, 32, 4),
    ] {
        let case = Case {
            seed: 0,
            ordinal: 0,
            edges,
        };
        assert_eq!(
            expected(&case),
            Expected {
                nodes,
                call_edges: calls,
                call_depth: depth,
                expanded_work: work,
                occurrences,
            }
        );
        let measured = measure(&case.ordinary_contract(), Limits::CENSUS).unwrap();
        assert_eq!(measured.nodes, nodes);
        assert_eq!(measured.expanded_work, work);
    }
}

#[test]
fn seeded_case_inventory_is_bounded_and_retains_required_shapes() {
    let cases = cases();
    assert_eq!(cases.len(), SEEDS.len() * CASES_PER_SEED);
    let mut unique = BTreeSet::new();
    let mut rows = Vec::new();
    let mut motifs = [0; 3];
    for case in &cases {
        assert!((1..=MAX_DECLARATIONS).contains(&case.edges.len()));
        for (from, edges) in case.edges.iter().enumerate() {
            assert!(edges.len() <= MAX_OUTGOING);
            assert!(edges.iter().all(|&to| to > from && to < case.edges.len()));
        }
        match case.ordinal {
            1 => {
                assert_eq!(case.edges, [vec![1, 2], vec![3], vec![3], vec![]]);
                motifs[0] += 1;
            }
            2 => {
                assert_eq!(case.edges, [vec![1, 1, 1], vec![2, 2], vec![]]);
                motifs[1] += 1;
            }
            3 => {
                assert_eq!(case.edges, [vec![1], vec![], vec![3], vec![]]);
                motifs[2] += 1;
            }
            _ => {}
        }
        let input = serde_json::to_vec(&case.ordinary_contract()).unwrap();
        assert!(input.len() <= MAX_INPUT_BYTES);
        let bytes = input.len();
        let hash = format!("{:x}", Sha256::digest(&input));
        unique.insert(input);
        let model = expected(case);
        rows.push(serde_json::json!({
            "seed":format!("{:#x}",case.seed),"case":case.ordinal,"edges":case.edges,
            "input_bytes":bytes,"input_sha256":hash,"nodes":model.nodes,
            "call_edges":model.call_edges,"call_depth":model.call_depth,
            "expanded_work":model.expanded_work,"occurrences":model.occurrences,
        }));
    }
    assert_eq!(motifs, [SEEDS.len(); 3]);
    println!(
        "seeded_graph_inventory={}",
        serde_json::json!({
            "seeds":SEEDS,"cases":cases.len(),"unique_inputs":unique.len(),
            "required_diamond_cases":motifs[0],"required_repeated_cases":motifs[1],
            "required_disconnected_cases":motifs[2],"rows":rows,
        })
    );
}

#[test]
fn seeded_unit_dags_match_occurrence_work_and_permuted_declarations() {
    for case in cases() {
        let contract = case.ordinary_contract();
        let diagnostic = case.diagnostic(&contract);
        let expected = expected(&case);
        let measured = measure(&contract, Limits::CENSUS).expect(&diagnostic);
        assert_eq!(measured.graph_status, GraphStatus::Acyclic, "{diagnostic}");
        assert_eq!(measured.nodes, expected.nodes, "nodes: {diagnostic}");
        assert_eq!(
            measured.call_edges, expected.call_edges,
            "edges: {diagnostic}"
        );
        assert_eq!(
            measured.call_depth, expected.call_depth,
            "depth: {diagnostic}"
        );
        assert_eq!(
            measured.expanded_work, expected.expanded_work,
            "expanded occurrence work: {diagnostic}"
        );
        let permuted = measure(&case.permuted_contract(), Limits::CENSUS).expect(&diagnostic);
        assert_eq!(permuted.graph_status, GraphStatus::Acyclic, "{diagnostic}");
        assert_eq!(
            (
                permuted.nodes,
                permuted.call_edges,
                permuted.call_depth,
                permuted.expanded_depth,
                permuted.expanded_work,
                permuted.string_bytes,
                permuted.literal_bytes,
            ),
            (
                measured.nodes,
                measured.call_edges,
                measured.call_depth,
                measured.expanded_depth,
                measured.expanded_work,
                measured.string_bytes,
                measured.literal_bytes,
            ),
            "declaration permutation: {diagnostic}"
        );
    }
}

#[test]
fn seeded_unit_dags_accept_exact_work_and_depth_limits_and_reject_one_less() {
    for case in cases() {
        let contract = case.ordinary_contract();
        let diagnostic = case.diagnostic(&contract);
        let expected = expected(&case);
        for (kind, exact) in [
            (Kind::ExpandedWork, expected.expanded_work),
            (Kind::CallDepth, expected.call_depth),
        ] {
            let mut limits = Limits::CENSUS;
            match kind {
                Kind::ExpandedWork => limits.expanded_work = exact,
                Kind::CallDepth => limits.call_depth = exact,
                _ => unreachable!(),
            }
            assert!(
                measure(&contract, limits).is_ok(),
                "exact {kind:?} limit={exact} must accept: {diagnostic}"
            );
            match kind {
                Kind::ExpandedWork => limits.expanded_work -= 1,
                Kind::CallDepth => limits.call_depth -= 1,
                _ => unreachable!(),
            }
            let error = measure(&contract, limits).expect_err(&diagnostic);
            assert_eq!(
                (error.resource, error.limit, error.observed),
                (kind, exact - 1, exact),
                "one-under limit: {diagnostic}"
            );
        }
    }
}

#[test]
fn selected_seeded_dags_render_under_default_policy_on_ordinary_worker() {
    for case in cases().into_iter().filter(|case| case.ordinal == 4) {
        let contract = case.ordinary_contract();
        let diagnostic = case.diagnostic(&contract);
        measure(&contract, Limits::DEFAULT).expect(&diagnostic);
        let source = crate::render(&contract).expect(&diagnostic);
        syn::parse_file(&source).expect(&diagnostic);
    }
}
