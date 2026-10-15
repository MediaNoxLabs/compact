// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! No-syntax child visitor and complete conversion-family contracts.
use super::*;
use crate::pure_semantics::{
    carrier::CheckSink,
    leaf::{LeafSource, fold_leaf},
    sink::{CheckedLeaf, ConversionSink, LeafSink},
};
use std::collections::HashMap;

struct CheckConsumer {
    carriers: CheckSink,
    conversions: usize,
}
impl Default for CheckConsumer {
    fn default() -> Self {
        Self {
            carriers: CheckSink,
            conversions: 0,
        }
    }
}
impl LeafSink for CheckConsumer {
    type Value = ();
    type Binding = ();
    type Carriers = CheckSink;
    fn carriers(&mut self) -> &mut CheckSink {
        &mut self.carriers
    }
    fn leaf(&mut self, _: CheckedLeaf<'_, (), ()>) {}
}
impl ConversionSink for CheckConsumer {
    fn conversion(&mut self, _: ConversionFacts, _: ()) {
        self.conversions += 1;
    }
}

struct LeafChildren<'a> {
    types: HashMap<&'a str, (&'a Type, ())>,
    visits: usize,
}
impl Children<CheckConsumer> for LeafChildren<'_> {
    fn visit(&mut self, source: &Expr, sink: &mut CheckConsumer) -> Result<Typed<()>, RenderError> {
        self.visits += 1;
        let leaf = match source {
            Expr::Parameter { name } => LeafSource::Parameter(name),
            Expr::Boolean { value } => LeafSource::Boolean(*value),
            Expr::FieldLiteral { value } => LeafSource::FieldLiteral(value),
            Expr::UnsignedLiteral { value, max } => LeafSource::UnsignedLiteral { value, max },
            Expr::KernelSelf { .. } => LeafSource::Effectful,
            _ => unreachable!("prepared test child protocol is leaf-only"),
        };
        fold_leaf(leaf, &self.types, sink)
    }
}
fn parameter(name: &str) -> Expr {
    Expr::Parameter { name: name.into() }
}
fn uint(max: &str) -> Type {
    Type::Unsigned { max: max.into() }
}

#[test]
fn no_syntax_consumer_supports_all_four_conversions_with_one_child_visit() {
    let unsigned = uint("255");
    let field = Type::Field;
    let child = parameter("value");
    for (source, ty, expected) in [
        (
            ConversionSource::FieldCast { value: &child },
            &unsigned,
            Type::Field,
        ),
        (
            ConversionSource::FieldToBytes32 { value: &child },
            &field,
            Type::Bytes { length: 32 },
        ),
        (
            ConversionSource::Coerce {
                value: &child,
                ty: &field,
            },
            &unsigned,
            Type::Field,
        ),
        (
            ConversionSource::UnsignedCast {
                max: "15",
                value: &child,
            },
            &unsigned,
            uint("15"),
        ),
    ] {
        let mut children = LeafChildren {
            types: HashMap::from([("value", (ty, ()))]),
            visits: 0,
        };
        let mut sink = CheckConsumer::default();
        assert_eq!(
            fold_conversion(source, &mut children, &mut sink)
                .unwrap()
                .ty,
            expected
        );
        assert_eq!((children.visits, sink.conversions), (1, 1));
    }
}

#[test]
fn explicit_target_precedes_child_while_other_conversions_preserve_child_errors() {
    let missing = parameter("missing");
    let mut children = LeafChildren {
        types: HashMap::new(),
        visits: 0,
    };
    let mut sink = CheckConsumer::default();
    assert_eq!(
        fold_conversion(
            ConversionSource::UnsignedCast {
                max: "01",
                value: &missing
            },
            &mut children,
            &mut sink
        )
        .err(),
        Some(RenderError::InvalidUnsignedMaximum("01".into()))
    );
    assert_eq!((children.visits, sink.conversions), (0, 0));
    for source in [
        ConversionSource::FieldCast { value: &missing },
        ConversionSource::FieldToBytes32 { value: &missing },
        ConversionSource::Coerce {
            value: &missing,
            ty: &uint("bad"),
        },
    ] {
        let mut children = LeafChildren {
            types: HashMap::new(),
            visits: 0,
        };
        let mut sink = CheckConsumer::default();
        assert_eq!(
            fold_conversion(source, &mut children, &mut sink).err(),
            Some(RenderError::UnknownParameter("missing".into()))
        );
        assert_eq!((children.visits, sink.conversions), (1, 0));
    }
}

#[test]
fn wrong_operand_kind_and_malformed_source_bound_keep_exact_refusals() {
    let source = parameter("value");
    let boolean = Type::Boolean;
    let field = Type::Field;
    let mut children = LeafChildren {
        types: HashMap::from([("value", (&boolean, ()))]),
        visits: 0,
    };
    assert_eq!(
        fold_conversion(
            ConversionSource::FieldCast { value: &source },
            &mut children,
            &mut CheckConsumer::default()
        )
        .err(),
        Some(RenderError::ExpectedUnsigned(Type::Boolean))
    );
    let malformed = uint("bad");
    for operation in [
        ConversionSource::FieldCast { value: &source },
        ConversionSource::UnsignedCast {
            max: "255",
            value: &source,
        },
    ] {
        let mut children = LeafChildren {
            types: HashMap::from([("value", (&malformed, ()))]),
            visits: 0,
        };
        assert_eq!(
            fold_conversion(operation, &mut children, &mut CheckConsumer::default()).err(),
            Some(RenderError::InvalidUnsignedMaximum("bad".into()))
        );
    }
    let mut children = LeafChildren {
        types: HashMap::from([("value", (&field, ()))]),
        visits: 0,
    };
    assert_eq!(
        fold_conversion(
            ConversionSource::UnsignedCast {
                max: "255",
                value: &source
            },
            &mut children,
            &mut CheckConsumer::default()
        )
        .err(),
        Some(RenderError::TypeMismatch {
            expected: uint("255"),
            actual: Type::Field
        })
    );
}

#[test]
fn implicit_relation_and_explicit_narrowing_remain_distinct() {
    let source = parameter("value");
    let actual = uint("255");
    let target = uint("15");
    let mut children = LeafChildren {
        types: HashMap::from([("value", (&actual, ()))]),
        visits: 0,
    };
    assert_eq!(
        fold_conversion(
            ConversionSource::Coerce {
                value: &source,
                ty: &target
            },
            &mut children,
            &mut CheckConsumer::default()
        )
        .err(),
        Some(RenderError::TypeMismatch {
            expected: target.clone(),
            actual: actual.clone()
        })
    );
    let mut children = LeafChildren {
        types: HashMap::from([("value", (&actual, ()))]),
        visits: 0,
    };
    assert_eq!(
        fold_conversion(
            ConversionSource::UnsignedCast {
                max: "15",
                value: &source
            },
            &mut children,
            &mut CheckConsumer::default()
        )
        .unwrap()
        .ty,
        target
    );
}

#[test]
fn coercion_reuses_actual_per_request_limit_after_checking_child() {
    let source = parameter("value");
    let actual = Type::Vector {
        element: Box::new(Type::Tuple {
            elements: vec![uint("15"); 16],
        }),
        length: 733,
    };
    let target = Type::Tuple {
        elements: vec![
            Type::Vector {
                element: Box::new(uint("255")),
                length: 16
            };
            733
        ],
    };
    let mut children = LeafChildren {
        types: HashMap::from([("value", (&actual, ()))]),
        visits: 0,
    };
    let mut sink = CheckConsumer::default();
    assert!(matches!(
        fold_conversion(
            ConversionSource::Coerce {
                value: &source,
                ty: &target
            },
            &mut children,
            &mut sink
        ),
        Err(RenderError::ResourceLimit {
            resource: "coercion_work",
            limit: 131_072,
            ..
        })
    ));
    assert_eq!((children.visits, sink.conversions), (1, 0));
}
