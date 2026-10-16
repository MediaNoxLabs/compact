// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! All eight operations run through real generic owners and a () consumer.
use super::*;

#[test]
fn all_eight_variants_support_generic_outputs_and_borrowed_signatures() {
    let ty = Type::Struct {
        name: "Record".into(),
        fields: vec![StructField {
            name: "value".into(),
            ty: Type::Field,
        }],
    };
    let vector = || Expr::Vector {
        element: Type::Field,
        elements: vec![field("1")],
    };
    let step = circuit("step$raw", vec![Type::Field, Type::Field], Type::Field);
    let declarations = HashMap::from([("step$raw", &step)]);
    let sources = [
        (
            Expr::StructField {
                value: Box::new(parameter("record")),
                field: "value".into(),
                index: 0,
            },
            Type::Field,
        ),
        (
            Expr::TupleIndex {
                value: Box::new(Expr::Tuple {
                    elements: vec![field("1")],
                }),
                index: 0,
            },
            Type::Field,
        ),
        (
            Expr::StructLiteral {
                ty: ty.clone(),
                fields: vec![field("1")],
            },
            ty.clone(),
        ),
        (
            Expr::Tuple {
                elements: vec![field("1")],
            },
            Type::Tuple {
                elements: vec![Type::Field],
            },
        ),
        (vector(), vector_type(Type::Field, 1)),
        (
            Expr::VectorMap {
                parameter: Parameter {
                    name: "item$raw".into(),
                    ty: Type::Field,
                },
                source: Box::new(vector()),
                body: Box::new(parameter("item$raw")),
                result: Type::Field,
                length: 1,
            },
            vector_type(Type::Field, 1),
        ),
        (
            Expr::VectorFoldCall {
                name: "step$raw".into(),
                initial: Box::new(field("0")),
                source: Box::new(vector()),
                accumulator: Type::Field,
                element: Type::Field,
                length: 1,
            },
            Type::Field,
        ),
        (
            Expr::Call {
                name: "step$raw".into(),
                arguments: vec![field("0"), field("1")],
            },
            Type::Field,
        ),
    ];
    for (source, expected) in sources {
        let mut visitor = Visitor::new(HashMap::from([("record", (&ty, ()))]), &declarations);
        let mut sink = Consumer::default();
        assert_eq!(visitor.check(&source, &mut sink).unwrap().ty, expected);
        assert!(!sink.outputs.is_empty());
        let mut distinct = visitor.visits.clone();
        distinct.sort_unstable();
        distinct.dedup();
        assert_eq!(distinct.len(), visitor.visits.len());
    }
}
