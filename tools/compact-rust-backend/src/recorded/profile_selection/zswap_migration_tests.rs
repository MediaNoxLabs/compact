// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! The carrier domain is an interface boundary, not a whole-body recognizer.
use super::*;
use crate::ir::{Contract, Expr, Parameter, ReturnPlan, StateAction, StateReturn, Type};
use crate::recorded::admission_probe::trace;

fn source() -> Contract {
    serde_json::from_str(include_str!(
        "../../../tests/native-zswap-intents-schema20-ir.json"
    ))
    .unwrap()
}
fn root<'a>(c: &'a Contract, name: &str) -> &'a StatefulCircuit {
    c.stateful_circuits.iter().find(|r| r.name == name).unwrap()
}
struct Context<'a> {
    ledger: HashMap<&'a str, &'a LedgerField>,
    witnesses: HashMap<&'a str, &'a WitnessDeclaration>,
    pure: HashMap<&'a str, &'a PureCircuit>,
    circuits: HashMap<&'a str, &'a StatefulCircuit>,
}
impl<'a> Context<'a> {
    fn new(c: &'a Contract) -> Self {
        Self {
            ledger: c.ledger_fields.iter().map(|f| (f.id.as_str(), f)).collect(),
            witnesses: c.witnesses.iter().map(|w| (w.name.as_str(), w)).collect(),
            pure: c.circuits.iter().map(|r| (r.name.as_str(), r)).collect(),
            circuits: c
                .stateful_circuits
                .iter()
                .map(|r| (r.name.as_str(), r))
                .collect(),
        }
    }
    fn domain(&self, r: &StatefulCircuit) -> Result<Option<TypedPlan>, RenderError> {
        checked_zswap_domain(r, &self.ledger, &self.witnesses, &self.pure, &self.circuits)
    }
    fn selected(&self, r: &StatefulCircuit) -> (ProfileSelection, serde_json::Value) {
        let (plan, rows) = trace::capture_json(|| {
            let scope = trace::Scope::begin(r, r.internal);
            let plan = select(
                r,
                &self.ledger,
                &self.witnesses,
                &self.pure,
                &self.circuits,
                None,
            )
            .unwrap();
            // Selector trace only; full-render assertions below check actual emission.
            scope.finish(&Ok::<_, RenderError>(
                crate::recorded::RecordingOutcome::Supported(()),
            ));
            plan
        });
        assert_eq!(rows.as_array().unwrap().len(), 1);
        (plan, rows[0].clone())
    }
}
fn attempts(row: &serde_json::Value) -> Vec<&str> {
    row["attempts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["route"].as_str().unwrap())
        .collect()
}

#[test]
fn intent_only_expansion_uses_first_eligible_algebra_owner() {
    let c = source();
    let maps = Context::new(&c);
    for (name, owner) in [
        ("produce", "algebra.membership"),
        ("consume", "algebra.zswap"),
    ] {
        let r = root(&c, name);
        assert!(
            crate::recorded::zswap_plan::lower(r, &maps.ledger, &maps.circuits, &maps.witnesses)
                .is_none()
        );
        assert!(maps.domain(r).unwrap().is_some());
        let (_, selector_row) = maps.selected(r);
        assert_eq!(selector_row["body_owner"], owner);
        assert_eq!(attempts(&selector_row).last(), Some(&owner));
    }
    let (output, rows) = trace::capture_json(|| crate::render_with_capabilities(&c));
    let output = output.unwrap();
    for (name, owner) in [
        ("produce", "algebra.membership"),
        ("consume", "algebra.zswap"),
    ] {
        let capability = output
            .capabilities
            .circuits
            .iter()
            .find(|r| r.name == name)
            .unwrap();
        assert!(capability.recorded && capability.observed_call);
        let rows = rows
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| {
                r["circuit"] == name && r["phase"] == "public_emission" && r["helper"] == false
            })
            .collect::<Vec<_>>();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["body_owner"], owner);
        // Full emission also evaluates capability guards after the selector.
        // Actual body ownership and once-only selection remain authoritative;
        // the isolated selector trace above checks the first eligible domain at its maintained priority.
        assert_eq!(attempts(rows[0]).iter().filter(|&&r| r == owner).count(), 1);
        assert!(!attempts(rows[0]).contains(&"selector.zswap"));
    }
    // Admission/emission alone does not qualify a nonempty proof transcript.
}

#[test]
fn native_valid_interfaces_outside_zswap_domain_stay_excluded() {
    for mode in 0..7 {
        let mut c = source();
        let r = c
            .stateful_circuits
            .iter_mut()
            .find(|r| r.name == "flow")
            .unwrap();
        match mode {
            0 => r.internal = true,
            1 => {
                r.result = Type::Boolean;
                r.return_value = StateReturn::Expression {
                    value: Expr::Boolean { value: true },
                };
            }
            2..=5 => r.parameters.push(Parameter {
                name: "unused_noncarrier".into(),
                ty: match mode {
                    2 => Type::Field,
                    3 => Type::Bytes { length: 31 },
                    4 => Type::Unsigned {
                        max: "65535".into(),
                    },
                    _ => {
                        let Type::Struct { fields, .. } = crate::coin_shapes::shielded_coin_type()
                        else {
                            unreachable!()
                        };
                        Type::Struct {
                            name: "LookalikeCoin".into(),
                            fields,
                        }
                    }
                },
            }),
            6 => {
                r.return_value = StateReturn::Effectful {
                    body: ReturnPlan::Sequence {
                        actions: std::mem::take(&mut r.actions),
                        result: Box::new(ReturnPlan::Value { value: Expr::Unit }),
                    },
                }
            }
            _ => unreachable!(),
        }
        assert!(
            Context::new(&c).domain(root(&c, "flow")).unwrap().is_none(),
            "interface {mode}"
        );
        crate::render_with_capabilities(&c)
            .unwrap_or_else(|error| panic!("interface {mode} must remain native-valid: {error:?}"));
    }
}

#[test]
fn earlier_winners_never_attempt_the_final_zswap_domain() {
    for (text, name, owner) in [
        (
            include_str!("../../../tests/field-observation-schema20-ir.json"),
            "snapshot",
            "algebra.composite-values",
        ),
        (
            include_str!("../../../tests/shielded-receive-schema20-ir.json"),
            "accept",
            "algebra.shielded-receive",
        ),
        (
            include_str!("../../../tests/native-zswap-intents-schema20-ir.json"),
            "witness_order",
            "algebra.membership",
        ),
    ] {
        let c: Contract = serde_json::from_str(text).unwrap();
        let (_, row) = Context::new(&c).selected(root(&c, name));
        assert_eq!(row["body_owner"], owner);
        assert!(!attempts(&row).contains(&"algebra.zswap"));
    }
}

#[test]
fn unsupported_body_refuses_at_last_priority_without_old_fallback() {
    let mut c = source();
    let r = c
        .stateful_circuits
        .iter_mut()
        .find(|r| r.name == "flow")
        .unwrap();
    r.actions = vec![StateAction::Expression {
        // The checked persistent-commit input remains a Bytes32 pair, not Field.
        value: Expr::Equal {
            left: Box::new(Expr::HashToCurve {
                value: Box::new(Expr::PersistentCommit {
                    value: Box::new(Expr::JubjubPointX {
                        value: Box::new(Expr::HashToCurve {
                            value: Box::new(Expr::BytesLiteral { bytes: vec![7; 32] }),
                        }),
                    }),
                    opening: Box::new(Expr::FieldToBytes32 {
                        value: Box::new(Expr::FieldLiteral { value: "1".into() }),
                    }),
                }),
            }),
            right: Box::new(Expr::Default {
                ty: Type::JubjubPoint,
            }),
        },
    }];
    let maps = Context::new(&c);
    assert!(
        crate::recorded::zswap_plan::lower(
            root(&c, "flow"),
            &maps.ledger,
            &maps.circuits,
            &maps.witnesses
        )
        .is_none(),
        "whole frozen Zswap oracle also refuses this expression"
    );
    assert!(maps.domain(root(&c, "flow")).unwrap().is_none());
    let (selected, row) = maps.selected(root(&c, "flow"));
    assert!(selected.typed_result.is_none());
    let attempted = attempts(&row);
    assert_eq!(
        &attempted[attempted.len() - 3..],
        &[
            "algebra.unit-composition",
            "algebra.zswap",
            "algebra.ordinary"
        ]
    );
    assert!(!attempted.contains(&"selector.zswap"));
    crate::render_with_capabilities(&c).unwrap();
}

#[test]
fn former_boolean_inequality_refusal_preserves_earlier_checked_unit_owner() {
    let mut c = source();
    c.stateful_circuits
        .iter_mut()
        .find(|r| r.name == "flow")
        .unwrap()
        .actions = vec![StateAction::Expression {
        value: Expr::NotEqual {
            left: Box::new(Expr::Boolean { value: true }),
            right: Box::new(Expr::Boolean { value: false }),
        },
    }];
    let maps = Context::new(&c);
    assert!(
        crate::recorded::zswap_plan::lower(
            root(&c, "flow"),
            &maps.ledger,
            &maps.circuits,
            &maps.witnesses
        )
        .is_none()
    );
    assert!(maps.domain(root(&c, "flow")).unwrap().is_some());
    let (selected, row) = maps.selected(root(&c, "flow"));
    assert!(selected.typed_result.is_some());
    assert_eq!(row["body_owner"], "algebra.unit-composition");
    let (output, rows) = trace::capture_json(|| crate::render_with_capabilities(&c));
    let output = output.unwrap();
    let capability = output
        .capabilities
        .circuits
        .iter()
        .find(|r| r.name == "flow")
        .unwrap();
    assert!(capability.recorded && capability.observed_call);
    let rows = rows
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| {
            r["circuit"] == "flow" && r["phase"] == "public_emission" && r["helper"] == false
        })
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["body_owner"], "algebra.unit-composition");
    assert_eq!(
        attempts(rows[0])
            .iter()
            .filter(|&&r| r == "algebra.zswap")
            .count(),
        0
    );
    assert!(!attempts(rows[0]).contains(&"algebra.ordinary"));
    assert!(!attempts(rows[0]).contains(&"selector.zswap"));
}
