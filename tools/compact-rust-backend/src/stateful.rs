//! Typed stateful circuit syntax emission.

use proc_macro2::Span;
use std::collections::HashMap;

use crate::ir::{
    CounterAmount, LedgerField, LedgerFieldKind, StateAction, StateReturn, StatefulCircuit, Type,
};
use crate::{RenderError, expression, ident, rust_type};

pub(crate) fn render_stateful_circuit(
    circuit: &StatefulCircuit,
    ledger_fields: &HashMap<&str, &LedgerField>,
) -> Result<syn::Item, RenderError> {
    let name = ident(&circuit.name)?;
    let mut parameters = HashMap::new();
    let mut args = Vec::<syn::FnArg>::new();
    for (index, parameter) in circuit.parameters.iter().enumerate() {
        ident(&parameter.name)?;
        let rust_name = syn::Ident::new(&format!("__compact_param_{index}"), Span::call_site());
        if parameters
            .insert(parameter.name.as_str(), (&parameter.ty, rust_name.clone()))
            .is_some()
        {
            return Err(RenderError::DuplicateParameter(parameter.name.clone()));
        }
        let arg_ty = rust_type(&parameter.ty)?;
        args.push(syn::parse_quote!(#rust_name: #arg_ty));
    }
    let mut statements = Vec::<syn::Stmt>::new();
    for action in &circuit.actions {
        match action {
            StateAction::CounterIncrement {
                field,
                index,
                amount,
            }
            | StateAction::CounterDecrement {
                field,
                index,
                amount,
            } => {
                let declaration = ledger_fields
                    .get(field.as_str())
                    .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
                if declaration.declaration != LedgerFieldKind::Counter
                    || declaration.index != *index
                {
                    return Err(RenderError::UnknownLedgerField(field.clone()));
                }
                let index = syn::LitInt::new(&declaration.index.to_string(), Span::call_site());
                let amount: syn::Expr = match amount {
                    CounterAmount::Literal { value } => {
                        let value = syn::LitInt::new(&value.to_string(), Span::call_site());
                        syn::parse_quote!(#value)
                    }
                    CounterAmount::Parameter { name } => {
                        let (ty, rust_name) = parameters
                            .get(name.as_str())
                            .ok_or_else(|| RenderError::UnknownParameter(name.clone()))?;
                        let expected = Type::Unsigned {
                            max: "65535".into(),
                        };
                        if *ty != &expected {
                            return Err(RenderError::TypeMismatch {
                                expected,
                                actual: (*ty).clone(),
                            });
                        }
                        syn::parse_quote!(#rust_name.value() as u16)
                    }
                };
                let method = if matches!(action, StateAction::CounterIncrement { .. }) {
                    syn::Ident::new("increment_counter", Span::call_site())
                } else {
                    syn::Ident::new("decrement_counter", Span::call_site())
                };
                statements.push(syn::parse_quote! {
                    let step = context.#method(#index, #amount)?;
                });
                statements.push(syn::parse_quote! {
                    let context = step.context;
                });
                statements.push(syn::parse_quote! {
                    total_cost += step.gas_cost;
                });
            }
            StateAction::CounterReset { field, index } => {
                let declaration = ledger_fields
                    .get(field.as_str())
                    .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
                if declaration.declaration != LedgerFieldKind::Counter
                    || declaration.index != *index
                {
                    return Err(RenderError::UnknownLedgerField(field.clone()));
                }
                let index = syn::LitInt::new(&index.to_string(), Span::call_site());
                statements.push(syn::parse_quote! {
                    let step = context.write_cell(#index, 0_u64)?;
                });
                statements.push(syn::parse_quote! {
                    let context = step.context;
                });
                statements.push(syn::parse_quote! {
                    total_cost += step.gas_cost;
                });
            }
            StateAction::CellWrite {
                field,
                index,
                value,
            } => {
                let declaration = ledger_fields
                    .get(field.as_str())
                    .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
                let LedgerFieldKind::Cell { ty } = &declaration.declaration else {
                    return Err(RenderError::UnknownLedgerField(field.clone()));
                };
                if declaration.index != *index {
                    return Err(RenderError::UnknownLedgerField(field.clone()));
                }
                let (value, actual) = expression(value, &parameters)?;
                if &actual != ty {
                    return Err(RenderError::TypeMismatch {
                        expected: ty.clone(),
                        actual,
                    });
                }
                let index = syn::LitInt::new(&index.to_string(), Span::call_site());
                statements.push(syn::parse_quote! {
                    let step = context.write_cell(#index, #value)?;
                });
                statements.push(syn::parse_quote! {
                    let context = step.context;
                });
                statements.push(syn::parse_quote! {
                    total_cost += step.gas_cost;
                });
            }
            StateAction::SetInsert {
                field,
                index,
                value,
            }
            | StateAction::SetRemove {
                field,
                index,
                value,
            } => {
                let declaration = ledger_fields
                    .get(field.as_str())
                    .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
                let LedgerFieldKind::Set { ty } = &declaration.declaration else {
                    return Err(RenderError::UnknownLedgerField(field.clone()));
                };
                if declaration.index != *index {
                    return Err(RenderError::UnknownLedgerField(field.clone()));
                }
                let (value, actual) = expression(value, &parameters)?;
                if &actual != ty {
                    return Err(RenderError::TypeMismatch {
                        expected: ty.clone(),
                        actual,
                    });
                }
                let index = syn::LitInt::new(&index.to_string(), Span::call_site());
                let method = if matches!(action, StateAction::SetInsert { .. }) {
                    syn::Ident::new("insert_set", Span::call_site())
                } else {
                    syn::Ident::new("remove_set", Span::call_site())
                };
                statements.push(syn::parse_quote! {
                    let step = context.#method(#index, #value)?;
                });
                statements.push(syn::parse_quote! {
                    let context = step.context;
                });
                statements.push(syn::parse_quote! {
                    total_cost += step.gas_cost;
                });
            }
            StateAction::SetReset { field, index } => {
                let declaration = ledger_fields
                    .get(field.as_str())
                    .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
                if !matches!(declaration.declaration, LedgerFieldKind::Set { .. })
                    || declaration.index != *index
                {
                    return Err(RenderError::UnknownLedgerField(field.clone()));
                }
                let index = syn::LitInt::new(&index.to_string(), Span::call_site());
                statements.push(syn::parse_quote! {
                    let step = context.reset_set(#index)?;
                });
                statements.push(syn::parse_quote! {
                    let context = step.context;
                });
                statements.push(syn::parse_quote! {
                    total_cost += step.gas_cost;
                });
            }
        }
    }
    let result_ty = rust_type(&circuit.result)?;
    let return_expr: syn::Expr = match &circuit.return_value {
        StateReturn::Unit => {
            if circuit.result != Type::Unit {
                return Err(RenderError::TypeMismatch {
                    expected: circuit.result.clone(),
                    actual: Type::Unit,
                });
            }
            syn::parse_quote!(())
        }
        StateReturn::CellRead { field, index } => {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            let LedgerFieldKind::Cell { ty } = &declaration.declaration else {
                return Err(RenderError::UnknownLedgerField(field.clone()));
            };
            if declaration.index != *index {
                return Err(RenderError::UnknownLedgerField(field.clone()));
            }
            if ty != &circuit.result {
                return Err(RenderError::TypeMismatch {
                    expected: circuit.result.clone(),
                    actual: ty.clone(),
                });
            }
            let index = syn::LitInt::new(&index.to_string(), Span::call_site());
            statements.push(syn::parse_quote! {
                let read_step = context.read_cell::<#result_ty>(#index)?;
            });
            statements.push(syn::parse_quote! {
                let context = read_step.context;
            });
            statements.push(syn::parse_quote! {
                total_cost += read_step.gas_cost;
            });
            syn::parse_quote!(read_step.result)
        }
        StateReturn::CounterRead { field, index } => {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            if declaration.declaration != LedgerFieldKind::Counter || declaration.index != *index {
                return Err(RenderError::UnknownLedgerField(field.clone()));
            }
            let expected = Type::Unsigned {
                max: u64::MAX.to_string(),
            };
            if circuit.result != expected {
                return Err(RenderError::TypeMismatch {
                    expected,
                    actual: circuit.result.clone(),
                });
            }
            let index = syn::LitInt::new(&index.to_string(), Span::call_site());
            let max = syn::LitInt::new(&u64::MAX.to_string(), Span::call_site());
            statements.push(syn::parse_quote! {
                let read_step = context.read_cell::<u64>(#index)?;
            });
            statements.push(syn::parse_quote! {
                let context = read_step.context;
            });
            statements.push(syn::parse_quote! {
                total_cost += read_step.gas_cost;
            });
            syn::parse_quote!(runtime::BoundedUint::<#max>::new(read_step.result as u128).expect("ledger Counter fits Uint<64>"))
        }
        StateReturn::SetMember {
            field,
            index,
            value,
        } => {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            let LedgerFieldKind::Set { ty } = &declaration.declaration else {
                return Err(RenderError::UnknownLedgerField(field.clone()));
            };
            if declaration.index != *index {
                return Err(RenderError::UnknownLedgerField(field.clone()));
            }
            if circuit.result != Type::Boolean {
                return Err(RenderError::TypeMismatch {
                    expected: Type::Boolean,
                    actual: circuit.result.clone(),
                });
            }
            let (value, actual) = expression(value, &parameters)?;
            if &actual != ty {
                return Err(RenderError::TypeMismatch {
                    expected: ty.clone(),
                    actual,
                });
            }
            let index = syn::LitInt::new(&index.to_string(), Span::call_site());
            statements.push(syn::parse_quote! {
                let read_step = context.member_set(#index, #value)?;
            });
            statements.push(syn::parse_quote! {
                let context = read_step.context;
            });
            statements.push(syn::parse_quote! {
                total_cost += read_step.gas_cost;
            });
            syn::parse_quote!(read_step.result)
        }
        StateReturn::SetSize { field, index } | StateReturn::SetIsEmpty { field, index } => {
            let declaration = ledger_fields
                .get(field.as_str())
                .ok_or_else(|| RenderError::UnknownLedgerField(field.clone()))?;
            if !matches!(declaration.declaration, LedgerFieldKind::Set { .. })
                || declaration.index != *index
            {
                return Err(RenderError::UnknownLedgerField(field.clone()));
            }
            let is_size = matches!(&circuit.return_value, StateReturn::SetSize { .. });
            let expected = if is_size {
                Type::Unsigned {
                    max: u64::MAX.to_string(),
                }
            } else {
                Type::Boolean
            };
            if circuit.result != expected {
                return Err(RenderError::TypeMismatch {
                    expected,
                    actual: circuit.result.clone(),
                });
            }
            let index = syn::LitInt::new(&index.to_string(), Span::call_site());
            let method = if is_size {
                syn::Ident::new("size_set", Span::call_site())
            } else {
                syn::Ident::new("is_empty_set", Span::call_site())
            };
            statements.push(syn::parse_quote! {
                let read_step = context.#method(#index)?;
            });
            statements.push(syn::parse_quote! {
                let context = read_step.context;
            });
            statements.push(syn::parse_quote! {
                total_cost += read_step.gas_cost;
            });
            if is_size {
                let max = syn::LitInt::new(&u64::MAX.to_string(), Span::call_site());
                syn::parse_quote!(runtime::BoundedUint::<#max>::new(read_step.result as u128).expect("ledger Set size fits Uint<64>"))
            } else {
                syn::parse_quote!(read_step.result)
            }
        }
    };
    let item: syn::Item = syn::parse_quote! {
        pub fn #name<Private>(
            context: runtime::context::CircuitContext<Private>,
            #(#args),*
        ) -> Result<runtime::context::CircuitResult<Private, #result_ty>, runtime::CompactError> {
            let mut total_cost = runtime::context::RunningCost::default();
            #(#statements)*
            Ok(runtime::context::CircuitResult { context, result: #return_expr, gas_cost: total_cost })
        }
    };
    Ok(item)
}
