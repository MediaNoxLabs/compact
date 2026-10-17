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
fn nested_prior_families_and_aggregate_scopes_visit_each_source_once() {
    let source = Expr::Tuple {
        elements: vec![
            Expr::StructLiteral {
                ty: structure("Record", vec![("value", Type::Field)]),
                fields: vec![Expr::Add {
                    left: Box::new(Expr::FieldCast {
                        value: Box::new(Expr::UnsignedLiteral {
                            value: "1".into(),
                            max: "255".into(),
                        }),
                    }),
                    right: Box::new(Expr::TransientHash {
                        value: Box::new(field("2")),
                    }),
                }],
            },
            Expr::VectorMap {
                parameter: Parameter {
                    name: "item$raw".into(),
                    ty: Type::Field,
                },
                source: Box::new(vector(vec![field("3")])),
                body: Box::new(Expr::Tuple {
                    elements: vec![
                        reference("item$raw"),
                        Expr::JubjubScalarFromNative {
                            value: Box::new(reference("item$raw")),
                        },
                    ],
                }),
                result: Type::Tuple {
                    elements: vec![Type::Field, Type::Field],
                },
                length: 1,
            },
        ],
    };
    assert!(compare_map_successor(&source, &HashMap::new(), &HashMap::new()).is_ok());
    let (result, visits) =
        visitation::observe(|| observe(&source, &HashMap::new(), &HashMap::new()));
    result.unwrap();
    assert_eq!(visits.len(), 14);
    assert!(visits.values().all(|count| *count == 1));
}

#[test]
fn native_control_nested_in_map_preserves_complete_template_successor_without_a_copied_check_sink()
{
    let source = Expr::VectorMap {
        parameter: Parameter {
            name: "item".into(),
            ty: Type::Field,
        },
        source: Box::new(vector(vec![field("1")])),
        body: Box::new(branch(boolean(true), reference("item"), field("0"))),
        result: Type::Field,
        length: 1,
    };
    assert_eq!(
        compare_map_views(&source, &HashMap::new(), &HashMap::new()),
        Ok(vector_type(Type::Field, 1))
    );
    let (result, visits) =
        visitation::observe(|| observe(&source, &HashMap::new(), &HashMap::new()));
    result.unwrap();
    assert_eq!(visits.len(), 7);
    assert!(visits.values().all(|count| *count == 1));
}
