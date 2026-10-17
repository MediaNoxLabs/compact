// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
use super::*;
fn mapping(name: &str, input: Type, source: Expr, body: Expr, result: Type, length: usize) -> Expr {
    Expr::VectorMap {
        parameter: Parameter {
            name: name.into(),
            ty: input,
        },
        source: Box::new(source),
        body: Box::new(body),
        result,
        length,
    }
}

#[test]
fn map_coercion_name_body_and_result_refusals_stay_ordered_even_at_zero_length() {
    let empty = || vector(vec![]);
    assert_eq!(
        compare_map_successor(
            &mapping(
                "bad-name",
                Type::Field,
                boolean(true),
                reference("missing"),
                Type::Field,
                0
            ),
            &HashMap::new(),
            &HashMap::new()
        ),
        Err(RenderError::TypeMismatch {
            expected: vector_type(Type::Field, 0),
            actual: Type::Boolean
        })
    );
    assert_eq!(
        compare_map_successor(
            &mapping(
                "bad-name",
                Type::Field,
                empty(),
                reference("missing"),
                Type::Field,
                0
            ),
            &HashMap::new(),
            &HashMap::new()
        ),
        Err(RenderError::InvalidIdentifier("bad-name".into()))
    );
    assert_eq!(
        compare_map_successor(
            &mapping(
                "item$raw",
                Type::Field,
                empty(),
                reference("missing"),
                Type::Field,
                0
            ),
            &HashMap::new(),
            &HashMap::new()
        ),
        Err(RenderError::UnknownParameter("missing".into()))
    );
    assert_eq!(
        compare_map_successor(
            &mapping(
                "item$raw",
                Type::Field,
                empty(),
                reference("item$raw"),
                Type::Boolean,
                0
            ),
            &HashMap::new(),
            &HashMap::new()
        ),
        Err(RenderError::TypeMismatch {
            expected: Type::Boolean,
            actual: Type::Field
        })
    );
    assert_eq!(
        compare_map_successor(
            &mapping(
                "item$raw",
                Type::Field,
                empty(),
                reference("item$raw"),
                Type::Field,
                0
            ),
            &HashMap::new(),
            &HashMap::new()
        ),
        Ok(vector_type(Type::Field, 0))
    );
}

#[test]
fn map_widening_raw_names_nested_scope_and_outer_binding_restore_match_template_successor() {
    let outer = Type::Boolean;
    let params = HashMap::from([("item$raw", (&outer, syn::parse_quote!(outer_symbol)))]);
    let small = uint("15");
    let large = uint("255");
    let input = Expr::Vector {
        element: small,
        elements: vec![Expr::UnsignedLiteral {
            value: "7".into(),
            max: "15".into(),
        }],
    };
    let mapped = mapping(
        "item$raw",
        large.clone(),
        input,
        reference("item$raw"),
        large.clone(),
        1,
    );
    assert_eq!(
        compare_map_successor(&mapped, &params, &HashMap::new()),
        Ok(vector_type(large, 1))
    );
    let inner = mapping(
        "item$raw",
        Type::Field,
        vector(vec![field("2")]),
        reference("item$raw"),
        Type::Field,
        1,
    );
    let mapped = mapping(
        "item$raw",
        Type::Field,
        vector(vec![field("1")]),
        inner,
        vector_type(Type::Field, 1),
        1,
    );
    let source = Expr::Tuple {
        elements: vec![mapped, reference("item$raw")],
    };
    assert_eq!(
        compare_map_successor(&source, &params, &HashMap::new()),
        Ok(Type::Tuple {
            elements: vec![vector_type(vector_type(Type::Field, 1), 1), Type::Boolean]
        })
    );
    let (result, visits) = visitation::observe(|| observe(&source, &params, &HashMap::new()));
    result.unwrap();
    assert_eq!(visits.len(), 9);
    assert!(visits.values().all(|count| *count == 1));
}

#[test]
fn prepared_map_body_keeps_binary_association_without_child_reparse() {
    let source = mapping(
        "item",
        Type::Field,
        vector(vec![field("1")]),
        Expr::Subtract {
            left: Box::new(field("10")),
            right: Box::new(Expr::Subtract {
                left: Box::new(field("7")),
                right: Box::new(field("2")),
            }),
        },
        Type::Field,
        1,
    );
    assert_eq!(
        compare_map_successor(&source, &HashMap::new(), &HashMap::new()),
        Ok(vector_type(Type::Field, 1))
    );
    let (syntax, _) =
        crate::pure_aggregate_expression(&source, &HashMap::new(), &HashMap::new()).unwrap();
    let (_, closure) = mapped_parts(&syntax);
    let body = mapped_body(closure);
    let syn::Expr::Binary(binary) = body else {
        panic!("binary body");
    };
    assert!(
        matches!(binary.right.as_ref(), syn::Expr::Binary(inner) if matches!(inner.op, syn::BinOp::Sub(_)))
    );
}

// This translator recognizes the complete former Map template, not arbitrary
// loops. Child expressions are recursively preserved, including evaluation order.
pub(super) struct LegacyMapTemplate;
impl syn::visit_mut::VisitMut for LegacyMapTemplate {
    fn visit_expr_mut(&mut self, expression: &mut syn::Expr) {
        syn::visit_mut::visit_expr_mut(self, expression);
        let syn::Expr::Block(block) = expression else {
            return;
        };
        let [
            syn::Stmt::Local(source),
            syn::Stmt::Local(_),
            syn::Stmt::Expr(syn::Expr::ForLoop(mapped), _),
            _,
        ] = block.block.stmts.as_slice()
        else {
            return;
        };
        let syn::Pat::Ident(source_name) = &source.pat else {
            return;
        };
        if source_name.ident != "__compact_map_source" {
            return;
        }
        let Some(source) = &source.init else {
            return;
        };
        let source = &source.expr;
        let item = &mapped.pat;
        let [syn::Stmt::Expr(syn::Expr::MethodCall(push), _)] = mapped.body.stmts.as_slice() else {
            return;
        };
        let Some(body) = push.args.first() else {
            return;
        };
        // Length appears in the original Vec capacity and final array type.
        let syn::Stmt::Local(capacity) = &block.block.stmts[1] else {
            return;
        };
        let Some(capacity) = &capacity.init else {
            return;
        };
        let syn::Expr::Call(capacity) = capacity.expr.as_ref() else {
            return;
        };
        let Some(length) = capacity.args.first() else {
            return;
        };
        let expected: syn::Expr = syn::parse_quote!({
            let __compact_map_source = #source;
            let mut __compact_mapped = Vec::with_capacity(#length);
            for #item in __compact_map_source.into_array() {
                __compact_mapped.push(#body);
            }
            runtime::FixedVector::new(<[_; #length]>::try_from(__compact_mapped).expect("Vector map preserves its length"))
        });
        if block.to_token_stream().to_string() != expected.to_token_stream().to_string() {
            return;
        }
        // The checked successor closure and every native fallible callee use
        // CompactError. Preserve annotated tries and all inner operations.
        let result = match body {
            syn::Expr::Try(fallible) if fallible.attrs.is_empty() => fallible.expr.as_ref().clone(),
            other => syn::parse_quote!(Ok(#other)),
        };
        *expression = syn::parse_quote!((#source).try_map(|#item| -> Result<_, runtime::CompactError> { #result })?);
    }
}

fn mapped_parts(expression: &syn::Expr) -> (&syn::Expr, &syn::ExprClosure) {
    let syn::Expr::Try(outer) = expression else {
        panic!("fallible Map");
    };
    let syn::Expr::MethodCall(call) = outer.expr.as_ref() else {
        panic!("Map method");
    };
    assert_eq!(call.method, "try_map");
    assert_eq!(call.args.len(), 1);
    let syn::Expr::Closure(closure) = call.args.first().unwrap() else {
        panic!("Map closure");
    };
    assert_eq!(closure.inputs.len(), 1);
    let syn::ReturnType::Type(_, output) = &closure.output else {
        panic!("explicit closure result");
    };
    assert_eq!(
        output.to_token_stream().to_string(),
        quote::quote!(Result<_, runtime::CompactError>).to_string()
    );
    (&call.receiver, closure)
}
fn mapped_result(closure: &syn::ExprClosure) -> &syn::Expr {
    let syn::Expr::Block(body) = closure.body.as_ref() else {
        panic!("closure block");
    };
    let [syn::Stmt::Expr(result, None)] = body.block.stmts.as_slice() else {
        panic!("single Result body");
    };
    result
}

fn mapped_body(closure: &syn::ExprClosure) -> &syn::Expr {
    let syn::Expr::Call(ok) = mapped_result(closure) else {
        panic!("infallible body must retain Ok wrapper");
    };
    assert_eq!(ok.func.to_token_stream().to_string(), "Ok");
    assert_eq!(ok.args.len(), 1);
    ok.args.first().unwrap()
}

#[test]
fn fallible_map_scopes_body_question_mark_and_evaluates_coerced_source_once_without_scratch() {
    let small = uint("15");
    let large = uint("255");
    let source_circuit = declaration("source", vec![], vector_type(small, 2));
    let body_circuit = declaration("step", vec![large.clone()], large.clone());
    let circuits = HashMap::from([("source", &source_circuit), ("step", &body_circuit)]);
    for name in ["__compact_mapped", "__compact_map_source"] {
        let source = mapping(
            name,
            large.clone(),
            call("source"),
            Expr::Call {
                name: "step".into(),
                arguments: vec![reference(name)],
            },
            large.clone(),
            2,
        );
        assert_eq!(
            compare_map_successor(&source, &HashMap::new(), &circuits),
            Ok(vector_type(large.clone(), 2))
        );
        let (syntax, _) =
            crate::pure_aggregate_expression(&source, &HashMap::new(), &circuits).unwrap();
        let (receiver, closure) = mapped_parts(&syntax);
        let receiver = receiver.to_token_stream().to_string();
        assert_eq!(
            receiver.matches("pure_circuits :: source").count(),
            1,
            "{receiver}"
        );
        assert!(
            receiver.contains("cast_unsigned :: < 15 , 255 >"),
            "source widening precedes try_map: {receiver}"
        );
        let syn::Expr::Call(result) = mapped_result(closure) else {
            panic!("checked CompactError result call must be returned directly");
        };
        assert_eq!(
            result.func.to_token_stream().to_string(),
            "crate :: pure_circuits :: step"
        );
        assert_eq!(
            closure
                .inputs
                .first()
                .unwrap()
                .to_token_stream()
                .to_string(),
            name
        );
        let output = syntax.to_token_stream().to_string();
        assert!(!output.contains("let __compact_map"), "{output}");
        assert!(!output.contains("let mut __compact_mapped"), "{output}");
    }
}

#[test]
fn legacy_map_translation_refuses_changed_order_body_and_foreign_templates() {
    let source = mapping(
        "item",
        Type::Field,
        vector(vec![field("1")]),
        reference("item"),
        Type::Field,
        1,
    );
    let old = old_oracle::observe(&source, &HashMap::new(), &HashMap::new())
        .unwrap()
        .0;
    let original: syn::Expr = syn::parse_str(&old).unwrap();
    for mutation in 0..3 {
        let mut changed = original.clone();
        let syn::Expr::Block(block) = &mut changed else {
            unreachable!()
        };
        match mutation {
            0 => block.block.stmts.swap(0, 1),
            1 => {
                let syn::Stmt::Expr(syn::Expr::ForLoop(mapped), _) = &mut block.block.stmts[2]
                else {
                    unreachable!()
                };
                mapped
                    .body
                    .stmts
                    .push(syn::parse_quote!(unexpected_effect();));
            }
            _ => block
                .block
                .stmts
                .push(syn::parse_quote!(unexpected_effect();)),
        }
        let before = changed.to_token_stream().to_string();
        syn::visit_mut::VisitMut::visit_expr_mut(&mut LegacyMapTemplate, &mut changed);
        assert_eq!(changed.to_token_stream().to_string(), before);
    }
    // A changed child remains visible after translation, rather than erased.
    let changed = old.replace("push (item)", "push (different)");
    assert_ne!(changed, old);
    let mut changed: syn::Expr = syn::parse_str(&changed).unwrap();
    syn::visit_mut::VisitMut::visit_expr_mut(&mut LegacyMapTemplate, &mut changed);
    assert!(changed.to_token_stream().to_string().contains("different"));
}
