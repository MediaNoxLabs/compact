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

//! Positional roots for the default proof suite.
//!
//! Dispatch happens before this parser. Preserve operating-system path bytes,
//! optional-prefix order, and the original single extra-argument lookahead.
//! The caller owns path borrowing, environment reads, and proof execution.

use std::error::Error;
use std::ffi::OsString;

#[derive(Debug)]
pub(super) struct PositionalInputs {
    pub(super) counter_root: OsString,
    pub(super) cell_root: OsString,
    pub(super) cell_read_root: OsString,
    pub(super) witness_root: Option<OsString>,
    pub(super) nested_root: Option<OsString>,
    pub(super) expression_root: Option<OsString>,
    pub(super) set_root: Option<OsString>,
    pub(super) set_oracle_root: Option<OsString>,
    pub(super) map_root: Option<OsString>,
    pub(super) constructor_map_root: Option<OsString>,
    pub(super) list_root: Option<OsString>,
    pub(super) constructor_list_root: Option<OsString>,
    pub(super) enum_cell_root: Option<OsString>,
    pub(super) tiny_root: Option<OsString>,
    pub(super) nested_map_shape_root: Option<OsString>,
    pub(super) list_shapes_root: Option<OsString>,
    pub(super) merkle_root: Option<OsString>,
    pub(super) historic_merkle_root: Option<OsString>,
    pub(super) vector_key_root: Option<OsString>,
    pub(super) counter_parameter_root: Option<OsString>,
    pub(super) composite_key_root: Option<OsString>,
    pub(super) chunked_set_root: Option<OsString>,
    pub(super) chunked_list_root: Option<OsString>,
    pub(super) chunked_map_root: Option<OsString>,
    pub(super) chunked_cell_root: Option<OsString>,
    pub(super) uints_root: Option<OsString>,
    pub(super) wide_root: Option<OsString>,
}

impl PositionalInputs {
    pub(super) fn parse(
        first: Option<OsString>,
        arguments: &mut impl Iterator<Item = OsString>,
    ) -> Result<Self, Box<dyn Error>> {
        let counter_root = first.ok_or(
            "usage: compact-rust-proof-smoke <counter-output> <cell-output> <cell-read-output>",
        )?;
        let cell_root = arguments.next().ok_or(
            "usage: compact-rust-proof-smoke <counter-output> <cell-output> <cell-read-output>",
        )?;
        let cell_read_root = arguments.next().ok_or(
            "usage: compact-rust-proof-smoke <counter-output> <cell-output> <cell-read-output>",
        )?;
        let witness_root = arguments.next();
        let nested_root = arguments.next();
        let expression_root = arguments.next();
        let set_root = arguments.next();
        let set_oracle_root = arguments.next();
        let map_root = arguments.next();
        let constructor_map_root = arguments.next();
        let list_root = arguments.next();
        let constructor_list_root = arguments.next();
        let enum_cell_root = arguments.next();
        let tiny_root = arguments.next();
        let nested_map_shape_root = arguments.next();
        let list_shapes_root = arguments.next();
        let merkle_root = arguments.next();
        let historic_merkle_root = arguments.next();
        let vector_key_root = arguments.next();
        let counter_parameter_root = arguments.next();
        let composite_key_root = arguments.next();
        let chunked_set_root = arguments.next();
        let chunked_list_root = arguments.next();
        let chunked_map_root = arguments.next();
        let chunked_cell_root = arguments.next();
        let uints_root = arguments.next();
        let wide_root = arguments.next();
        if arguments.next().is_some() {
            return Err(
                "usage: compact-rust-proof-smoke <counter-output> <cell-output> <cell-read-output> [witness-output] [nested-output] [nested-witness-output] [set-output] [set-oracle-output] [map-output] [constructor-map-output] [list-output] [constructor-list-output] [enum-cell-output] [tiny-output] [nested-map-shape-output] [list-shapes-output] [merkle-output] [historic-merkle-output] [vector-key-output] [counter-parameter-output] [composite-key-output] [chunked-set-output] [chunked-list-output] [chunked-map-output] [chunked-cell-output] [uints-output] [wide-uint-output]"
                    .into(),
            );
        }
        Ok(Self {
            counter_root,
            cell_root,
            cell_read_root,
            witness_root,
            nested_root,
            expression_root,
            set_root,
            set_oracle_root,
            map_root,
            constructor_map_root,
            list_root,
            constructor_list_root,
            enum_cell_root,
            tiny_root,
            nested_map_shape_root,
            list_shapes_root,
            merkle_root,
            historic_merkle_root,
            vector_key_root,
            counter_parameter_root,
            composite_key_root,
            chunked_set_root,
            chunked_list_root,
            chunked_map_root,
            chunked_cell_root,
            uints_root,
            wide_root,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::PositionalInputs;
    use std::ffi::OsString;

    const MISSING_ROOT_USAGE: &str =
        "usage: compact-rust-proof-smoke <counter-output> <cell-output> <cell-read-output>";
    const EXTRA_ROOT_USAGE: &str = "usage: compact-rust-proof-smoke <counter-output> <cell-output> <cell-read-output> [witness-output] [nested-output] [nested-witness-output] [set-output] [set-oracle-output] [map-output] [constructor-map-output] [list-output] [constructor-list-output] [enum-cell-output] [tiny-output] [nested-map-shape-output] [list-shapes-output] [merkle-output] [historic-merkle-output] [vector-key-output] [counter-parameter-output] [composite-key-output] [chunked-set-output] [chunked-list-output] [chunked-map-output] [chunked-cell-output] [uints-output] [wide-uint-output]";

    fn roots_in_cli_order(inputs: PositionalInputs) -> Vec<Option<OsString>> {
        vec![
            Some(inputs.counter_root),
            Some(inputs.cell_root),
            Some(inputs.cell_read_root),
            inputs.witness_root,
            inputs.nested_root,
            inputs.expression_root,
            inputs.set_root,
            inputs.set_oracle_root,
            inputs.map_root,
            inputs.constructor_map_root,
            inputs.list_root,
            inputs.constructor_list_root,
            inputs.enum_cell_root,
            inputs.tiny_root,
            inputs.nested_map_shape_root,
            inputs.list_shapes_root,
            inputs.merkle_root,
            inputs.historic_merkle_root,
            inputs.vector_key_root,
            inputs.counter_parameter_root,
            inputs.composite_key_root,
            inputs.chunked_set_root,
            inputs.chunked_list_root,
            inputs.chunked_map_root,
            inputs.chunked_cell_root,
            inputs.uints_root,
            inputs.wide_root,
        ]
    }

    #[test]
    fn missing_required_roots_keep_usage_and_stop_at_the_missing_argument() {
        let mut untouched = [OsString::from("unconsumed")].into_iter();
        assert_eq!(
            PositionalInputs::parse(None, &mut untouched)
                .unwrap_err()
                .to_string(),
            MISSING_ROOT_USAGE
        );
        assert_eq!(untouched.next(), Some(OsString::from("unconsumed")));

        for available in 0..2 {
            let mut calls = 0;
            let mut arguments =
                (0..available).map(|index| OsString::from(format!("required-{index}")));
            let mut counted = std::iter::from_fn(|| {
                calls += 1;
                arguments.next()
            });
            let error =
                PositionalInputs::parse(Some(OsString::from("counter")), &mut counted).unwrap_err();
            assert_eq!(error.to_string(), MISSING_ROOT_USAGE);
            assert_eq!(calls, available + 1);
        }
    }

    #[test]
    fn every_optional_prefix_retains_root_positions_and_lookahead_consumption() {
        for optional_count in 0..=24 {
            let input: Vec<_> = (0..3 + optional_count)
                .map(|index| OsString::from(format!("root-{index}")))
                .collect();
            let mut expected: Vec<_> = input.iter().cloned().map(Some).collect();
            expected.resize(27, None);
            let mut arguments = input.into_iter();
            let first = arguments.next();
            let mut calls = 0;
            let mut counted = std::iter::from_fn(|| {
                calls += 1;
                arguments.next()
            });
            let inputs = PositionalInputs::parse(first, &mut counted).unwrap();
            assert_eq!(
                roots_in_cli_order(inputs),
                expected,
                "optional prefix {optional_count}"
            );
            // Two required roots, 24 optional reads, one extra-argument check.
            assert_eq!(calls, 27, "optional prefix {optional_count}");
        }
    }

    #[test]
    fn excess_argument_uses_full_usage_and_leaves_later_arguments_unconsumed() {
        let mut arguments = (1..30).map(|index| OsString::from(format!("root-{index}")));
        let error =
            PositionalInputs::parse(Some(OsString::from("root-0")), &mut arguments).unwrap_err();
        assert_eq!(error.to_string(), EXTRA_ROOT_USAGE);
        assert_eq!(arguments.next(), Some(OsString::from("root-28")));
        assert_eq!(arguments.next(), Some(OsString::from("root-29")));
    }

    #[test]
    fn paths_are_neither_normalized_nor_reinterpreted_as_selectors() {
        for first in ["", "--unknown-selector", "counter dir/../counter", "λ"] {
            let roots = [first, "", "--cell-read", "witness path/../witness"].map(OsString::from);
            let mut arguments = roots.clone().into_iter();
            let first = arguments.next();
            let inputs = PositionalInputs::parse(first, &mut arguments).unwrap();
            assert_eq!(inputs.counter_root, roots[0]);
            assert_eq!(inputs.cell_root, roots[1]);
            assert_eq!(inputs.cell_read_root, roots[2]);
            assert_eq!(inputs.witness_root, Some(roots[3].clone()));
        }
    }

    #[cfg(unix)]
    #[test]
    fn non_utf8_roots_are_preserved_in_all_required_and_optional_positions() {
        use std::os::unix::ffi::OsStringExt;

        let roots: Vec<_> = (0..27)
            .map(|index| OsString::from_vec(vec![b'r', index, 0xff]))
            .collect();
        let expected: Vec<_> = roots.iter().cloned().map(Some).collect();
        let mut arguments = roots.into_iter();
        let first = arguments.next();
        let inputs = PositionalInputs::parse(first, &mut arguments).unwrap();
        assert_eq!(roots_in_cli_order(inputs), expected);
    }
}
