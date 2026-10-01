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

use midnight_base_crypto::fab::{AlignedValue, Alignment, AlignmentAtom, Value, ValueAtom};
use midnight_compact_runtime::CompactError;
use midnight_compact_runtime::ledger::{
    DefaultDB, StateValue, constructor_cell, constructor_counter, read_cell, read_counter,
};
use midnight_compact_runtime::{BoundedUint, Field, FixedBytes, FixedVector};

#[test]
fn constructor_cell_round_trips_and_checks_alignment() {
    let state = constructor_cell::<_, DefaultDB>([1_u8, 2, 0, 0]);
    assert_eq!(read_cell::<[u8; 4], _>(&state).unwrap(), [1, 2, 0, 0]);
    assert!(matches!(
        read_cell::<u64, _>(&state),
        Err(CompactError::InvalidLedgerCell(_))
    ));
    assert!(matches!(
        read_cell::<bool, _>(&StateValue::<DefaultDB>::Null),
        Err(CompactError::InvalidLedgerCell(_))
    ));
}

#[test]
fn counter_constructor_uses_ledger_uint64_cell_shape() {
    let state = constructor_counter::<DefaultDB>();
    let StateValue::Cell(cell) = &state else {
        panic!("counter must be Cell")
    };
    assert_eq!(
        **cell,
        AlignedValue::new(
            Value(vec![ValueAtom(vec![])]),
            Alignment::singleton(AlignmentAtom::Bytes { length: 8 })
        )
        .unwrap()
    );
    assert_eq!(read_counter(&state).unwrap(), 0);
}

#[test]
fn bounded_uint_cell_rejects_a_value_above_its_declared_maximum() {
    type Small = BoundedUint<8>;
    let cell = constructor_cell::<_, DefaultDB>(Small::new(8).unwrap());
    assert_eq!(read_cell::<Small, _>(&cell).unwrap().value(), 8);
    let wider = constructor_cell::<_, DefaultDB>(9_u8);
    assert!(matches!(
        read_cell::<Small, _>(&wider),
        Err(CompactError::UnsignedOutOfRange { value: 9, max: 8 })
    ));
}

#[test]
fn fixed_bytes_cell_preserves_compact_length_and_normalized_value() {
    let bytes = FixedBytes::<4>::new([1, 2, 0, 0]);
    let cell = constructor_cell::<_, DefaultDB>(bytes);
    assert_eq!(read_cell::<FixedBytes<4>, _>(&cell).unwrap(), bytes);
}

#[test]
fn fixed_vector_cell_round_trips_composite_alignment() {
    let value = FixedVector::new([Field::from(3_u64), Field::from(5_u64)]);
    let state = constructor_cell::<_, DefaultDB>(value.clone());
    assert_eq!(
        read_cell::<FixedVector<Field, 2>, _>(&state).unwrap(),
        value
    );
}

#[test]
fn tuple_and_vector_of_tuples_decode_from_ledger_cells() {
    let pair = (Field::from(9_u64), true);
    let pair_state = constructor_cell::<_, DefaultDB>(pair);
    assert_eq!(read_cell::<(Field, bool), _>(&pair_state).unwrap(), pair);

    let vector = FixedVector::new([pair, (Field::from(10_u64), false)]);
    let state = constructor_cell::<_, DefaultDB>(vector.clone());
    assert_eq!(
        read_cell::<FixedVector<(Field, bool), 2>, _>(&state).unwrap(),
        vector
    );
}
