// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Aggregate output from checked values, names, ownership and conversion plans.
//! No source Expr/Type pairs or support decisions enter these formatters.
use super::leaf::NativeLeafSink;
use crate::{
    naming::checked_ident,
    pure_semantics::{aggregate::CheckedAggregate, identifier::CheckedName, sink::AggregateSink},
};
mod literal;
mod projection;
mod vector;

impl AggregateSink for NativeLeafSink {
    fn binding(&mut self, name: &CheckedName) -> syn::Ident {
        checked_ident(name)
    }
    fn aggregate(
        &mut self,
        checked: CheckedAggregate<syn::Expr, syn::Ident, syn::Type>,
    ) -> syn::Expr {
        match checked {
            CheckedAggregate::StructField {
                value,
                field,
                ownership,
            } => projection::field(value, field, ownership),
            CheckedAggregate::TupleIndex { value, index } => projection::tuple(value, index),
            CheckedAggregate::StructLiteral { name, fields } => literal::structure(name, fields),
            CheckedAggregate::Tuple(values) => literal::tuple(values),
            CheckedAggregate::Vector(values) => {
                syn::parse_quote!(runtime::FixedVector::new([#(#values),*]))
            }
            CheckedAggregate::EmptyVector(element) => {
                syn::parse_quote!(runtime::FixedVector::<#element, 0>::new([]))
            }
            CheckedAggregate::Map {
                source,
                source_plan,
                binding,
                body,
                length,
            } => vector::map(source, &source_plan, binding, body, length),
            CheckedAggregate::Fold {
                function,
                initial,
                initial_plan,
                source,
                source_plan,
            } => vector::fold(function, initial, &initial_plan, source, &source_plan),
            CheckedAggregate::Call {
                function,
                arguments,
            } => {
                let function = checked_ident(&function);
                syn::parse_quote!(crate::pure_circuits::#function(#(#arguments),*)?)
            }
        }
    }
}
