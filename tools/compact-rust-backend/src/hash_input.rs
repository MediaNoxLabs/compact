// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Select exact opaque hash encoding without reconstructing source aggregates.
use crate::ir::Type;

fn contains_opaque(ty: &Type) -> bool {
    match ty {
        Type::OpaqueString | Type::OpaqueBytes => true,
        Type::Struct { fields, .. } => fields.iter().any(|field| contains_opaque(&field.ty)),
        Type::Tuple { elements } => elements.iter().any(contains_opaque),
        Type::Vector { element, .. } => contains_opaque(element),
        _ => false,
    }
}

/// Receives an already-lowered operand once. Only qualified dynamic hash call
/// sites use this helper; ordinary Value conversion and fixed inputs stay intact.
pub(crate) fn syntax(value: syn::Expr, ty: &Type) -> syn::Expr {
    if contains_opaque(ty) {
        syn::parse_quote!(runtime::exact_hash_input(#value))
    } else {
        value
    }
}

/// Private transcript encoding has its own boundary; preserving an opaque
/// preimage here does not relax its later preparation/serialization policy.
pub(crate) fn private_output(value: syn::Expr, ty: &Type) -> syn::Expr {
    if contains_opaque(ty) {
        syn::parse_quote!(runtime::exact_private_output(#value))
    } else {
        syn::parse_quote!(runtime::fab::AlignedValue::from(#value))
    }
}

pub(crate) fn witness_method(ty: &Type) -> syn::Ident {
    if contains_opaque(ty) {
        syn::parse_quote!(try_witness_metered_exact)
    } else {
        syn::parse_quote!(try_witness_metered)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;

    #[test]
    fn wraps_nested_opaque_input_once_without_rebuilding_its_expression() {
        let ty = Type::Struct {
            name: "Envelope".into(),
            fields: vec![crate::ir::StructField {
                name: "items".into(),
                ty: Type::Vector {
                    length: 2,
                    element: Box::new(Type::Tuple {
                        elements: vec![Type::Field, Type::OpaqueString],
                    }),
                },
            }],
        };
        let value = syntax(syn::parse_quote!(read_once()), &ty);
        assert_eq!(
            quote!(#value).to_string(),
            "runtime :: exact_hash_input (read_once ())"
        );
    }

    #[test]
    fn fixed_inputs_keep_the_original_ast() {
        let value: syn::Expr = syn::parse_quote!(read_once());
        for ty in [
            Type::Bytes { length: 32 },
            Type::Field,
            Type::Tuple {
                elements: vec![Type::Field; 11],
            },
        ] {
            let output = syntax(value.clone(), &ty);
            assert_eq!(quote!(#output).to_string(), quote!(#value).to_string());
        }
    }

    #[test]
    fn pure_emission_wraps_only_the_three_qualified_operations() {
        use crate::ir::Expr;
        use std::collections::HashMap;
        let ty = Type::OpaqueString;
        let params = HashMap::from([("text", (&ty, syn::parse_quote!(text)))]);
        let input = || {
            Box::new(Expr::Parameter {
                name: "text".into(),
            })
        };
        for (expression, wrapped) in [
            (Expr::TransientHash { value: input() }, true),
            (Expr::HashToCurve { value: input() }, true),
            (
                Expr::TransientCommit {
                    value: input(),
                    opening: Box::new(Expr::FieldLiteral { value: "7".into() }),
                },
                true,
            ),
            (Expr::PersistentHash { value: input() }, false),
            (Expr::Keccak256 { value: input() }, false),
        ] {
            let (value, _) =
                crate::pure_crypto_expression(&expression, &params, &HashMap::new()).unwrap();
            assert_eq!(
                quote!(#value).to_string().contains("exact_hash_input"),
                wrapped
            );
        }
    }

    #[test]
    fn genuine_opaque_contract_wraps_native_and_recorded_hashes() {
        use syn::visit_mut::{self, VisitMut};
        #[derive(Default)]
        struct HashCalls(Vec<bool>);
        impl VisitMut for HashCalls {
            fn visit_expr_call_mut(&mut self, call: &mut syn::ExprCall) {
                if let syn::Expr::Path(path) = call.func.as_ref()
                    && path.path.segments.last().is_some_and(|segment| {
                        matches!(
                            segment.ident.to_string().as_str(),
                            "hash_to_curve" | "transient_hash" | "transient_commit"
                        )
                    })
                {
                    let wrapped = matches!(call.args.first(), Some(syn::Expr::Call(argument))
                            if matches!(argument.func.as_ref(), syn::Expr::Path(wrapper)
                                if wrapper.path.segments.last().unwrap().ident == "exact_hash_input"));
                    self.0.push(wrapped);
                }
                visit_mut::visit_expr_call_mut(self, call);
            }
        }
        let contract = serde_json::from_str(include_str!(
            "../tests/effect-plan-opaque-aligned-hashes-schema20-ir.json"
        ))
        .unwrap();
        let generated = crate::render_with_capabilities(&contract).unwrap();
        let mut syntax = syn::parse_file(&generated.source).unwrap();
        let mut calls = HashCalls::default();
        calls.visit_file_mut(&mut syntax);
        assert_eq!(calls.0.len(), 12, "six native and six recorded calls");
        assert!(calls.0.into_iter().all(|wrapped| wrapped));
        assert!(generated.source.contains("CompactHashInput"));
    }

    #[test]
    fn witness_encoding_selects_exact_outputs_without_changing_eligibility() {
        for (ty, exact) in [
            (Type::OpaqueBytes, true),
            (Type::OpaqueString, true),
            (Type::Field, false),
        ] {
            let output = private_output(syn::parse_quote!(value.clone()), &ty);
            assert_eq!(
                quote!(#output).to_string().contains("exact_private_output"),
                exact
            );
            assert_eq!(witness_method(&ty) == "try_witness_metered_exact", exact);
        }
        // This syntax helper never admits a witness. The existing checker still
        // rejects string results until the distinct result-domain slice qualifies them.
    }

    #[test]
    fn encoding_projection_preserves_parsed_admission_conditions_and_guards() {
        use syn::visit_mut::{self, VisitMut};
        #[derive(Default)]
        struct Predicates(Vec<String>);
        impl VisitMut for Predicates {
            fn visit_expr_if_mut(&mut self, expression: &mut syn::ExprIf) {
                let condition = &expression.cond;
                self.0.push(quote!(#condition).to_string());
                visit_mut::visit_expr_if_mut(self, expression);
            }
            fn visit_arm_mut(&mut self, arm: &mut syn::Arm) {
                let pattern = &arm.pat;
                self.0.push(quote!(#pattern).to_string());
                if let Some((_, condition)) = &arm.guard {
                    self.0.push(quote!(#condition).to_string());
                }
                visit_mut::visit_arm_mut(self, arm);
            }
        }
        let current = include_str!("recorded/typed_plan/effects.rs");
        let manifest: serde_json::Value = serde_json::from_str(include_str!(
            "../recording_admission_inventory.encoding-583.json"
        ))
        .unwrap();
        let mut previous = current.to_owned();
        for edit in manifest["rendering_replacements"].as_array().unwrap() {
            let from = edit["current"].as_str().unwrap();
            assert_eq!(previous.matches(from).count(), 1);
            previous = previous.replacen(from, edit["previous"].as_str().unwrap(), 1);
        }
        let mut actual = Predicates::default();
        actual.visit_file_mut(&mut syn::parse_file(current).unwrap());
        let mut historical = Predicates::default();
        historical.visit_file_mut(&mut syn::parse_file(&previous).unwrap());
        assert!(actual.0.len() > 5);
        assert_eq!(actual.0, historical.0);
    }
}
