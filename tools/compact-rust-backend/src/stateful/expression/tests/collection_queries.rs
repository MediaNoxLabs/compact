// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Declaration, operand and effect contracts for native ledger queries.
use super::*;
use syn::visit_mut::{self, VisitMut};

fn slot(kind: LedgerFieldKind) -> Declarations {
    Declarations {
        ledger: vec![LedgerField {
            id: "slot".into(),
            index: 2,
            path: vec![1, 2],
            source: None,
            declaration: kind,
        }],
        ..Default::default()
    }
}

type OperandQuery = fn(String, u8, Box<Expr>) -> Expr;
fn operand_queries() -> [(OperandQuery, LedgerFieldKind, Type); 3] {
    [
        (
            |field, index, value| Expr::SetMember {
                field,
                index,
                value,
            },
            LedgerFieldKind::Set { ty: Type::Field },
            Type::Boolean,
        ),
        (
            |field, index, key| Expr::MapMember { field, index, key },
            LedgerFieldKind::Map {
                key: Type::Field,
                value: Type::JubjubPoint,
            },
            Type::Boolean,
        ),
        (
            |field, index, key| Expr::MapLookup { field, index, key },
            LedgerFieldKind::Map {
                key: Type::Field,
                value: Type::JubjubPoint,
            },
            Type::JubjubPoint,
        ),
    ]
}

#[test]
fn membership_and_lookup_validate_declarations_before_operands() {
    for (query, kind, result) in operand_queries() {
        let mut declarations = slot(kind);
        declarations
            .lower(query("slot".into(), 2, parameter("field")))
            .accepted(result, false, true);
        // An unresolved operand cannot mask an invalid ledger reference.
        declarations
            .lower(query("missing".into(), 2, parameter("absent")))
            .refused_before_effects(RenderError::UnknownLedgerField("missing".into()));
        declarations
            .lower(query("slot".into(), 9, parameter("absent")))
            .refused_before_effects(RenderError::InvalidLedgerIndex(9));
        declarations
            .lower(query("slot".into(), 2, parameter("point")))
            .refused_before_effects(mismatch(Type::Field, Type::JubjubPoint));
        declarations
            .lower(query("slot".into(), 2, parameter("absent")))
            .refused_before_effects(RenderError::UnknownParameter("absent".into()));
        declarations.ledger[0].declaration = LedgerFieldKind::Counter;
        declarations
            .lower(query("slot".into(), 2, parameter("absent")))
            .refused_before_effects(RenderError::UnknownLedgerField("slot".into()));
    }
}

// Inspect only witness/query calls, not incidental emitted syntax or temp names.
#[derive(Default)]
struct Calls(Vec<&'static str>);
impl VisitMut for Calls {
    fn visit_expr_method_call_mut(&mut self, node: &mut syn::ExprMethodCall) {
        if node.method == "item"
            && matches!(node.receiver.as_ref(), syn::Expr::Path(p) if p.path.is_ident("witnesses"))
        {
            self.0.push("witness");
        }
        if let syn::Expr::Path(path) = node.receiver.as_ref() {
            let names: Vec<_> = path
                .path
                .segments
                .iter()
                .map(|s| s.ident.to_string())
                .collect();
            if names == ["crate", "ledger_slots", "slot"]
                && (node.method == "member" || node.method == "lookup")
            {
                self.0.push("query");
            }
        }
        visit_mut::visit_expr_method_call_mut(self, node);
    }
}
fn calls(observation: &mut Observation) -> Vec<&'static str> {
    let mut visitor = Calls::default();
    for statement in &mut observation.statements {
        visitor.visit_stmt_mut(statement);
    }
    if let Ok((value, _, _)) = &mut observation.result {
        visitor.visit_expr_mut(value);
    }
    visitor.0
}

#[test]
fn witness_result_is_bound_once_before_the_matching_query() {
    for (query, kind, result) in operand_queries() {
        let mut declarations = slot(kind);
        declarations.witnesses.push(WitnessDeclaration {
            source: None,
            name: "item".into(),
            parameters: vec![],
            result: Type::Field,
        });
        let expression = query(
            "slot".into(),
            2,
            Box::new(Expr::WitnessCall {
                name: "item".into(),
                arguments: vec![],
            }),
        );
        let mut accepted = declarations.lower(expression.clone());
        assert_eq!(calls(&mut accepted), ["witness", "query"]);
        // A preorder visitor alone would also accept calls hidden in dead branches.
        // Require direct, unconditional let initializers and link the queried value
        // to the witness result binding, rather than merely counting syntax nodes.
        let direct: Vec<_> = accepted
            .statements
            .iter()
            .filter_map(|statement| {
                let syn::Stmt::Local(local) = statement else {
                    return None;
                };
                let syn::Expr::Try(attempt) = local.init.as_ref()?.expr.as_ref() else {
                    return None;
                };
                let syn::Expr::MethodCall(call) = attempt.expr.as_ref() else {
                    return None;
                };
                if call.method == "item" || call.method == "member" || call.method == "lookup" {
                    Some((&local.pat, call))
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(direct.len(), 2);
        assert_eq!(direct[0].1.method, "item");
        let syn::Pat::Tuple(bindings) = direct[0].0 else {
            panic!("witness must bind private state and result")
        };
        assert_eq!(bindings.elems.len(), 2);
        let syn::Pat::Ident(result_binding) = &bindings.elems[1] else {
            panic!("witness result must have a local binding")
        };
        let method = if matches!(expression, Expr::MapLookup { .. }) {
            "lookup"
        } else {
            "member"
        };
        assert_eq!(direct[1].1.method, method);
        assert_eq!(direct[1].1.args.len(), 2);
        assert!(matches!(&direct[1].1.args[0], syn::Expr::Path(p) if p.path.is_ident("context")));
        assert!(
            matches!(&direct[1].1.args[1], syn::Expr::Path(p) if p.path.is_ident(&result_binding.ident))
        );
        accepted.accepted(result, true, true);

        declarations.witnesses[0].result = Type::JubjubPoint;
        let mut rejected = declarations.lower(expression);
        assert_eq!(
            calls(&mut rejected),
            ["witness"],
            "no query after an invalid operand type"
        );
        assert!(!rejected.query_effect);
        rejected.refused(mismatch(Type::Field, Type::JubjubPoint));
    }
}

type PlainQuery = fn(String, u8) -> Expr;
#[test]
fn collection_metadata_checks_kind_and_index_before_emitting_queries() {
    let cases: [(PlainQuery, LedgerFieldKind, Type); 6] = [
        (
            |field, index| Expr::CounterRead { field, index },
            LedgerFieldKind::Counter,
            uint(&u64::MAX.to_string()),
        ),
        (
            |field, index| Expr::SetSize { field, index },
            LedgerFieldKind::Set { ty: Type::Field },
            uint(&u64::MAX.to_string()),
        ),
        (
            |field, index| Expr::SetIsEmpty { field, index },
            LedgerFieldKind::Set { ty: Type::Field },
            Type::Boolean,
        ),
        (
            |field, index| Expr::MapIsEmpty { field, index },
            LedgerFieldKind::Map {
                key: Type::Field,
                value: Type::JubjubPoint,
            },
            Type::Boolean,
        ),
        (
            |field, index| Expr::ListLength { field, index },
            LedgerFieldKind::List { ty: Type::Field },
            uint(&u64::MAX.to_string()),
        ),
        (
            |field, index| Expr::ListIsEmpty { field, index },
            LedgerFieldKind::List { ty: Type::Field },
            Type::Boolean,
        ),
    ];
    for (query, kind, result) in cases {
        let mut declarations = slot(kind);
        declarations
            .lower(query("slot".into(), 2))
            .accepted(result, false, true);
        declarations
            .lower(query("missing".into(), 2))
            .refused_before_effects(RenderError::UnknownLedgerField("missing".into()));
        declarations
            .lower(query("slot".into(), 9))
            .refused_before_effects(RenderError::UnknownLedgerField("slot".into()));
        declarations.ledger[0].declaration = LedgerFieldKind::Cell { ty: Type::Field };
        declarations
            .lower(query("slot".into(), 2))
            .refused_before_effects(RenderError::UnknownLedgerField("slot".into()));
    }
}

#[test]
fn merkle_checks_distinguish_historic_trees_and_digest_domains() {
    let digest = Type::Struct {
        name: "MerkleTreeDigest".into(),
        fields: vec![StructField {
            name: "field".into(),
            ty: Type::Field,
        }],
    };
    for historic in [false, true] {
        let kind = if historic {
            LedgerFieldKind::HistoricMerkleTree {
                depth: 8,
                ty: Type::Field,
            }
        } else {
            LedgerFieldKind::MerkleTree {
                depth: 8,
                ty: Type::Field,
            }
        };
        let mut declarations = slot(kind);
        let query = |historic, index, root| {
            if historic {
                Expr::HistoricMerkleCheckRoot {
                    field: "slot".into(),
                    index,
                    root,
                }
            } else {
                Expr::MerkleCheckRoot {
                    field: "slot".into(),
                    index,
                    root,
                }
            }
        };
        let root = Box::new(Expr::StructLiteral {
            ty: digest.clone(),
            fields: vec![*parameter("field")],
        });
        declarations
            .lower(query(historic, 2, root))
            .accepted(Type::Boolean, false, true);
        for (flavor, index) in [(!historic, 2), (historic, 9)] {
            declarations
                .lower(query(flavor, index, parameter("absent")))
                .refused_before_effects(RenderError::UnknownLedgerField("slot".into()));
        }
        declarations
            .lower(query(historic, 2, parameter("field")))
            .refused_before_effects(mismatch(digest.clone(), Type::Field));
        declarations.ledger[0].declaration = LedgerFieldKind::Cell { ty: digest.clone() };
        declarations
            .lower(query(historic, 2, parameter("absent")))
            .refused_before_effects(RenderError::UnknownLedgerField("slot".into()));
    }
}

#[test]
fn list_head_preserves_maybe_element_type_and_rejects_wrong_annotation() {
    let mut declarations = slot(LedgerFieldKind::List {
        ty: Type::JubjubPoint,
    });
    let maybe = Type::Struct {
        name: "Maybe".into(),
        fields: vec![
            StructField {
                name: "is_some".into(),
                ty: Type::Boolean,
            },
            StructField {
                name: "value".into(),
                ty: Type::JubjubPoint,
            },
        ],
    };
    let query = |ty| Expr::ListHead {
        field: "slot".into(),
        index: 2,
        ty,
    };
    declarations
        .lower(query(maybe.clone()))
        .accepted(maybe.clone(), false, true);
    declarations
        .lower(query(Type::Boolean))
        .refused_before_effects(mismatch(maybe.clone(), Type::Boolean));
    declarations.ledger[0].declaration = LedgerFieldKind::Set {
        ty: Type::JubjubPoint,
    };
    declarations
        .lower(query(maybe))
        .refused_before_effects(RenderError::UnknownLedgerField("slot".into()));
}

#[test]
fn counter_comparison_requires_uint64_and_checks_ledger_before_threshold() {
    let mut declarations = slot(LedgerFieldKind::Counter);
    let query = |index, threshold| Expr::CounterLessThan {
        field: "slot".into(),
        index,
        threshold,
    };
    let limit = uint(&u64::MAX.to_string());
    declarations
        .lower(query(
            2,
            Box::new(Expr::UnsignedLiteral {
                max: u64::MAX.to_string(),
                value: "10".into(),
            }),
        ))
        .accepted(Type::Boolean, false, true);
    declarations
        .lower(query(2, parameter("field")))
        .refused_before_effects(mismatch(limit, Type::Field));
    declarations
        .lower(query(9, parameter("absent")))
        .refused_before_effects(RenderError::UnknownLedgerField("slot".into()));
    declarations.ledger[0].declaration = LedgerFieldKind::Cell { ty: Type::Field };
    declarations
        .lower(query(2, parameter("absent")))
        .refused_before_effects(RenderError::UnknownLedgerField("slot".into()));
}
