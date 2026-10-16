// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Authenticated old complete pure-expression and Unit dispatch. Test only.
use crate::ir::{ComparisonOperator, Expr, PureCircuit, Type};
use crate::{
    RenderError, UnsignedMaximum, coerce_expression, condition_needs_statement, copy_type,
    field_literal_bytes, field_to_bytes_32_syntax, hash_input, ident, jubjub_value,
    native_representation, retained_value, rust_type, unsigned_arithmetic_syntax,
    unsigned_cast_syntax, unsigned_maximum, wide_uint_type,
};
use proc_macro2::Span;
use quote::ToTokens;
use std::collections::HashMap;

include!("old_oracle/control.rs");
include!("old_oracle/dispatch.rs");
include!("old_oracle/aggregate.rs");
include!("old_oracle/conversion.rs");
include!("old_oracle/arithmetic.rs");
include!("old_oracle/crypto.rs");
include!("old_oracle/leaf.rs");

pub(super) fn observe(
    expr: &Expr,
    parameters: &HashMap<&str, (&Type, syn::Ident)>,
    circuits: &HashMap<&str, &PureCircuit>,
) -> Result<(String, String, Type), RenderError> {
    let (expression, actual) = expression_with_calls(expr, parameters, circuits)?;
    let statements = if actual == Type::Unit {
        unit_statements(expr, parameters, circuits)?
    } else {
        Vec::new()
    };
    Ok((
        expression.to_token_stream().to_string(),
        quote::quote!(#(#statements)*).to_string(),
        actual,
    ))
}
