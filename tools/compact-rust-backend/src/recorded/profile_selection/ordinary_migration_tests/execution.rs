// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;

#[test]
fn ordinary_root_snapshot_is_read_once_before_the_write() {
    let mut c: Contract = serde_json::from_str(include_str!(
        "../../../../tests/root-let-action-return-schema20-ir.json"
    ))
    .unwrap();
    let r = &mut c.stateful_circuits[0];
    r.parameters.push(Parameter {
        name: "label".into(),
        ty: Type::OpaqueBytes,
    });
    r.return_value = StateReturn::Expression {
        value: Expr::Parameter {
            name: "before".into(),
        },
    };
    let plan = ordinary(&c, "step").unwrap().unwrap();
    let calls = methods(&plan);
    assert_eq!(calls.iter().filter(|m| *m == "record_read").count(), 1);
    assert_eq!(calls.iter().filter(|m| *m == "record_write").count(), 1);
    assert!(
        calls.iter().position(|m| m == "record_read").unwrap()
            < calls.iter().position(|m| m == "record_write").unwrap()
    );
    let (_, row) = public(&c, "step");
    assert_eq!(row["body_owner"], "algebra.ordinary");
}

#[test]
fn ordinary_conditional_witness_stays_inside_the_selected_branch() {
    use syn::visit_mut::VisitMut;
    let mut c = identity(Type::Bytes { length: 32 });
    c.stateful_circuits[0].parameters.push(Parameter {
        name: "choose".into(),
        ty: Type::Boolean,
    });
    c.witnesses.push(WitnessDeclaration {
        source: None,
        name: "echo".into(),
        parameters: vec![Parameter {
            name: "input".into(),
            ty: Type::Bytes { length: 32 },
        }],
        result: Type::Bytes { length: 32 },
    });
    c.stateful_circuits[0].return_value = StateReturn::Expression {
        value: Expr::If {
            condition: Box::new(Expr::Parameter {
                name: "choose".into(),
            }),
            then: Box::new(Expr::WitnessCall {
                name: "echo".into(),
                arguments: vec![Expr::Parameter {
                    name: "value".into(),
                }],
            }),
            otherwise: Box::new(Expr::Parameter {
                name: "value".into(),
            }),
        },
    };
    let plan = ordinary(&c, "identity").unwrap().unwrap();
    struct Branches {
        depth: usize,
        witness_depths: Vec<usize>,
    }
    impl VisitMut for Branches {
        fn visit_expr_if_mut(&mut self, node: &mut syn::ExprIf) {
            self.visit_expr_mut(&mut node.cond);
            self.depth += 1;
            self.visit_block_mut(&mut node.then_branch);
            if let Some((_, other)) = &mut node.else_branch {
                self.visit_expr_mut(other);
            }
            self.depth -= 1;
        }
        fn visit_expr_method_call_mut(&mut self, node: &mut syn::ExprMethodCall) {
            if node.method == "try_witness_metered" {
                self.witness_depths.push(self.depth);
            }
            syn::visit_mut::visit_expr_method_call_mut(self, node);
        }
    }
    let steps = &plan.steps;
    let mut block: syn::Block = syn::parse_quote!({ #(#steps)* });
    let mut branches = Branches {
        depth: 0,
        witness_depths: vec![],
    };
    branches.visit_block_mut(&mut block);
    assert_eq!(branches.witness_depths, vec![1]);
    let (_, row) = public(&c, "identity");
    assert_eq!(row["body_owner"], "algebra.ordinary");
}

#[test]
fn ordinary_complete_witness_hash_consumers_use_checked_composition() {
    let c: Contract = serde_json::from_str(include_str!(
        "../../../../tests/witness-hash-schema20-ir.json"
    ))
    .unwrap();
    for name in ["persistent_hash_echo", "upgrade_echo"] {
        let plan = ordinary(&c, name).unwrap().unwrap();
        assert_eq!(
            methods(&plan)
                .iter()
                .filter(|m| m.as_str() == "try_witness_metered")
                .count(),
            1
        );
        let (_, row) = public(&c, name);
        assert_eq!(row["body_owner"], "algebra.ordinary");
    }
}
