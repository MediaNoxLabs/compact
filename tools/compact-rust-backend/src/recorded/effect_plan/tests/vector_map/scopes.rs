// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;

#[test]
fn nested_maps_shadow_only_their_formal_and_preserve_outer_capture_ids() {
    let inner = mapped(
        Expr::Tuple {
            elements: vec![parameter("item")],
        },
        addition(),
        Type::Field,
        1,
    );
    let mut c = circuit(2, inner);
    c.result = vector(vector(Type::Field, 1), 2);
    let StateReturn::Expression {
        value: Expr::VectorMap { result, .. },
    } = &mut c.return_value
    else {
        unreachable!()
    };
    *result = vector(Type::Field, 1);
    let checked = checked(&c);
    let Operation::VectorMap {
        parameter: outer,
        body,
        ..
    } = &checked.plan().body.bindings.last().unwrap().operation
    else {
        unreachable!()
    };
    let Operation::VectorMap {
        parameter: inner,
        body,
        ..
    } = &body.bindings.last().unwrap().operation
    else {
        unreachable!()
    };
    assert_ne!(inner.id, outer.id);
    let Operation::Add(left, capture) = &body.bindings[0].operation else {
        unreachable!()
    };
    assert_eq!(left.id, inner.id);
    assert_eq!(capture.id, checked.plan().parameters[1].id);
    crate::render_with_capabilities(&contract(c)).unwrap();
}

#[test]
fn certified_calls_and_lazy_pure_branches_are_allowed_but_direct_context_is_not() {
    let pure = PureCircuit {
        source: None,
        name: "identity".into(),
        internal: true,
        parameters: vec![Parameter {
            name: "x".into(),
            ty: Type::Field,
        }],
        result: Type::Field,
        body: parameter("x"),
    };
    let mut c = circuit(
        0,
        Expr::If {
            condition: Box::new(Expr::Boolean { value: true }),
            then: Box::new(Expr::Call {
                name: "identity".into(),
                arguments: vec![parameter("item")],
            }),
            otherwise: Box::new(addition()),
        },
    );
    let source_helper = PureCircuit {
        source: None,
        name: "source_values".into(),
        internal: true,
        parameters: vec![Parameter {
            name: "values".into(),
            ty: vector(Type::Field, 0),
        }],
        result: vector(Type::Field, 0),
        body: parameter("values"),
    };
    let StateReturn::Expression {
        value: Expr::VectorMap { source, .. },
    } = &mut c.return_value
    else {
        unreachable!()
    };
    **source = Expr::Call {
        name: "source_values".into(),
        arguments: vec![parameter("input")],
    };
    let pure_env = HashMap::from([("identity", &pure), ("source_values", &source_helper)]);
    let checked = check_with_calls(
        &c,
        &HashMap::new(),
        &HashMap::new(),
        &pure_env,
        &HashMap::new(),
    )
    .unwrap();
    let syntax = render::render(&checked).unwrap();
    let mut collector = SourceCalls::default();
    for mut step in syntax.steps {
        syn::visit_mut::VisitMut::visit_stmt_mut(&mut collector, &mut step);
    }
    assert_eq!(collector.calls, 1);
    assert_eq!(collector.calls_inside_loop, 0);
    let mut whole = contract(c);
    whole.circuits.extend([pure, source_helper]);
    let rendered = crate::render_with_capabilities(&whole).unwrap();
    let mut file = syn::parse_file(&rendered.source).unwrap();
    let mut all = SourceCalls::default();
    syn::visit_mut::VisitMut::visit_file_mut(&mut all, &mut file);
    assert_eq!(
        all.calls, 2,
        "one native source call and one recorded source call"
    );
    assert_eq!(all.calls_inside_loop, 0);
    for in_source in [false, true] {
        let mut c = circuit(1, parameter("item"));
        let StateReturn::Expression {
            value: Expr::VectorMap { source, body, .. },
        } = &mut c.return_value
        else {
            unreachable!()
        };
        let context = Expr::CellRead {
            field: "value".into(),
            index: 0,
        };
        if in_source {
            **source = context;
        } else {
            **body = context;
        }
        let mut whole = contract(c.clone());
        whole.ledger_fields.push(LedgerField {
            source: None,
            id: "value".into(),
            index: 0,
            path: vec![0],
            declaration: LedgerFieldKind::Cell {
                ty: if in_source {
                    vector(Type::Field, 1)
                } else {
                    Type::Field
                },
            },
        });
        let ledger = whole
            .ledger_fields
            .iter()
            .map(|f| (f.id.as_str(), f))
            .collect();
        let error = check(&c, &ledger, &HashMap::new()).unwrap_err();
        assert!(
            matches!(error, PlanError::Unsupported(ref gap) if gap.detail.contains("must be context-free"))
        );
        assert!(matches!(
            crate::render_with_capabilities(&whole),
            Err(crate::RenderError::EffectfulExpression)
        ));
    }
}

// Origin is separate from semantic effects: an already evaluated private value
// may be used by a pure mapper, while evaluating a witness inside it is forbidden.
#[test]
fn outer_witness_vector_is_evaluated_before_mapping_and_direct_source_is_rejected() {
    let witness = WitnessDeclaration {
        source: None,
        name: "values".into(),
        parameters: vec![],
        result: vector(Type::Field, 2),
    };
    let call = Expr::WitnessCall {
        name: "values".into(),
        arguments: vec![],
    };
    let mut c = circuit(2, addition());
    let StateReturn::Expression { value } = &mut c.return_value else {
        unreachable!()
    };
    *value = Expr::Let {
        bindings: vec![LocalBinding {
            name: "input".into(),
            ty: vector(Type::Field, 2),
            value: call.clone(),
        }],
        body: Box::new(value.clone()),
    };
    let witnesses = HashMap::from([("values", &witness)]);
    let checked = check(&c, &HashMap::new(), &witnesses).unwrap();
    assert!(matches!(
        checked.plan().body.bindings[0].operation,
        Operation::Witness { .. }
    ));
    let Operation::VectorMap { source, .. } = &checked.plan().body.bindings[1].operation else {
        unreachable!()
    };
    assert_eq!(source.id, checked.plan().body.bindings[0].value.id);
    let mut whole = contract(c.clone());
    whole.witnesses.push(witness.clone());
    crate::render_with_capabilities(&whole).unwrap();
    c.return_value = StateReturn::Expression {
        value: mapped(call, addition(), Type::Field, 2),
    };
    let error = check(&c, &HashMap::new(), &witnesses).unwrap_err();
    assert!(
        matches!(error, PlanError::Unsupported(ref gap) if gap.path == "return_value.value.source" && gap.detail == "VectorMap source must be context-free")
    );
    whole.stateful_circuits = vec![c];
    assert!(matches!(
        crate::render_with_capabilities(&whole),
        Err(crate::RenderError::EffectfulExpression)
    ));
}

#[derive(Default)]
struct SourceCalls {
    calls: usize,
    calls_inside_loop: usize,
    depth: usize,
}
impl syn::visit_mut::VisitMut for SourceCalls {
    fn visit_expr_for_loop_mut(&mut self, node: &mut syn::ExprForLoop) {
        self.depth += 1;
        syn::visit_mut::visit_expr_for_loop_mut(self, node);
        self.depth -= 1;
    }
    fn visit_expr_call_mut(&mut self, node: &mut syn::ExprCall) {
        if matches!(&*node.func, syn::Expr::Path(path) if path.path.segments.last().is_some_and(|segment| segment.ident == "source_values"))
        {
            self.calls += 1;
            self.calls_inside_loop += usize::from(self.depth > 0);
        }
        syn::visit_mut::visit_expr_call_mut(self, node);
    }
}
