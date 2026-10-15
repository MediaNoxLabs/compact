// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Target-edition escaping at actual generated declaration and use owners.

use crate::ir::{Contract, Expr, Parameter, PureCircuit, SCHEMA_VERSION, StructField, Type};
use crate::{RenderError, naming};
use syn::visit_mut::{self, VisitMut};

fn contract(circuits: Vec<PureCircuit>) -> Contract {
    Contract {
        schema_version: SCHEMA_VERSION,
        type_aliases: vec![],
        ledger_fields: vec![],
        constructor: None,
        witnesses: vec![],
        circuits,
        stateful_circuits: vec![],
    }
}

fn circuit(name: &str, parameters: Vec<Parameter>, result: Type, body: Expr) -> PureCircuit {
    PureCircuit {
        source: None,
        name: name.into(),
        internal: false,
        parameters,
        result,
        body,
    }
}

fn parameter(name: &str, ty: Type) -> Parameter {
    Parameter {
        name: name.into(),
        ty,
    }
}

#[derive(Default)]
struct Uses {
    functions: usize,
    bindings: usize,
    calls: usize,
    types: usize,
    fields: usize,
    projections: usize,
    constructors: usize,
}

fn escaped(name: &syn::Ident) -> bool {
    name == "r#gen"
}

impl VisitMut for Uses {
    fn visit_ident_mut(&mut self, name: &mut syn::Ident) {
        assert_ne!(
            name.to_string(),
            "gen",
            "bare reserved target keyword escaped its owner"
        );
    }
    fn visit_item_fn_mut(&mut self, item: &mut syn::ItemFn) {
        self.functions += usize::from(escaped(&item.sig.ident));
        visit_mut::visit_item_fn_mut(self, item);
    }
    fn visit_pat_ident_mut(&mut self, pat: &mut syn::PatIdent) {
        self.bindings += usize::from(escaped(&pat.ident));
        visit_mut::visit_pat_ident_mut(self, pat);
    }
    fn visit_expr_call_mut(&mut self, call: &mut syn::ExprCall) {
        if let syn::Expr::Path(path) = &*call.func {
            self.calls += usize::from(path.path.segments.last().is_some_and(|s| escaped(&s.ident)));
        }
        visit_mut::visit_expr_call_mut(self, call);
    }
    fn visit_type_path_mut(&mut self, path: &mut syn::TypePath) {
        self.types += usize::from(path.path.segments.last().is_some_and(|s| escaped(&s.ident)));
        visit_mut::visit_type_path_mut(self, path);
    }
    fn visit_field_mut(&mut self, field: &mut syn::Field) {
        self.fields += usize::from(field.ident.as_ref().is_some_and(escaped));
        visit_mut::visit_field_mut(self, field);
    }
    fn visit_expr_field_mut(&mut self, field: &mut syn::ExprField) {
        if let syn::Member::Named(name) = &field.member {
            self.projections += usize::from(escaped(name));
        }
        visit_mut::visit_expr_field_mut(self, field);
    }
    fn visit_expr_struct_mut(&mut self, value: &mut syn::ExprStruct) {
        self.constructors += usize::from(
            value
                .path
                .segments
                .last()
                .is_some_and(|s| escaped(&s.ident)),
        );
        visit_mut::visit_expr_struct_mut(self, value);
    }
}

fn uses(contract: &Contract) -> Uses {
    let source = crate::render(contract).unwrap();
    let mut file = syn::parse_file(&source).unwrap();
    let mut uses = Uses::default();
    uses.visit_file_mut(&mut file);
    uses
}

#[test]
fn gen_function_parameter_and_call_site_use_one_target_escape() {
    let contract = contract(vec![
        circuit(
            "gen",
            vec![parameter("gen", Type::Field)],
            Type::Field,
            Expr::Parameter { name: "gen".into() },
        ),
        circuit(
            "invoke",
            vec![parameter("value", Type::Field)],
            Type::Field,
            Expr::Call {
                name: "gen".into(),
                arguments: vec![Expr::Parameter {
                    name: "value".into(),
                }],
            },
        ),
    ]);
    let emitted = uses(&contract);
    assert!(emitted.functions > 0);
    assert!(emitted.bindings > 0);
    assert!(emitted.calls > 0);
}

#[test]
fn gen_named_carrier_field_constructor_and_projection_use_one_target_escape() {
    let product = Type::Struct {
        name: "gen".into(),
        fields: vec![StructField {
            name: "gen".into(),
            ty: Type::Field,
        }],
    };
    let contract = contract(vec![
        circuit(
            "make",
            vec![parameter("value", Type::Field)],
            product.clone(),
            Expr::StructLiteral {
                ty: product.clone(),
                fields: vec![Expr::Parameter {
                    name: "value".into(),
                }],
            },
        ),
        circuit(
            "read",
            vec![parameter("value", product)],
            Type::Field,
            Expr::StructField {
                value: Box::new(Expr::Parameter {
                    name: "value".into(),
                }),
                field: "gen".into(),
                index: 0,
            },
        ),
    ]);
    let emitted = uses(&contract);
    assert!(emitted.types > 0);
    assert!(emitted.fields > 0);
    assert!(emitted.constructors > 0);
    assert!(emitted.projections > 0);
}

#[test]
fn gen_semantic_aliases_keep_collision_and_binding_allocation_behavior() {
    assert_eq!(
        naming::validate_circuit_function_namespace([("gen", None), ("r#gen", None)]),
        Err(RenderError::ConflictingCircuitIdentifier {
            first: "gen".into(),
            second: "r#gen".into(),
            rust_identifier: "gen".into(),
        })
    );
    let allocated = naming::parameter_idents(
        &[
            parameter("gen", Type::Field),
            parameter("r#gen", Type::Field),
        ],
        &[],
    );
    assert_eq!(
        allocated
            .into_iter()
            .map(|name| name.to_string())
            .collect::<Vec<_>>(),
        ["r#gen", "__compact_param_1"]
    );
    for name in ["gen x", "/// doc\ngen", "r##gen", "g en", "r# gen"] {
        assert_eq!(naming::ident(name), super::reference::ident(name));
    }
}
