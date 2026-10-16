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

use super::*;

#[test]
fn all_thirteen_old_winning_interfaces_produce_identical_checked_syntax() {
    let bytes = Type::Bytes { length: 32 };
    let product = Type::Struct {
        name: "Payload".into(),
        fields: vec![StructField {
            name: "value".into(),
            ty: Type::Field,
        }],
    };
    let beneficiary = Type::Struct {
        name: "Beneficiary".into(),
        fields: vec![StructField {
            name: "bytes".into(),
            ty: bytes.clone(),
        }],
    };
    let mut zswap = unit(vec![crate::coin_shapes::qualified_coin_type()]);
    // Unit-return would win the earlier general Unit adapter. The old Zswap
    // interface also accepts an expression return; that is a real staged winner.
    zswap.stateful_circuits[0].return_value = StateReturn::Expression { value: Expr::Unit };
    let cases = [
        ("algebra.membership", identity(Type::Field)),
        ("algebra.jubjub-values", identity(Type::JubjubPoint)),
        ("algebra.kernel", unit(vec![bytes.clone()])),
        (
            "algebra.context-query",
            contract(
                vec![],
                bytes.clone(),
                StateReturn::Expression {
                    value: Expr::BytesLiteral { bytes: vec![0; 32] },
                },
            ),
        ),
        ("algebra.composite-values", identity(product.clone())),
        (
            "algebra.shielded-receive",
            unit(vec![crate::coin_shapes::shielded_coin_type()]),
        ),
        (
            "algebra.voting-commit",
            unit(vec![
                Type::Boolean,
                crate::coin_shapes::shielded_coin_type(),
            ]),
        ),
        (
            "algebra.guarded-deposit",
            unit(vec![
                Type::OpaqueString,
                beneficiary,
                crate::coin_shapes::shielded_coin_type(),
            ]),
        ),
        (
            "algebra.terminal-returns",
            contract(
                vec![Type::Boolean],
                Type::Field,
                StateReturn::Expression {
                    value: Expr::FieldLiteral { value: "7".into() },
                },
            ),
        ),
        ("algebra.unit-actions", unit(vec![Type::Field])),
        ("algebra.unit-composition", unit(vec![Type::OpaqueBytes])),
        ("algebra.zswap", zswap),
        ("algebra.ordinary", identity(Type::Bytes { length: 16 })),
    ];
    for (owner, c) in cases {
        let (_, old_trace) = equivalent(&c, "root");
        assert_eq!(
            old_trace[0]["body_owner"], owner,
            "wrong old winner {owner}"
        );
        public(&c, "root"); // Complete native and public API validation, not predicate-only inclusion.
    }
    let overlap = unit(vec![product.clone(), product]);
    let (_, old_trace) = equivalent(&overlap, "root");
    assert_eq!(old_trace[0]["body_owner"], "algebra.membership");
    assert_eq!(old_trace[0]["attempts"].as_array().unwrap().len(), 1);
    public(&overlap, "root");
}

#[test]
fn empty_success_and_effectful_passthrough_never_become_fallback() {
    let c = unit(vec![]);
    let (current, old_trace) = equivalent(&c, "root");
    assert_eq!(old_trace[0]["body_owner"], "algebra.membership");
    // An empty checked Unit plan succeeds without a payload temporary or fallback.
    assert!(current.organizer_steps.as_ref().unwrap().is_empty());
    assert!(
        matches!(current.typed_result.as_ref(), Some(syn::Expr::Tuple(unit)) if unit.elems.is_empty())
    );
    public(&c, "root");

    let context = Context::new(&c);
    for (old, empty_steps) in [(false, true), (true, true), (false, false), (true, false)] {
        let plan = TypedPlan {
            steps: if empty_steps {
                vec![]
            } else {
                vec![syn::parse_quote!(let retained = 7u8;)]
            },
            result: syn::parse_quote!(retained),
        };
        let (selected, rows) = context.selected(&c.stateful_circuits[0], old, Some(plan));
        let selected = selected.unwrap();
        assert_eq!(
            selected.organizer_steps.unwrap().len(),
            usize::from(!empty_steps)
        );
        assert_eq!(
            selected.typed_result.unwrap().to_token_stream().to_string(),
            "retained"
        );
        assert!(rows[0]["attempts"].as_array().unwrap().is_empty());
    }
    // Passthrough intentionally does not revalidate the caller's already checked
    // plan. Actual Effectful/start-funding qualification stays with its caller.
}
