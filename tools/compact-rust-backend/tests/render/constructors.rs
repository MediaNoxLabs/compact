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

//! Constructor parameters, ordered collection steps, and iteration.

use super::support::identity;
use compact_rust_backend::ir::{
    Constructor, ConstructorStep, CounterAmount, Expr, LedgerField, LedgerFieldKind, Parameter,
    Type,
};
use compact_rust_backend::{RenderError, render};

#[test]
fn constructor_cell_parameters_are_typed_and_validated() {
    let mut contract = identity(
        Type::Field,
        Expr::Parameter {
            name: "value".into(),
        },
    );
    contract.ledger_fields = vec![LedgerField {
        source: None,
        id: "stored".into(),
        index: 0,
        path: vec![],
        declaration: LedgerFieldKind::Cell { ty: Type::Field },
    }];
    contract.constructor = Some(Constructor {
        source: None,
        parameters: vec![Parameter {
            name: "seed".into(),
            ty: Type::Field,
        }],
        steps: vec![ConstructorStep::CellWrite {
            field: "stored".into(),
            index: 0,
            value: Expr::Parameter {
                name: "seed".into(),
            },
        }],
    });
    let source = render(&contract).unwrap();
    assert!(source.contains("__compact_constructor_param_0: runtime::Field"));
    assert!(source.contains("let __compact_constructor_value_0 = __compact_constructor_param_0;"));
    assert!(!source.contains("__compact_constructor_value_0.clone()"));
    assert!(
        source.contains(
            "Result<runtime::context::ConstructorResult<Private>, runtime::CompactError>"
        )
    );

    // Constructor input validation is fallible and must propagate the error.
    use syn::visit_mut::VisitMut;
    #[derive(Default)]
    struct CheckedCellCalls(usize);
    impl VisitMut for CheckedCellCalls {
        fn visit_expr_macro_mut(&mut self, expression: &mut syn::ExprMacro) {
            if expression.mac.path.is_ident("vec") {
                use syn::parse::Parser;
                let mut elements =
                    syn::punctuated::Punctuated::<syn::Expr, syn::Token![,]>::parse_terminated
                        .parse2(expression.mac.tokens.clone())
                        .expect("constructor state uses a comma-separated vec expression");
                for element in &mut elements {
                    self.visit_expr_mut(element);
                }
            }
        }
        fn visit_expr_try_mut(&mut self, expression: &mut syn::ExprTry) {
            if let syn::Expr::Call(call) = expression.expr.as_ref()
                && let syn::Expr::Path(path) = call.func.as_ref()
                && path
                    .path
                    .segments
                    .last()
                    .is_some_and(|segment| segment.ident == "try_constructor_cell")
            {
                self.0 += 1;
            }
            syn::visit_mut::visit_expr_try_mut(self, expression);
        }
    }
    let mut syntax = syn::parse_file(&source).unwrap();
    let mut checked = CheckedCellCalls::default();
    checked.visit_file_mut(&mut syntax);
    assert_eq!(
        checked.0, 1,
        "one checked Cell constructor must propagate its Result"
    );
    assert!(!source.contains("::constructor_cell::"));

    let constructor = contract.constructor.as_mut().unwrap();
    let ConstructorStep::CellWrite { index, .. } = &mut constructor.steps[0] else {
        panic!("expected Cell write")
    };
    *index = 1;
    assert_eq!(render(&contract), Err(RenderError::InvalidLedgerIndex(1)));
    let ConstructorStep::CellWrite { index, .. } =
        &mut contract.constructor.as_mut().unwrap().steps[0]
    else {
        panic!("expected Cell write")
    };
    *index = 0;
    contract.constructor.as_mut().unwrap().parameters[0].ty = Type::Boolean;
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Field,
            actual: Type::Boolean
        })
    );
    contract.constructor.as_mut().unwrap().parameters[0].ty = Type::Field;
    let duplicate = contract.constructor.as_ref().unwrap().steps[0].clone();
    contract.constructor.as_mut().unwrap().steps.push(duplicate);
    let source = render(&contract).unwrap();
    assert_eq!(source.matches("context.write_cell(0,").count(), 2);
}

#[test]
fn constructor_counter_steps_use_typed_vm_calls() {
    let mut contract = identity(
        Type::Field,
        Expr::Parameter {
            name: "value".into(),
        },
    );
    contract.ledger_fields = vec![LedgerField {
        source: None,
        id: "count".into(),
        index: 0,
        path: vec![],
        declaration: LedgerFieldKind::Counter,
    }];
    contract.constructor = Some(Constructor {
        source: None,
        parameters: vec![Parameter {
            name: "amount".into(),
            ty: Type::Unsigned {
                max: "65535".into(),
            },
        }],
        steps: vec![
            ConstructorStep::CounterIncrement {
                field: "count".into(),
                index: 0,
                amount: CounterAmount::Parameter {
                    name: "amount".into(),
                },
            },
            ConstructorStep::CounterIncrement {
                field: "count".into(),
                index: 0,
                amount: CounterAmount::Literal { value: 3 },
            },
            ConstructorStep::CounterReset {
                field: "count".into(),
                index: 0,
            },
            ConstructorStep::CounterDecrement {
                field: "count".into(),
                index: 0,
                amount: CounterAmount::Literal { value: 1 },
            },
        ],
    });
    let source = render(&contract).unwrap();
    assert!(source.contains("increment_counter(0, __compact_constructor_param_0.value() as u16)?"));
    assert!(source.contains("context.increment_counter(0, 3u16)?"));
    assert!(source.contains("context.write_cell(0, 0_u64)?"));
    assert!(source.contains("context.decrement_counter(0, 1u16)?"));
    assert!(source.contains("context.into_constructor_result()"));
    contract.constructor.as_mut().unwrap().parameters[0].ty = Type::Field;
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Unsigned {
                max: "65535".into()
            },
            actual: Type::Field,
        })
    );
}

#[test]
fn constructor_for_each_checks_element_type_and_loop_binding() {
    let mut contract = identity(
        Type::Field,
        Expr::Parameter {
            name: "value".into(),
        },
    );
    contract.ledger_fields = vec![LedgerField {
        source: None,
        id: "count".into(),
        index: 0,
        path: vec![],
        declaration: LedgerFieldKind::Counter,
    }];
    contract.constructor = Some(Constructor {
        source: None,
        parameters: vec![],
        steps: vec![ConstructorStep::ForEach {
            binding: Parameter {
                name: "item".into(),
                ty: Type::Unsigned {
                    max: "65535".into(),
                },
            },
            values: vec![Expr::UnsignedLiteral {
                value: "2".into(),
                max: "65535".into(),
            }],
            steps: vec![ConstructorStep::CounterIncrement {
                field: "count".into(),
                index: 0,
                amount: CounterAmount::Parameter {
                    name: "item".into(),
                },
            }],
        }],
    });
    let source = render(&contract).unwrap();
    assert!(source.contains("for __compact_constructor_item_0 in __compact_constructor_values_0"));
    assert!(source.contains("increment_counter(0, __compact_constructor_item_0.value() as u16)?"));
    let ConstructorStep::ForEach { binding, steps, .. } =
        &mut contract.constructor.as_mut().unwrap().steps[0]
    else {
        unreachable!()
    };
    binding.name = "_".into();
    let ConstructorStep::CounterIncrement { amount, .. } = &mut steps[0] else {
        unreachable!()
    };
    *amount = CounterAmount::Parameter { name: "_".into() };
    assert!(
        render(&contract)
            .unwrap()
            .contains("__compact_constructor_item_0")
    );
    let ConstructorStep::ForEach { values, .. } =
        &mut contract.constructor.as_mut().unwrap().steps[0]
    else {
        unreachable!()
    };
    values[0] = Expr::Boolean { value: true };
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Unsigned {
                max: "65535".into()
            },
            actual: Type::Boolean,
        })
    );
}

#[test]
fn constructor_vector_parameter_loop_checks_shape_and_conditional_order() {
    let mut contract = identity(
        Type::Field,
        Expr::Parameter {
            name: "value".into(),
        },
    );
    contract.ledger_fields = vec![LedgerField {
        source: None,
        id: "count".into(),
        index: 0,
        path: vec![],
        declaration: LedgerFieldKind::Counter,
    }];
    contract.constructor = Some(Constructor {
        source: None,
        parameters: vec![Parameter {
            name: "initial".into(),
            ty: Type::Vector {
                element: Box::new(Type::Boolean),
                length: 2,
            },
        }],
        steps: vec![ConstructorStep::ForEachVector {
            binding: Parameter {
                name: "item".into(),
                ty: Type::Boolean,
            },
            source: Expr::Parameter {
                name: "initial".into(),
            },
            length: 2,
            steps: vec![ConstructorStep::If {
                condition: Expr::Parameter {
                    name: "item".into(),
                },
                then_steps: vec![ConstructorStep::CounterIncrement {
                    field: "count".into(),
                    index: 0,
                    amount: CounterAmount::Literal { value: 1 },
                }],
                otherwise_steps: vec![],
            }],
        }],
    });
    let source = render(&contract).unwrap();
    assert!(source.contains("__compact_constructor_param_0"));
    assert!(source.contains(".iter()"));
    assert!(!source.contains(".cloned()"));
    assert!(!source.contains("into_array()"));
    assert!(source.contains("for __compact_constructor_item_0 in"));
    assert!(source.contains("if __compact_constructor_item_0"));
    assert!(source.contains("context.increment_counter(0, 1u16)?"));

    {
        let ConstructorStep::ForEachVector { length, .. } =
            &mut contract.constructor.as_mut().unwrap().steps[0]
        else {
            unreachable!()
        };
        *length = 3;
    }
    assert!(matches!(
        render(&contract),
        Err(RenderError::TypeMismatch { .. })
    ));
    let ConstructorStep::ForEachVector { length, .. } =
        &mut contract.constructor.as_mut().unwrap().steps[0]
    else {
        unreachable!()
    };
    *length = 2;
    let ConstructorStep::ForEachVector { steps, .. } =
        &mut contract.constructor.as_mut().unwrap().steps[0]
    else {
        unreachable!()
    };
    let ConstructorStep::If { condition, .. } = &mut steps[0] else {
        unreachable!()
    };
    *condition = Expr::SetMember {
        field: "count".into(),
        index: 0,
        value: Box::new(Expr::Boolean { value: true }),
    };
    assert_eq!(
        render(&contract),
        Err(RenderError::InvalidConstructorInitializer("if".into()))
    );
}

#[test]
fn constructor_set_steps_validate_values_and_use_vm_methods() {
    let mut contract = identity(
        Type::Field,
        Expr::Parameter {
            name: "value".into(),
        },
    );
    contract.ledger_fields = vec![LedgerField {
        source: None,
        id: "seen".into(),
        index: 0,
        path: vec![],
        declaration: LedgerFieldKind::Set { ty: Type::Boolean },
    }];
    contract.constructor = Some(Constructor {
        source: None,
        parameters: vec![],
        steps: vec![
            ConstructorStep::SetInsert {
                field: "seen".into(),
                index: 0,
                value: Expr::Boolean { value: true },
            },
            ConstructorStep::SetRemove {
                field: "seen".into(),
                index: 0,
                value: Expr::Boolean { value: false },
            },
            ConstructorStep::SetReset {
                field: "seen".into(),
                index: 0,
            },
        ],
    });
    let source = render(&contract).unwrap();
    assert!(source.contains("context.insert_set(0, true)?"));
    assert!(source.contains("context.remove_set(0, false)?"));
    assert!(source.contains("context.reset_set(0)?"));
    let ConstructorStep::SetInsert { value, .. } =
        &mut contract.constructor.as_mut().unwrap().steps[0]
    else {
        unreachable!()
    };
    *value = Expr::FieldLiteral { value: "1".into() };
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Boolean,
            actual: Type::Field,
        })
    );
}

#[test]
fn constructor_list_steps_validate_values_and_use_vm_methods() {
    let mut contract = identity(
        Type::Field,
        Expr::Parameter {
            name: "value".into(),
        },
    );
    contract.ledger_fields = vec![LedgerField {
        source: None,
        id: "items".into(),
        index: 0,
        path: vec![],
        declaration: LedgerFieldKind::List { ty: Type::Field },
    }];
    contract.constructor = Some(Constructor {
        source: None,
        parameters: vec![],
        steps: vec![
            ConstructorStep::ListPushFront {
                field: "items".into(),
                index: 0,
                value: Expr::FieldLiteral { value: "3".into() },
            },
            ConstructorStep::ListPopFront {
                field: "items".into(),
                index: 0,
            },
            ConstructorStep::ListReset {
                field: "items".into(),
                index: 0,
            },
        ],
    });
    let source = render(&contract).unwrap();
    assert!(source.contains("push_front_list(0,"));
    assert!(source.contains("context.pop_front_list(0)?"));
    assert!(source.contains("context.reset_list(0)?"));
    let ConstructorStep::ListPushFront { value, .. } =
        &mut contract.constructor.as_mut().unwrap().steps[0]
    else {
        unreachable!()
    };
    *value = Expr::Boolean { value: true };
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Field,
            actual: Type::Boolean,
        })
    );
}

#[test]
fn constructor_map_steps_validate_keys_values_and_use_vm_methods() {
    let mut contract = identity(
        Type::Field,
        Expr::Parameter {
            name: "value".into(),
        },
    );
    contract.ledger_fields = vec![LedgerField {
        source: None,
        id: "table".into(),
        index: 0,
        path: vec![],
        declaration: LedgerFieldKind::Map {
            key: Type::Boolean,
            value: Type::Field,
        },
    }];
    contract.constructor = Some(Constructor {
        source: None,
        parameters: vec![],
        steps: vec![
            ConstructorStep::MapInsert {
                field: "table".into(),
                index: 0,
                key: Expr::Boolean { value: true },
                value: Expr::FieldLiteral { value: "7".into() },
            },
            ConstructorStep::MapRemove {
                field: "table".into(),
                index: 0,
                key: Expr::Boolean { value: false },
            },
            ConstructorStep::MapReset {
                field: "table".into(),
                index: 0,
            },
            ConstructorStep::MapInsertDefault {
                field: "table".into(),
                index: 0,
                key: Expr::Boolean { value: false },
            },
        ],
    });
    let source = render(&contract).unwrap();
    for method in ["insert_map(0,", "remove_map(0,", "reset_map(0)?"] {
        assert!(source.contains(method), "missing {method}");
    }
    let ConstructorStep::MapInsert { value, .. } =
        &mut contract.constructor.as_mut().unwrap().steps[0]
    else {
        unreachable!()
    };
    *value = Expr::Boolean { value: true };
    assert_eq!(
        render(&contract),
        Err(RenderError::TypeMismatch {
            expected: Type::Field,
            actual: Type::Boolean,
        })
    );
}

#[test]
fn constructor_validates_each_direct_cell_rhs_before_the_next_assignment() {
    use syn::visit_mut::VisitMut;
    #[derive(Default)]
    struct Order(Vec<String>);
    impl VisitMut for Order {
        fn visit_item_impl_mut(&mut self, _: &mut syn::ItemImpl) {
            // Consumer wrapper methods are not the constructor under test.
        }
        fn visit_item_fn_mut(&mut self, item: &mut syn::ItemFn) {
            // Recording may validate the same cell type elsewhere in the module.
            // This contract concerns ordering within the constructor only.
            if item.sig.ident == "initial_state" {
                syn::visit_mut::visit_item_fn_mut(self, item);
            }
        }
        fn visit_local_mut(&mut self, local: &mut syn::Local) {
            if let syn::Pat::Ident(binding) = &local.pat
                && binding
                    .ident
                    .to_string()
                    .starts_with("__compact_constructor_value_")
            {
                self.0.push(format!("bind:{}", binding.ident));
            }
            syn::visit_mut::visit_local_mut(self, local);
        }
        fn visit_expr_call_mut(&mut self, call: &mut syn::ExprCall) {
            if let syn::Expr::Path(path) = call.func.as_ref()
                && path
                    .path
                    .segments
                    .last()
                    .is_some_and(|segment| segment.ident == "validate_cell_value")
                && let Some(syn::Expr::Reference(value)) = call.args.first()
                && let syn::Expr::Path(value) = value.expr.as_ref()
                && let Some(name) = value.path.get_ident()
            {
                self.0.push(format!("validate:{name}"));
            }
            syn::visit_mut::visit_expr_call_mut(self, call);
        }
    }
    let contract =
        serde_json::from_str(include_str!("../opaque-cell-inputs-schema20-ir.json")).unwrap();
    let source = render(&contract).unwrap();
    let mut syntax = syn::parse_file(&source).unwrap();
    let mut order = Order::default();
    order.visit_file_mut(&mut syntax);
    assert_eq!(
        order.0,
        [
            "bind:__compact_constructor_value_0",
            "validate:__compact_constructor_value_0",
            "bind:__compact_constructor_value_1",
            "validate:__compact_constructor_value_1",
            "bind:__compact_constructor_value_2",
            "validate:__compact_constructor_value_2",
        ]
    );
}
