// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Conversion syntax from checked facts and one already checked child.
//! No source Type pair, support predicate or numeric parsing enters formatting.
use super::leaf::NativeLeafSink;
use crate::{
    pure_semantics::{conversion::ConversionFacts, sink::ConversionSink},
    value_lowering::{coercion, field_to_bytes_32_syntax},
};

impl ConversionSink for NativeLeafSink {
    fn conversion(&mut self, facts: ConversionFacts, child: syn::Expr) -> syn::Expr {
        match facts {
            ConversionFacts::UintToField { wide: false } => {
                syn::parse_quote!(runtime::Field::from((#child).value()))
            }
            ConversionFacts::UintToField { wide: true } => syn::parse_quote!((#child).as_field()),
            ConversionFacts::FieldToBytes32 => field_to_bytes_32_syntax(child),
            ConversionFacts::Coercion(plan) => coercion::render(&plan, child, 0),
            ConversionFacts::ExplicitUnsignedCast(cast) => {
                coercion::render_unsigned_cast(child, cast)
            }
        }
    }
}
