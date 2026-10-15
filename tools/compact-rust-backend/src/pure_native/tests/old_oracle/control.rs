// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

fn lift_block_condition(
    condition: syn::Expr,
    parameters: &HashMap<&str, (&Type, syn::Ident)>,
) -> (Vec<syn::Stmt>, syn::Expr) {
    fn contains_local(expression: &syn::Expr) -> bool {
        match expression {
            syn::Expr::Block(block) => block
                .block
                .stmts
                .iter()
                .any(|statement| matches!(statement, syn::Stmt::Local(_))),
            syn::Expr::Paren(paren) => contains_local(&paren.expr),
            syn::Expr::Group(group) => contains_local(&group.expr),
            _ => false,
        }
    }
    if !contains_local(&condition) {
        return (Vec::new(), condition);
    }
    let mut suffix = 0;
    let name = loop {
        let candidate = if suffix == 0 {
            "__compact_condition".to_owned()
        } else {
            format!("__compact_condition_{suffix}")
        };
        if parameters.values().all(|(_, name)| *name != candidate) {
            break syn::Ident::new(&candidate, Span::call_site());
        }
        suffix += 1;
    };
    (
        vec![syn::parse_quote!(let #name: bool = #condition;)],
        syn::parse_quote!(#name),
    )
}

fn unit_statements(
    expr: &Expr,
    parameters: &HashMap<&str, (&Type, syn::Ident)>,
    circuits: &HashMap<&str, &PureCircuit>,
) -> Result<Vec<syn::Stmt>, RenderError> {
    match expr {
        Expr::Unit | Expr::Default { ty: Type::Unit } => Ok(Vec::new()),
        Expr::Sequence { steps, value } => {
            let mut statements = Vec::new();
            for step in steps {
                statements.extend(unit_statements(step, parameters, circuits)?);
            }
            statements.extend(unit_statements(value, parameters, circuits)?);
            Ok(statements)
        }
        Expr::Assert { condition, message } => {
            let (condition, actual) = expression_with_calls(condition, parameters, circuits)?;
            if actual != Type::Boolean {
                return Err(RenderError::TypeMismatch {
                    expected: Type::Boolean,
                    actual,
                });
            }
            let (mut statements, condition) = lift_block_condition(condition, parameters);
            statements.push(syn::parse_quote!(if !(#condition) {
                return Err(runtime::CompactError::AssertionFailed(#message.to_owned()));
            }));
            Ok(statements)
        }
        Expr::If {
            condition,
            then,
            otherwise,
        } => {
            let evaluate_condition = condition_needs_statement(condition);
            let (condition, actual) = expression_with_calls(condition, parameters, circuits)?;
            if actual != Type::Boolean {
                return Err(RenderError::TypeMismatch {
                    expected: Type::Boolean,
                    actual,
                });
            }
            let (mut statements, condition) = lift_block_condition(condition, parameters);
            if then == otherwise {
                if evaluate_condition && statements.is_empty() {
                    statements.push(syn::parse_quote!(let _ = #condition;));
                }
                statements.extend(unit_statements(then, parameters, circuits)?);
                return Ok(statements);
            }
            let then = unit_statements(then, parameters, circuits)?;
            let otherwise = unit_statements(otherwise, parameters, circuits)?;
            if otherwise.is_empty()
                && let [syn::Stmt::Expr(syn::Expr::If(inner), _)] = then.as_slice()
                && inner.else_branch.is_none()
            {
                let inner_condition = &inner.cond;
                let inner_body = &inner.then_branch;
                statements
                    .push(syn::parse_quote!(if (#condition) && (#inner_condition) #inner_body));
                return Ok(statements);
            }
            if otherwise.is_empty() && then.is_empty() {
                if statements.is_empty() && evaluate_condition {
                    statements.push(syn::parse_quote!(let _ = #condition;));
                }
            } else if otherwise.is_empty() {
                statements.push(syn::parse_quote!(if #condition { #(#then)* }));
            } else if then.is_empty() {
                statements.push(syn::parse_quote!(if !(#condition) { #(#otherwise)* }));
            } else {
                statements.push(syn::parse_quote!(if #condition {
                    #(#then)*
                } else {
                    #(#otherwise)*
                }));
            }
            Ok(statements)
        }
        Expr::Let { bindings, body } => {
            let mut locals = parameters.clone();
            let mut statements = Vec::<syn::Stmt>::new();
            for binding in bindings {
                ident(&binding.name)?;
                let (value, actual) = expression_with_calls(&binding.value, &locals, circuits)?;
                if actual != binding.ty {
                    return Err(RenderError::TypeMismatch {
                        expected: binding.ty.clone(),
                        actual,
                    });
                }
                let local_ty = rust_type(&binding.ty)?;
                let local_name = syn::Ident::new(
                    &format!("__compact_local_{}", binding.name),
                    Span::call_site(),
                );
                statements.push(syn::parse_quote!(let #local_name: #local_ty = #value;));
                locals.insert(binding.name.as_str(), (&binding.ty, local_name));
            }
            let body = unit_statements(body, &locals, circuits)?;
            if statements.is_empty() {
                Ok(body)
            } else {
                statements.extend(body);
                Ok(vec![syn::parse_quote!({ #(#statements)* })])
            }
        }
        _ => {
            let (rendered, actual) = expression_with_calls(expr, parameters, circuits)?;
            if actual != Type::Unit {
                return Err(RenderError::TypeMismatch {
                    expected: Type::Unit,
                    actual,
                });
            }
            Ok(vec![syn::parse_quote!(#rendered;)])
        }
    }
}

fn pure_control_expression(
    expr: &Expr,
    parameters: &HashMap<&str, (&Type, syn::Ident)>,
    circuits: &HashMap<&str, &PureCircuit>,
) -> Result<(syn::Expr, Type), RenderError> {
    match expr {
        Expr::Assert { condition, message } => {
            let (condition, actual) = expression_with_calls(condition, parameters, circuits)?;
            if actual != Type::Boolean {
                return Err(RenderError::TypeMismatch {
                    expected: Type::Boolean,
                    actual,
                });
            }
            let (prefix, condition) = lift_block_condition(condition, parameters);
            Ok((
                syn::parse_quote!({
                    #(#prefix)*
                    if !(#condition) {
                        return Err(runtime::CompactError::AssertionFailed(#message.to_owned()));
                    }
                }),
                Type::Unit,
            ))
        }
        Expr::Sequence { steps, value } => {
            let mut statements = Vec::<syn::Stmt>::new();
            for step in steps {
                let (_, actual) = expression_with_calls(step, parameters, circuits)?;
                if actual != Type::Unit {
                    return Err(RenderError::TypeMismatch {
                        expected: Type::Unit,
                        actual,
                    });
                }
                statements.extend(unit_statements(step, parameters, circuits)?);
            }
            let (value, ty) = expression_with_calls(value, parameters, circuits)?;
            Ok((syn::parse_quote!({ #(#statements)* #value }), ty))
        }
        Expr::If {
            condition,
            then,
            otherwise,
        } => {
            let evaluate_condition = condition_needs_statement(condition);
            let (condition, condition_ty) = expression_with_calls(condition, parameters, circuits)?;
            if condition_ty != Type::Boolean {
                return Err(RenderError::TypeMismatch {
                    expected: Type::Boolean,
                    actual: condition_ty,
                });
            }
            let (mut prefix, condition) = lift_block_condition(condition, parameters);
            if then == otherwise {
                let (value, ty) = expression_with_calls(then, parameters, circuits)?;
                if evaluate_condition && prefix.is_empty() {
                    prefix.push(syn::parse_quote!(let _ = #condition;));
                }
                return Ok((
                    if prefix.is_empty() {
                        value
                    } else {
                        syn::parse_quote!({ #(#prefix)* #value })
                    },
                    ty,
                ));
            }
            let (then, then_ty) = expression_with_calls(then, parameters, circuits)?;
            let (otherwise, otherwise_ty) = expression_with_calls(otherwise, parameters, circuits)?;
            if then_ty != otherwise_ty {
                return Err(RenderError::TypeMismatch {
                    expected: then_ty,
                    actual: otherwise_ty,
                });
            }
            let conditional: syn::Expr =
                syn::parse_quote!(if #condition { #then } else { #otherwise });
            Ok((
                if prefix.is_empty() {
                    conditional
                } else {
                    syn::parse_quote!({ #(#prefix)* #conditional })
                },
                then_ty,
            ))
        }
        Expr::Let { bindings, body } => {
            let mut locals = parameters.clone();
            let mut statements = Vec::<syn::Stmt>::new();
            for binding in bindings {
                ident(&binding.name)?;
                let (value, actual) = expression_with_calls(&binding.value, &locals, circuits)?;
                if actual != binding.ty {
                    return Err(RenderError::TypeMismatch {
                        expected: binding.ty.clone(),
                        actual,
                    });
                }
                let local_ty = rust_type(&binding.ty)?;
                let local_name = syn::Ident::new(
                    &format!("__compact_local_{}", binding.name),
                    Span::call_site(),
                );
                statements.push(syn::parse_quote!(let #local_name: #local_ty = #value;));
                locals.insert(binding.name.as_str(), (&binding.ty, local_name));
            }
            let (body, body_ty) = expression_with_calls(body, &locals, circuits)?;
            Ok((syn::parse_quote!({ #(#statements)* #body }), body_ty))
        }
        _ => unreachable!("private pure-expression dispatcher selects this family"),
    }
}
