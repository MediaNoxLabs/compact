// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Diagnostic contracts at the native stateful expression dispatch boundary.
use super::*;
use crate::ir::{Parameter, StateReturn};

const WIDE_MAX: &str = "340282366920938463463374607431768211456";
type Unary = fn(Box<Expr>) -> Expr;
type Binary = fn(Box<Expr>, Box<Expr>) -> Expr;

fn uint(max: &str) -> Type {
    Type::Unsigned { max: max.into() }
}
fn parameter(name: &str) -> Box<Expr> {
    Box::new(Expr::Parameter { name: name.into() })
}
fn mismatch(expected: Type, actual: Type) -> RenderError {
    RenderError::TypeMismatch { expected, actual }
}

#[derive(Default)]
struct Declarations {
    ledger: Vec<LedgerField>,
    pure: Vec<PureCircuit>,
    stateful: Vec<StatefulCircuit>,
    witnesses: Vec<WitnessDeclaration>,
}
struct Observation {
    result: Result<(syn::Expr, Type, bool), RenderError>,
    statements: Vec<syn::Stmt>,
    query_effect: bool,
}
impl Observation {
    fn accepted(self, ty: Type, witness: bool, query: bool) {
        let (_, actual, witness_effect) = self.result.unwrap_or_else(|error| panic!("{error:?}"));
        assert_eq!(actual, ty);
        assert_eq!(witness_effect, witness);
        assert_eq!(self.query_effect, query);
    }
    fn refused(self, error: RenderError) {
        assert_eq!(self.result.err(), Some(error));
    }
    // Only use for rejection before any argument/operand statements are lowered.
    fn refused_before_effects(self, error: RenderError) {
        assert!(self.statements.is_empty());
        assert!(!self.query_effect);
        self.refused(error);
    }
}
impl Declarations {
    fn lower(&self, expression: Expr) -> Observation {
        let types = [
            ("field", Type::Field),
            ("point", Type::JubjubPoint),
            ("bytes", Type::Bytes { length: 32 }),
            ("short_bytes", Type::Bytes { length: 31 }),
            ("boolean", Type::Boolean),
            ("uint", uint("255")),
            ("wide", uint(WIDE_MAX)),
        ];
        let parameters = types
            .iter()
            .map(|(name, ty)| (*name, (ty, syn::Ident::new(name, Span::call_site()))))
            .collect();
        let mut statements = Vec::new();
        let mut next = 0;
        let mut query_effect = false;
        let result = render_state_expression(
            &expression,
            &parameters,
            &self
                .witnesses
                .iter()
                .map(|w| (w.name.as_str(), w))
                .collect(),
            &mut statements,
            &mut next,
            &self.pure.iter().map(|c| (c.name.as_str(), c)).collect(),
            &self.stateful.iter().map(|c| (c.name.as_str(), c)).collect(),
            &self.ledger.iter().map(|f| (f.id.as_str(), f)).collect(),
            &mut query_effect,
        );
        Observation {
            result,
            statements,
            query_effect,
        }
    }
}

#[test]
fn crypto_unary_domains_preserve_exact_diagnostics() {
    let cases: [(Unary, &str, Type, Type, &str, Type); 8] = [
        (
            |value| Expr::DegradeToTransient { value },
            "bytes",
            Type::Bytes { length: 32 },
            Type::Field,
            "short_bytes",
            Type::Bytes { length: 31 },
        ),
        (
            |value| Expr::UpgradeFromTransient { value },
            "field",
            Type::Field,
            Type::Bytes { length: 32 },
            "point",
            Type::JubjubPoint,
        ),
        (
            |value| Expr::JubjubScalarFromNative { value },
            "field",
            Type::Field,
            Type::Field,
            "point",
            Type::JubjubPoint,
        ),
        (
            |scalar| Expr::EcMulGenerator { scalar },
            "field",
            Type::Field,
            Type::JubjubPoint,
            "point",
            Type::JubjubPoint,
        ),
        (
            |value| Expr::JubjubPointX { value },
            "point",
            Type::JubjubPoint,
            Type::Field,
            "field",
            Type::Field,
        ),
        (
            |value| Expr::JubjubPointY { value },
            "point",
            Type::JubjubPoint,
            Type::Field,
            "field",
            Type::Field,
        ),
        (
            |value| Expr::EcNeg { value },
            "point",
            Type::JubjubPoint,
            Type::JubjubPoint,
            "field",
            Type::Field,
        ),
        (
            |value| Expr::FieldToBytes32 { value },
            "field",
            Type::Field,
            Type::Bytes { length: 32 },
            "uint",
            uint("255"),
        ),
    ];
    let declarations = Declarations::default();
    for (operation, good, expected, result, bad, actual) in cases {
        declarations
            .lower(operation(parameter(good)))
            .accepted(result, false, false);
        declarations
            .lower(operation(parameter(bad)))
            .refused_before_effects(mismatch(expected, actual));
    }
}

#[test]
fn point_binary_operations_validate_each_operand_independently() {
    let cases: [(Binary, &str, Type, &str, Type); 3] = [
        (
            |x, y| Expr::ConstructJubjubPoint { x, y },
            "field",
            Type::Field,
            "field",
            Type::Field,
        ),
        (
            |left, right| Expr::EcAdd { left, right },
            "point",
            Type::JubjubPoint,
            "point",
            Type::JubjubPoint,
        ),
        (
            |point, scalar| Expr::EcMul { point, scalar },
            "point",
            Type::JubjubPoint,
            "field",
            Type::Field,
        ),
    ];
    let declarations = Declarations::default();
    for (operation, left, left_ty, right, right_ty) in cases {
        declarations
            .lower(operation(parameter(left), parameter(right)))
            .accepted(Type::JubjubPoint, false, false);
        declarations
            .lower(operation(parameter("boolean"), parameter(right)))
            .refused_before_effects(mismatch(left_ty, Type::Boolean));
        // A valid left operand has already been bound; no blanket rollback claim.
        declarations
            .lower(operation(parameter(left), parameter("boolean")))
            .refused(mismatch(right_ty, Type::Boolean));
    }
}

#[test]
fn commitment_openings_require_their_declared_domain() {
    let declarations = Declarations::default();
    let cases: [(Binary, &str, Type, Type); 2] = [
        (
            |value, opening| Expr::TransientCommit { value, opening },
            "field",
            Type::Field,
            Type::Field,
        ),
        (
            |value, opening| Expr::PersistentCommit { value, opening },
            "bytes",
            Type::Bytes { length: 32 },
            Type::Bytes { length: 32 },
        ),
    ];
    for (operation, opening, expected, result) in cases {
        declarations
            .lower(operation(parameter("boolean"), parameter(opening)))
            .accepted(result, false, false);
        declarations
            .lower(operation(parameter("boolean"), parameter("point")))
            .refused(mismatch(expected, Type::JubjubPoint));
    }
}

#[test]
fn unsigned_conversions_accept_supported_widths_and_refuse_other_domains() {
    let declarations = Declarations::default();
    for name in ["uint", "wide"] {
        declarations
            .lower(Expr::FieldCast {
                value: parameter(name),
            })
            .accepted(Type::Field, false, false);
        declarations
            .lower(Expr::UnsignedCast {
                max: "255".into(),
                value: parameter(name),
            })
            .accepted(uint("255"), false, false);
    }
    declarations
        .lower(Expr::UnsignedCast {
            max: WIDE_MAX.into(),
            value: parameter("uint"),
        })
        .accepted(uint(WIDE_MAX), false, false);
    declarations
        .lower(Expr::FieldCast {
            value: parameter("field"),
        })
        .refused_before_effects(RenderError::ExpectedUnsigned(Type::Field));
    declarations
        .lower(Expr::UnsignedCast {
            max: "255".into(),
            value: parameter("field"),
        })
        .refused_before_effects(mismatch(uint("255"), Type::Field));
}

#[test]
fn arithmetic_validates_both_operands_before_returning_a_typed_value() {
    let declarations = Declarations::default();
    let unsigned: [Binary; 3] = [
        |left, right| Expr::UnsignedAdd {
            max: "65535".into(),
            left,
            right,
        },
        |left, right| Expr::UnsignedSubtract {
            max: "65535".into(),
            left,
            right,
        },
        |left, right| Expr::UnsignedMultiply {
            max: "65535".into(),
            left,
            right,
        },
    ];
    let field: [Binary; 3] = [
        |left, right| Expr::Add { left, right },
        |left, right| Expr::Subtract { left, right },
        |left, right| Expr::Multiply { left, right },
    ];
    for (operations, name, result) in [
        (unsigned, "uint", uint("65535")),
        (field, "field", Type::Field),
    ] {
        for operation in operations {
            declarations
                .lower(operation(parameter(name), parameter(name)))
                .accepted(result.clone(), false, false);
            declarations
                .lower(operation(parameter("boolean"), parameter(name)))
                .refused_before_effects(mismatch(result.clone(), Type::Boolean));
            declarations
                .lower(operation(parameter(name), parameter("boolean")))
                .refused(mismatch(result.clone(), Type::Boolean));
        }
    }
}

#[test]
fn comparisons_check_each_operand_domain_and_supported_width() {
    let declarations = Declarations::default();
    for operator in [
        ComparisonOperator::Less,
        ComparisonOperator::LessEqual,
        ComparisonOperator::Greater,
        ComparisonOperator::GreaterEqual,
    ] {
        let compare = |left, right| Expr::Compare {
            operator,
            left: parameter(left),
            right: parameter(right),
        };
        declarations
            .lower(compare("uint", "uint"))
            .accepted(Type::Boolean, false, false);
        declarations
            .lower(compare("field", "uint"))
            .refused_before_effects(RenderError::ExpectedUnsigned(Type::Field));
        declarations
            .lower(compare("uint", "field"))
            .refused(RenderError::ExpectedUnsigned(Type::Field));
        declarations
            .lower(compare("wide", "uint"))
            .refused_before_effects(RenderError::InvalidUnsignedMaximum(WIDE_MAX.into()));
        declarations
            .lower(compare("uint", "wide"))
            .refused(RenderError::InvalidUnsignedMaximum(WIDE_MAX.into()));
    }
}

fn ledger(declaration: LedgerFieldKind) -> Declarations {
    Declarations {
        ledger: vec![LedgerField {
            id: "slot".into(),
            index: 0,
            path: vec![],
            source: None,
            declaration,
        }],
        ..Default::default()
    }
}

#[test]
fn cell_reads_resolve_name_index_and_declaration_before_query_emission() {
    let read = |field: &str, index| Expr::CellRead {
        field: field.into(),
        index,
    };
    let valid = ledger(LedgerFieldKind::Cell { ty: Type::Field });
    valid
        .lower(read("slot", 0))
        .accepted(Type::Field, false, true);
    valid
        .lower(read("missing", 0))
        .refused_before_effects(RenderError::UnknownLedgerField("missing".into()));
    valid
        .lower(read("slot", 1))
        .refused_before_effects(RenderError::InvalidLedgerIndex(1));
    ledger(LedgerFieldKind::Counter)
        .lower(read("slot", 0))
        .refused_before_effects(RenderError::UnknownLedgerField("slot".into()));
}

#[test]
fn collection_observations_require_matching_kinds_and_indices() {
    type ObservationExpr = fn(String, u8) -> Expr;
    let cases: [(ObservationExpr, LedgerFieldKind); 2] = [
        (
            |field, index| Expr::SetIsEmpty { field, index },
            LedgerFieldKind::Set { ty: Type::Field },
        ),
        (
            |field, index| Expr::MapIsEmpty { field, index },
            LedgerFieldKind::Map {
                key: Type::Field,
                value: Type::Field,
            },
        ),
    ];
    for (expression, declaration) in cases {
        let valid = ledger(declaration);
        valid
            .lower(expression("slot".into(), 0))
            .accepted(Type::Boolean, false, true);
        valid
            .lower(expression("slot".into(), 1))
            .refused_before_effects(RenderError::UnknownLedgerField("slot".into()));
        valid
            .lower(expression("missing".into(), 0))
            .refused_before_effects(RenderError::UnknownLedgerField("missing".into()));
        ledger(LedgerFieldKind::Cell { ty: Type::Field })
            .lower(expression("slot".into(), 0))
            .refused_before_effects(RenderError::UnknownLedgerField("slot".into()));
    }
}

fn callable_parameters() -> Vec<Parameter> {
    vec![Parameter {
        name: "value".into(),
        ty: Type::Field,
    }]
}
fn call(witness: bool, name: &str, arguments: Vec<Expr>) -> Expr {
    if witness {
        Expr::WitnessCall {
            name: name.into(),
            arguments,
        }
    } else {
        Expr::Call {
            name: name.into(),
            arguments,
        }
    }
}

#[test]
fn pure_stateful_and_witness_calls_resolve_exact_argument_contracts() {
    let pure = Declarations {
        pure: vec![PureCircuit {
            source: None,
            name: "helper".into(),
            internal: false,
            parameters: callable_parameters(),
            result: Type::Field,
            body: *parameter("value"),
        }],
        ..Default::default()
    };
    let stateful = Declarations {
        stateful: vec![StatefulCircuit {
            source: None,
            name: "helper".into(),
            internal: true,
            parameters: callable_parameters(),
            actions: vec![],
            result: Type::Field,
            return_value: StateReturn::Expression {
                value: *parameter("value"),
            },
        }],
        ..Default::default()
    };
    let witness = Declarations {
        witnesses: vec![WitnessDeclaration {
            source: None,
            name: "helper".into(),
            parameters: callable_parameters(),
            result: Type::Field,
        }],
        ..Default::default()
    };
    for (declarations, is_witness, is_query) in [
        (pure, false, false),
        (stateful, false, true),
        (witness, true, false),
    ] {
        declarations
            .lower(call(is_witness, "helper", vec![*parameter("field")]))
            .accepted(Type::Field, is_witness, is_query);
        let unknown = if is_witness {
            RenderError::UnknownWitness("missing".into())
        } else {
            RenderError::UnknownCircuit("missing".into())
        };
        declarations
            .lower(call(is_witness, "missing", vec![]))
            .refused_before_effects(unknown);
        for arguments in [vec![], vec![*parameter("field"), *parameter("field")]] {
            let count = arguments.len();
            declarations
                .lower(call(is_witness, "helper", arguments))
                .refused_before_effects(RenderError::ArgumentCount {
                    circuit: "helper".into(),
                    expected: 1,
                    actual: count,
                });
        }
        declarations
            .lower(call(is_witness, "helper", vec![*parameter("point")]))
            .refused_before_effects(mismatch(Type::Field, Type::JubjubPoint));
    }
}
