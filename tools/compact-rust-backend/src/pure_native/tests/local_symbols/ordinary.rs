// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;

#[test]
fn ordinary_names_same_key_shadows_and_unit_views_match_frozen_output() {
    for source in [
        lexical(vec![binding("x", Type::Field, field("1"))], reference("x")),
        lexical(
            vec![binding("x_0", Type::Field, field("1"))],
            reference("x_0"),
        ),
        lexical(
            vec![
                binding("x", Type::Field, field("1")),
                binding("x", Type::Field, reference("x")),
            ],
            reference("x"),
        ),
        lexical(
            vec![binding("flag", Type::Boolean, boolean(true))],
            assertion(reference("flag")),
        ),
    ] {
        assert_eq!(
            observe(&source, &HashMap::new(), &HashMap::new()),
            old_oracle::observe(&source, &HashMap::new(), &HashMap::new())
        );
    }
    let outer = Type::Boolean;
    let parameters = HashMap::from([("x", (&outer, syn::parse_quote!(outer)))]);
    let source = Expr::Tuple {
        elements: vec![
            lexical(vec![binding("x", Type::Field, field("1"))], reference("x")),
            reference("x"),
        ],
    };
    assert_eq!(
        observe(&source, &parameters, &HashMap::new()),
        old_oracle::observe(&source, &parameters, &HashMap::new())
    );
}
