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
fn corrected_raw_api_names_and_distinct_alias_keys_emit_separate_safe_locals() {
    for (name, expected) in [
        ("x$0", "__compact_local_x_0"),
        ("r#value", "__compact_local_value"),
        ("x /* comment */", "__compact_local_x"),
    ] {
        let source = lexical(
            vec![binding(name, Type::Field, field("1"))],
            reference(name),
        );
        assert_eq!(local_symbols(&source, &HashMap::new()), [expected]);
        assert_eq!(
            observe(&source, &HashMap::new(), &HashMap::new())
                .unwrap()
                .2,
            Type::Field
        );
    }
    let source = lexical(
        vec![
            binding("a$b", Type::Field, field("1")),
            binding("a_b", Type::Field, field("2")),
        ],
        Expr::Tuple {
            elements: vec![reference("a$b"), reference("a_b")],
        },
    );
    assert_eq!(
        local_symbols(&source, &HashMap::new()),
        ["__compact_local_a_b", "__compact_local_a_b_1"]
    );
    let output = observe(&source, &HashMap::new(), &HashMap::new())
        .unwrap()
        .0;
    assert!(
        output.ends_with("(__compact_local_a_b , __compact_local_a_b_1) }"),
        "{output}"
    );
}

#[test]
fn occupied_parameter_and_map_symbols_are_protected_without_changing_their_allocators() {
    let field_ty = Type::Field;
    let parameters =
        HashMap::from([("other", (&field_ty, syn::parse_quote!(r#__compact_local_x)))]);
    let source = lexical(
        vec![
            binding("x", Type::Field, reference("other")),
            binding("x", Type::Field, reference("x")),
        ],
        Expr::Tuple {
            elements: vec![reference("x"), reference("other")],
        },
    );
    assert_eq!(
        local_symbols(&source, &parameters),
        ["__compact_local_x_1", "__compact_local_x_1"]
    );
    let source = Expr::VectorMap {
        parameter: Parameter {
            name: "__compact_local_x".into(),
            ty: Type::Field,
        },
        source: Box::new(Expr::Vector {
            element: Type::Field,
            elements: vec![field("1")],
        }),
        body: Box::new(lexical(
            vec![binding("x", Type::Field, field("2"))],
            Expr::Tuple {
                elements: vec![reference("x"), reference("__compact_local_x")],
            },
        )),
        result: Type::Tuple {
            elements: vec![Type::Field, Type::Field],
        },
        length: 1,
    };
    let output = observe(&source, &HashMap::new(), &HashMap::new())
        .unwrap()
        .0;
    assert!(output.contains("let __compact_local_x_1"), "{output}");
    assert!(output.contains("| __compact_local_x |"), "{output}");
}
