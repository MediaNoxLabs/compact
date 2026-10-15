// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//  	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Exact signed 05217556 conversion and explicit-cast bodies. Test only.

use crate::pure_semantics::numeric::{UnsignedCastStrategy, checked_unsigned_cast};
use crate::value_lowering::{UnsignedMaximum, unsigned_maximum, wide_uint_type};
use crate::{RenderError, ir::Type};
use proc_macro2::Span;

pub(crate) fn unsigned_cast_syntax(
    value: syn::Expr,
    source_max: &str,
    target_max: &str,
) -> Result<syn::Expr, RenderError> {
    Ok(match checked_unsigned_cast(source_max, target_max)? {
        UnsignedCastStrategy::SmallToSmall { source, target } => {
            let source = syn::LitInt::new(&source.to_string(), Span::call_site());
            let target = syn::LitInt::new(&target.to_string(), Span::call_site());
            syn::parse_quote!(runtime::cast_unsigned::<#source, #target>(#value)?)
        }
        UnsignedCastStrategy::SmallToWide { high, low } => {
            let target = wide_uint_type(high, low);
            syn::parse_quote!(<#target>::from_le_bytes(&(#value).value().to_le_bytes())?)
        }
        UnsignedCastStrategy::WideToSmall { high, low, target } => {
            let target = syn::LitInt::new(&target.to_string(), Span::call_site());
            let high = syn::LitInt::new(&high.to_string(), Span::call_site());
            let low = syn::LitInt::new(&low.to_string(), Span::call_site());
            syn::parse_quote!(runtime::narrow_wide_uint::<#target, #high, #low>(#value)?)
        }
        UnsignedCastStrategy::WideToWide { high, low } => {
            let target = wide_uint_type(high, low);
            syn::parse_quote!(<#target>::from_le_bytes((#value).as_le_bytes())?)
        }
    })
}

/// Whether conversion syntax needs an enclosing Result owner, including checked
/// operations whose valid input bounds guarantee success. The source expression
/// is evaluated outside element closures and is not included in this property.
struct Coercion {
    expression: syn::Expr,
    may_fail: bool,
}

impl Coercion {
    fn infallible(expression: syn::Expr) -> Self {
        Self {
            expression,
            may_fail: false,
        }
    }
}

struct AggregateCoercion {
    items: Vec<syn::Ident>,
    expressions: Vec<syn::Expr>,
    may_fail: bool,
}

fn coerce_aggregate_fields(
    sources: &[Type],
    targets: &[Type],
    depth: usize,
) -> Result<AggregateCoercion, RenderError> {
    debug_assert_eq!(sources.len(), targets.len());
    let items = (0..sources.len())
        .map(|index| {
            syn::Ident::new(
                &format!("__compact_cast_item_{depth}_{index}"),
                Span::call_site(),
            )
        })
        .collect::<Vec<_>>();
    let mapped = items
        .iter()
        .zip(sources.iter().zip(targets))
        .map(|(item, (source_ty, target_ty))| {
            coercion(syn::parse_quote!(#item), source_ty, target_ty, depth + 1)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let may_fail = mapped.iter().any(|child| child.may_fail);
    Ok(AggregateCoercion {
        items,
        expressions: mapped.into_iter().map(|child| child.expression).collect(),
        may_fail,
    })
}

pub(crate) fn coerce_expression(
    value: syn::Expr,
    actual: &Type,
    target: &Type,
    depth: usize,
) -> Result<syn::Expr, RenderError> {
    Ok(coercion(value, actual, target, depth)?.expression)
}

fn coercion(
    value: syn::Expr,
    actual: &Type,
    target: &Type,
    depth: usize,
) -> Result<Coercion, RenderError> {
    if actual == target {
        return Ok(Coercion::infallible(value));
    }
    match (actual, target) {
        (Type::Unsigned { max }, Type::Field) => match unsigned_maximum(max)? {
            UnsignedMaximum::Small(_) => Ok(Coercion::infallible(
                syn::parse_quote!(runtime::Field::from((#value).value())),
            )),
            UnsignedMaximum::Wide { .. } => {
                Ok(Coercion::infallible(syn::parse_quote!((#value).as_field())))
            }
        },
        (Type::Unsigned { max: source_max }, Type::Unsigned { max: target_max }) => {
            let bounds = |value: UnsignedMaximum| match value {
                UnsignedMaximum::Small(low) => (0, low),
                UnsignedMaximum::Wide { high, low } => (high, low),
            };
            if bounds(unsigned_maximum(source_max)?) > bounds(unsigned_maximum(target_max)?) {
                return Err(RenderError::TypeMismatch {
                    expected: target.clone(),
                    actual: actual.clone(),
                });
            }
            Ok(Coercion {
                expression: unsigned_cast_syntax(value, source_max, target_max)?,
                may_fail: true,
            })
        }
        (
            Type::Vector {
                element: source_element,
                length: source_length,
            },
            Type::Vector {
                element: target_element,
                length: target_length,
            },
        ) if source_length == target_length => {
            let source =
                syn::Ident::new(&format!("__compact_cast_source_{depth}"), Span::call_site());
            let item = syn::Ident::new(&format!("__compact_cast_item_{depth}"), Span::call_site());
            let Coercion {
                expression: mapped,
                may_fail,
            } = coercion(
                syn::parse_quote!(#item),
                source_element,
                target_element,
                depth + 1,
            )?;
            let expression = if may_fail {
                // A Result-returning conversion must run in the enclosing owner,
                // not an infallible array.map closure. Keep one loop per vector
                // level: unrolling would multiply output by nested dimensions.
                let converted =
                    syn::Ident::new(&format!("__compact_cast_values_{depth}"), Span::call_site());
                let length = syn::LitInt::new(&source_length.to_string(), Span::call_site());
                syn::parse_quote!({
                    let #source = #value;
                    let mut #converted = ::std::vec::Vec::with_capacity(#length);
                    for #item in #source.into_array() {
                        #converted.push(#mapped);
                    }
                    runtime::FixedVector::new(<[_; #length]>::try_from(#converted)
                        .expect("Vector coercion preserves its length"))
                })
            } else {
                syn::parse_quote!({
                    let #source = #value;
                    runtime::FixedVector::new(#source.into_array().map(|#item| #mapped))
                })
            };
            Ok(Coercion {
                expression,
                may_fail,
            })
        }
        (Type::Tuple { elements: sources }, Type::Tuple { elements: targets })
            if sources.len() == targets.len() && !sources.is_empty() =>
        {
            let source =
                syn::Ident::new(&format!("__compact_cast_source_{depth}"), Span::call_site());
            let AggregateCoercion {
                items,
                expressions: mapped,
                may_fail,
            } = coerce_aggregate_fields(sources, targets, depth)?;
            Ok(Coercion {
                may_fail,
                expression: syn::parse_quote!({
                    let #source = #value;
                    let (#(#items),*,) = #source;
                    (#(#mapped),*,)
                }),
            })
        }
        (
            Type::Tuple { elements: sources },
            Type::Vector {
                element: target_element,
                length,
            },
        ) if sources.len() == *length && !sources.is_empty() => {
            let source =
                syn::Ident::new(&format!("__compact_cast_source_{depth}"), Span::call_site());
            let targets = vec![*target_element.clone(); sources.len()];
            let AggregateCoercion {
                items,
                expressions: mapped,
                may_fail,
            } = coerce_aggregate_fields(sources, &targets, depth)?;
            Ok(Coercion {
                may_fail,
                expression: syn::parse_quote!({
                    let #source = #value;
                    let (#(#items),*,) = #source;
                    runtime::FixedVector::new([#(#mapped),*])
                }),
            })
        }
        (
            Type::Vector {
                element: source_element,
                length,
            },
            Type::Tuple { elements: targets },
        ) if *length == targets.len() && !targets.is_empty() => {
            let source =
                syn::Ident::new(&format!("__compact_cast_source_{depth}"), Span::call_site());
            let sources = vec![*source_element.clone(); targets.len()];
            let AggregateCoercion {
                items,
                expressions: mapped,
                may_fail,
            } = coerce_aggregate_fields(&sources, targets, depth)?;
            Ok(Coercion {
                may_fail,
                expression: syn::parse_quote!({
                    let #source = #value;
                    let [#(#items),*] = #source.into_array();
                    (#(#mapped),*,)
                }),
            })
        }
        _ => Err(RenderError::TypeMismatch {
            expected: target.clone(),
            actual: actual.clone(),
        }),
    }
}

pub(super) fn observed_coercion(
    value: syn::Expr,
    actual: &Type,
    target: &Type,
    depth: usize,
) -> Result<(syn::Expr, bool), RenderError> {
    coercion(value, actual, target, depth).map(|value| (value.expression, value.may_fail))
}
