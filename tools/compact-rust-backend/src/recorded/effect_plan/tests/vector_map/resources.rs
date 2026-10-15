// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Existing compiler/layout bounds; these are not runtime loop-cost estimates.
use super::*;

#[test]
fn vector_map_layout_and_nested_block_boundaries_keep_existing_limits() {
    let slots = crate::resource_limits::recording_fixed_layout_limit();
    let accepted = circuit(slots - 1, parameter("item"));
    checked(&accepted);
    let denied = circuit(slots, parameter("item"));
    let error = check(&denied, &HashMap::new(), &HashMap::new()).unwrap_err();
    assert!(
        matches!(error, PlanError::Unsupported(ref gap) if gap.resource.as_ref().is_some_and(|r| r.limit == slots))
    );

    let checked = checked(&circuit(1, parameter("item")));
    let limit = crate::resource_limits::recording_block_depth_limit();
    for (maps, valid) in [(limit - 1, true), (limit, false)] {
        let mut plan = checked.plan().clone();
        let source = plan.parameters[0].clone();
        // Ignored inner results keep every element/result type flat. This
        // isolates block recursion from unrelated recursive type-depth limits.
        let mut nested = None;
        for i in (0..maps).rev() {
            let formal = Value {
                id: 2 + i * 2,
                ty: Type::Field,
            };
            let output = Value {
                id: 3 + i * 2,
                ty: vector(Type::Field, 1),
            };
            let body = Block {
                bindings: nested.into_iter().collect(),
                result: formal.clone(),
            };
            nested = Some(Binding {
                value: output,
                operation: Operation::VectorMap {
                    source: source.clone(),
                    parameter: formal,
                    body,
                },
                path: format!("map[{i}]"),
            });
        }
        let outer = nested.unwrap();
        plan.body = Block {
            result: outer.value.clone(),
            bindings: vec![outer],
        };
        let result = validate::validate(&plan, checked.declarations());
        if valid {
            result.unwrap();
        } else {
            assert_eq!(
                result.unwrap_err().detail,
                "recording plan block depth limit exceeded"
            );
        }
    }
}
