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

//! Explicit execution order for the default positional proof suite.
//!
//! Parsing has succeeded before this owner reads handoff settings and seeds its
//! one RNG. Keep the calls in execution order, which differs from input order.

use std::error::Error;
use std::path::Path;

use rand::{SeedableRng, rngs::StdRng};

use super::positional_inputs::PositionalInputs;
use super::{
    HandoffConfig, positional_cells, positional_counters, positional_keys, positional_lists,
    positional_maps, positional_merkle, positional_sets, positional_unsigned, positional_witnesses,
};

pub(super) fn run(inputs: PositionalInputs) -> Result<(), Box<dyn Error>> {
    let PositionalInputs {
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
    } = inputs;
    let counter_root = Path::new(&counter_root);
    let cell_root = Path::new(&cell_root);
    let cell_read_root = Path::new(&cell_read_root);
    let handoff_config = HandoffConfig::from_env()?;
    let mut rng = StdRng::seed_from_u64(0x434f4d50414354);
    positional_counters::check_binding_and_handoff(
        counter_root,
        cell_root,
        &handoff_config,
        &mut rng,
    )?;

    if let Some(root) = counter_parameter_root.as_ref() {
        let root = Path::new(root);
        positional_counters::check_parameterized(root, &mut rng)?;
    }

    positional_cells::check_write_and_read(cell_root, cell_read_root, &mut rng)?;

    if let Some(witness_root) = witness_root {
        let witness_root = Path::new(&witness_root);
        positional_witnesses::check_cell_witnesses(witness_root, &mut rng)?;
    }
    if let Some(nested_root) = nested_root {
        let nested_root = Path::new(&nested_root);
        positional_counters::check_nested_calls(nested_root, &mut rng)?;
    }
    if let Some(expression_root) = expression_root {
        let expression_root = Path::new(&expression_root);
        positional_witnesses::check_nested_expressions(expression_root, &mut rng)?;
    }
    if let Some(set_root) = set_root {
        let set_root = Path::new(&set_root);
        positional_sets::check_operations(set_root, &mut rng)?;
    }
    if let Some(set_oracle_root) = set_oracle_root {
        let set_oracle_root = Path::new(&set_oracle_root);
        positional_sets::check_membership_oracle(set_oracle_root, &mut rng)?;
    }
    if let Some(map_root) = map_root {
        let map_root = Path::new(&map_root);
        positional_maps::check_operations(map_root, &mut rng)?;
    }
    if let Some(constructor_map_root) = constructor_map_root {
        let constructor_map_root = Path::new(&constructor_map_root);
        positional_maps::check_constructor_actions(constructor_map_root, &mut rng)?;
    }
    if let Some(list_root) = list_root {
        let list_root = Path::new(&list_root);
        positional_lists::check_operations(list_root, &mut rng)?;
    }
    if let Some(constructor_list_root) = constructor_list_root {
        let constructor_list_root = Path::new(&constructor_list_root);
        positional_lists::check_constructor_actions(constructor_list_root, &mut rng)?;
    }
    if let Some(list_shapes_root) = list_shapes_root.as_ref().map(Path::new) {
        positional_lists::check_witness_shapes(list_shapes_root, &mut rng)?;
    }
    if let Some(enum_cell_root) = enum_cell_root.as_ref().map(Path::new) {
        positional_cells::check_enum_value(enum_cell_root, &mut rng)?;
    }
    if let Some(tiny_root) = tiny_root.as_ref().map(Path::new) {
        positional_cells::check_optional_value(tiny_root, &mut rng)?;
    }
    if let Some(shape_root) = nested_map_shape_root.as_ref().map(Path::new) {
        positional_maps::check_nested_shape(shape_root, &mut rng)?;
    }
    if let Some(merkle_root) = merkle_root.as_ref().map(Path::new) {
        positional_merkle::check_plain(merkle_root, &mut rng)?;
    }
    if let Some(historic_root) = historic_merkle_root.as_ref().map(Path::new) {
        positional_merkle::check_historic(historic_root, &mut rng)?;
    }
    if let Some(vector_root) = vector_key_root.as_ref().map(Path::new) {
        positional_keys::check_vector(vector_root, &mut rng)?;
    }
    if let Some(composite_root) = composite_key_root.as_ref().map(Path::new) {
        positional_keys::check_composite(composite_root, &mut rng)?;
    }
    if let Some(chunked_root) = chunked_set_root.as_ref().map(Path::new) {
        positional_sets::check_chunked_layout(chunked_root, &mut rng)?;
    }
    if let Some(chunked_root) = chunked_list_root.as_ref().map(Path::new) {
        positional_lists::check_chunked_layout(chunked_root, &mut rng)?;
    }
    if let Some(chunked_root) = chunked_map_root.as_ref().map(Path::new) {
        positional_maps::check_chunked_layout(chunked_root, &mut rng)?;
    }
    if let Some(chunked_root) = chunked_cell_root.as_ref().map(Path::new) {
        positional_cells::check_chunked_layout(chunked_root, &mut rng)?;
    }
    if let Some(root) = uints_root.as_ref().map(Path::new) {
        positional_unsigned::check_bounded(root, &mut rng)?;
    }
    if let Some(root) = wide_root.as_ref().map(Path::new) {
        positional_unsigned::check_wide(root, &mut rng)?;
    }
    Ok(())
}
