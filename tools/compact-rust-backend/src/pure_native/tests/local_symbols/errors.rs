// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;

#[test]
fn name_initializer_type_and_carrier_first_errors_keep_the_original_positions() {
    let source = lexical(
        vec![binding("bad-name", Type::Field, reference("missing"))],
        Expr::Unit,
    );
    assert_eq!(
        observe(&source, &HashMap::new(), &HashMap::new()).err(),
        Some(RenderError::InvalidIdentifier("bad-name".into()))
    );
    let bad = Type::Unsigned { max: "bad".into() };
    let source = lexical(
        vec![binding("x$0", bad.clone(), reference("missing"))],
        Expr::Unit,
    );
    assert_eq!(
        observe(&source, &HashMap::new(), &HashMap::new()).err(),
        Some(RenderError::UnknownParameter("missing".into()))
    );
    let source = lexical(
        vec![binding("r#value", bad.clone(), boolean(true))],
        Expr::Unit,
    );
    assert_eq!(
        observe(&source, &HashMap::new(), &HashMap::new()).err(),
        Some(RenderError::TypeMismatch {
            expected: bad.clone(),
            actual: Type::Boolean
        })
    );
    let parameters = HashMap::from([("value", (&bad, syn::parse_quote!(value)))]);
    let source = lexical(
        vec![binding("r#value", bad.clone(), reference("value"))],
        Expr::Unit,
    );
    assert_eq!(
        observe(&source, &parameters, &HashMap::new()).err(),
        Some(RenderError::InvalidUnsignedMaximum("bad".into()))
    );
}
