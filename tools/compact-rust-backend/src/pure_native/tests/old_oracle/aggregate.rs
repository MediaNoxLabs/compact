// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

fn pure_aggregate_expression(
    expr: &Expr,
    parameters: &HashMap<&str, (&Type, syn::Ident)>,
    circuits: &HashMap<&str, &PureCircuit>,
) -> Result<(syn::Expr, Type), RenderError> {
    match expr {
        Expr::StructField {
            value,
            field,
            index,
        } => {
            let (value, ty) = expression_with_calls(value, parameters, circuits)?;
            let Type::Struct { fields, .. } = ty else {
                return Err(RenderError::InvalidStructField(field.clone()));
            };
            let declaration = fields
                .get(*index)
                .filter(|declaration| declaration.name == *field)
                .ok_or_else(|| RenderError::InvalidStructField(field.clone()))?;
            let name = ident(field)?;
            let field_value = retained_value(syn::parse_quote!((#value).#name), &declaration.ty);
            Ok((field_value, declaration.ty.clone()))
        }
        Expr::TupleIndex { value, index } => {
            let (value, ty) = expression_with_calls(value, parameters, circuits)?;
            let Type::Tuple { elements } = ty else {
                return Err(RenderError::ExpectedTuple(ty));
            };
            let result = elements
                .get(*index)
                .ok_or(RenderError::InvalidTupleIndex(*index))?
                .clone();
            let index = syn::Index::from(*index);
            Ok((syn::parse_quote!((#value).#index), result))
        }
        Expr::StructLiteral { ty, fields } => {
            let Type::Struct {
                name,
                fields: declarations,
            } = ty
            else {
                return Err(RenderError::InvalidStructField("<literal>".into()));
            };
            if fields.len() != declarations.len() {
                return Err(RenderError::InvalidStructField(name.clone()));
            }
            let mut field_values = Vec::<syn::FieldValue>::with_capacity(fields.len());
            for (value, declaration) in fields.iter().zip(declarations) {
                let (rendered, actual) = expression_with_calls(value, parameters, circuits)?;
                if actual != declaration.ty {
                    return Err(RenderError::TypeMismatch {
                        expected: declaration.ty.clone(),
                        actual,
                    });
                }
                let field_name = ident(&declaration.name)?;
                let shorthand = matches!(
                    &rendered,
                    syn::Expr::Path(path)
                        if path.qself.is_none()
                            && path.path.leading_colon.is_none()
                            && path.path.segments.len() == 1
                            && path.path.segments[0].ident == field_name
                            && matches!(path.path.segments[0].arguments, syn::PathArguments::None)
                );
                field_values.push(if shorthand {
                    syn::parse_quote!(#field_name)
                } else {
                    syn::parse_quote!(#field_name: #rendered)
                });
            }
            let name = ident(name)?;
            Ok((
                syn::parse_quote!(crate::types::#name { #(#field_values),* }),
                ty.clone(),
            ))
        }
        Expr::Tuple { elements } => {
            let (exprs, types): (Vec<_>, Vec<_>) = elements
                .iter()
                .map(|element| expression_with_calls(element, parameters, circuits))
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .unzip();
            let mut tuple = syn::ExprTuple {
                attrs: vec![],
                paren_token: syn::token::Paren::default(),
                elems: syn::punctuated::Punctuated::new(),
            };
            for expr in exprs {
                tuple.elems.push_value(expr);
                tuple.elems.push_punct(syn::token::Comma::default());
            }
            if tuple.elems.len() > 1 {
                tuple.elems.pop_punct();
            }
            Ok((syn::Expr::Tuple(tuple), Type::Tuple { elements: types }))
        }
        Expr::Vector { element, elements } => {
            let mut values = Vec::new();
            for value in elements {
                let (rendered, actual) = expression_with_calls(value, parameters, circuits)?;
                if actual != *element {
                    return Err(RenderError::TypeMismatch {
                        expected: element.clone(),
                        actual,
                    });
                }
                values.push(rendered);
            }
            let length = elements.len();
            let rendered = if values.is_empty() {
                // An empty literal has no value from which Rust can infer its
                // element type, including inside an ignored map argument.
                let element = rust_type(element)?;
                syn::parse_quote!(runtime::FixedVector::<#element, 0>::new([]))
            } else {
                syn::parse_quote!(runtime::FixedVector::new([#(#values),*]))
            };
            Ok((
                rendered,
                Type::Vector {
                    element: Box::new(element.clone()),
                    length,
                },
            ))
        }
        Expr::VectorMap {
            parameter,
            source,
            body,
            result,
            length,
        } => {
            let (source, source_ty) = expression_with_calls(source, parameters, circuits)?;
            let target_source_ty = Type::Vector {
                element: Box::new(parameter.ty.clone()),
                length: *length,
            };
            let source = coerce_expression(source, &source_ty, &target_source_ty, 0)?;
            let item_name = ident(&parameter.name)?;
            let mut body_parameters = parameters.clone();
            body_parameters.insert(parameter.name.as_str(), (&parameter.ty, item_name.clone()));
            let (body, actual) = expression_with_calls(body, &body_parameters, circuits)?;
            if actual != *result {
                return Err(RenderError::TypeMismatch {
                    expected: result.clone(),
                    actual,
                });
            }
            let mapped = syn::Ident::new("__compact_mapped", Span::call_site());
            let source_name = syn::Ident::new("__compact_map_source", Span::call_site());
            let length_lit = syn::LitInt::new(&length.to_string(), Span::call_site());
            Ok((
                syn::parse_quote!({
                    let #source_name = #source;
                    let mut #mapped = Vec::with_capacity(#length_lit);
                    for #item_name in #source_name.into_array() {
                        #mapped.push(#body);
                    }
                    runtime::FixedVector::new(<[_; #length_lit]>::try_from(#mapped).expect("Vector map preserves its length"))
                }),
                Type::Vector {
                    element: Box::new(result.clone()),
                    length: *length,
                },
            ))
        }
        Expr::VectorFoldCall {
            name,
            initial,
            source,
            accumulator,
            element,
            length,
        } => {
            let circuit = circuits
                .get(name.as_str())
                .ok_or_else(|| RenderError::UnknownCircuit(name.clone()))?;
            if circuit.parameters.len() != 2
                || circuit.parameters[0].ty != *accumulator
                || circuit.parameters[1].ty != *element
                || circuit.result != *accumulator
            {
                return Err(RenderError::TypeMismatch {
                    expected: accumulator.clone(),
                    actual: circuit.result.clone(),
                });
            }
            let (initial, initial_ty) = expression_with_calls(initial, parameters, circuits)?;
            let initial = coerce_expression(initial, &initial_ty, accumulator, 0)?;
            let (source, source_ty) = expression_with_calls(source, parameters, circuits)?;
            let expected_source = Type::Vector {
                element: Box::new(element.clone()),
                length: *length,
            };
            let source = coerce_expression(source, &source_ty, &expected_source, 0)?;
            let function = ident(name)?;
            let acc = syn::Ident::new("__compact_fold_accumulator", Span::call_site());
            let item = syn::Ident::new("__compact_fold_item", Span::call_site());
            let source_name = syn::Ident::new("__compact_fold_source", Span::call_site());
            Ok((
                syn::parse_quote!({
                    let #source_name = #source;
                    let mut #acc = #initial;
                    for #item in #source_name.into_array() {
                        #acc = crate::pure_circuits::#function(#acc, #item)?;
                    }
                    #acc
                }),
                accumulator.clone(),
            ))
        }
        Expr::Call { name, arguments } => {
            let circuit = circuits
                .get(name.as_str())
                .ok_or_else(|| RenderError::UnknownCircuit(name.clone()))?;
            if arguments.len() != circuit.parameters.len() {
                return Err(RenderError::ArgumentCount {
                    circuit: name.clone(),
                    expected: circuit.parameters.len(),
                    actual: arguments.len(),
                });
            }
            let mut rendered_arguments = Vec::with_capacity(arguments.len());
            for (argument, parameter) in arguments.iter().zip(&circuit.parameters) {
                let (value, actual) = expression_with_calls(argument, parameters, circuits)?;
                if actual != parameter.ty {
                    return Err(RenderError::TypeMismatch {
                        expected: parameter.ty.clone(),
                        actual,
                    });
                }
                rendered_arguments.push(value);
            }
            let name = ident(name)?;
            Ok((
                syn::parse_quote!(crate::pure_circuits::#name(#(#rendered_arguments),*)?),
                circuit.result.clone(),
            ))
        }
        _ => unreachable!("private pure-expression dispatcher selects this family"),
    }
}
