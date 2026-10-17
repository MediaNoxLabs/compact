// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use midnight_compact_runtime::FixedVector;
use std::{cell::RefCell, rc::Rc};

#[test]
fn consumes_owned_elements_in_order_without_extra_trait_bounds() {
    struct Input(String);
    struct Output(String);
    struct Failure;
    let mut observed = Vec::new();
    let mapped: Result<FixedVector<Output, 3>, Failure> = FixedVector::new([
        Input("first".into()),
        Input("second".into()),
        Input("third".into()),
    ])
    .try_map(|Input(value)| {
        observed.push(value.clone());
        Ok(Output(value))
    });
    let Ok(mapped) = mapped else {
        panic!("infallible test mapper returned an error");
    };
    assert_eq!(observed, ["first", "second", "third"]);
    assert_eq!(
        mapped.into_array().map(|Output(value)| value),
        observed.as_slice()
    );
}

#[test]
fn zero_length_never_calls_mapper() {
    let input: FixedVector<String, 0> = FixedVector::new([]);
    let mapped: Result<FixedVector<String, 0>, ()> = input.try_map(|_| {
        panic!("empty vector must not evaluate its mapper");
    });
    assert_eq!(mapped, Ok(FixedVector::new([])));
}

struct Tracked {
    id: usize,
    dropped: Rc<RefCell<Vec<usize>>>,
}

impl Drop for Tracked {
    fn drop(&mut self) {
        self.dropped.borrow_mut().push(self.id);
    }
}

fn inputs(dropped: &Rc<RefCell<Vec<usize>>>) -> FixedVector<Tracked, 4> {
    FixedVector::new(std::array::from_fn(|id| Tracked {
        id,
        dropped: Rc::clone(dropped),
    }))
}

#[test]
fn first_error_stops_mapping_and_drops_inputs_and_completed_outputs_once() {
    let input_drops = Rc::new(RefCell::new(Vec::new()));
    let output_drops = Rc::new(RefCell::new(Vec::new()));
    let mut visited = Vec::new();
    let error = Rc::new(());
    let result = inputs(&input_drops).try_map(|input| {
        visited.push(input.id);
        if input.id == 2 {
            return Err(Rc::clone(&error));
        }
        Ok(Tracked {
            id: input.id,
            dropped: Rc::clone(&output_drops),
        })
    });
    let Err(actual) = result else {
        panic!("expected the mapper's first error");
    };
    assert!(Rc::ptr_eq(&actual, &error));
    assert_eq!(visited, [0, 1, 2]);
    input_drops.borrow_mut().sort_unstable();
    output_drops.borrow_mut().sort_unstable();
    assert_eq!(*input_drops.borrow(), [0, 1, 2, 3]);
    assert_eq!(*output_drops.borrow(), [0, 1]);
}

#[test]
fn mapper_panic_releases_remaining_inputs_and_completed_outputs() {
    let input_drops = Rc::new(RefCell::new(Vec::new()));
    let output_drops = Rc::new(RefCell::new(Vec::new()));
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        inputs(&input_drops).try_map::<_, ()>(|input| {
            assert_ne!(input.id, 2, "test mapper panic");
            Ok(Tracked {
                id: input.id,
                dropped: Rc::clone(&output_drops),
            })
        })
    }));
    assert!(result.is_err());
    input_drops.borrow_mut().sort_unstable();
    output_drops.borrow_mut().sort_unstable();
    assert_eq!(*input_drops.borrow(), [0, 1, 2, 3]);
    assert_eq!(*output_drops.borrow(), [0, 1]);
}
