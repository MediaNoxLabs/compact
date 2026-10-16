// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Declared membership interfaces, actual ownership and ordered refusal behavior.
//! Finite whole-old inclusion is covered separately by membership_inclusion tests.
// The direct VectorFoldCall child is a native-valid recording refusal; fixed Field
// commitments themselves are supported. Do not replace this with malformed types.
use super::*;
use crate::ir::{Contract, Expr, Parameter, StateAction, StateReturn, StructField, Type};
use crate::recorded::admission_probe::trace;

fn uint64() -> Type {
    Type::Unsigned {
        max: u64::MAX.to_string(),
    }
}
fn structure(name: &str) -> Type {
    Type::Struct {
        name: name.into(),
        fields: vec![StructField {
            name: "value".into(),
            ty: Type::Field,
        }],
    }
}
fn subject(parameters: Vec<Type>, result: Type) -> Contract {
    let mut c: Contract = serde_json::from_str(include_str!(
        "../../../tests/root-let-action-return-schema20-ir.json"
    ))
    .unwrap();
    let root = &mut c.stateful_circuits[0];
    root.name = "composed".into();
    root.parameters = parameters
        .into_iter()
        .enumerate()
        .map(|(i, ty)| Parameter {
            name: format!("arg{i}"),
            ty,
        })
        .collect();
    root.return_value = StateReturn::Expression {
        value: match &result {
            Type::Unit => Expr::Unit,
            Type::Boolean => Expr::Boolean { value: true },
            Type::Field => Expr::FieldLiteral { value: "7".into() },
            Type::Unsigned { max } => Expr::UnsignedLiteral {
                value: "7".into(),
                max: max.clone(),
            },
            _ => panic!("unsupported test result"),
        },
    };
    root.result = result;
    root.actions = vec![StateAction::CellWrite {
        field: "stored".into(),
        index: 0,
        value: Expr::FieldLiteral { value: "7".into() },
    }];
    c
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
            ledger: c.ledger_fields.iter().map(|v| (v.id.as_str(), v)).collect(),
            witnesses: c.witnesses.iter().map(|v| (v.name.as_str(), v)).collect(),
            pure: c.circuits.iter().map(|v| (v.name.as_str(), v)).collect(),
            circuits: c
                .stateful_circuits
                .iter()
                .map(|v| (v.name.as_str(), v))
                .collect(),
        }
    }
    fn domain(&self, r: &StatefulCircuit) -> Result<Option<TypedPlan>, RenderError> {
        checked_membership_domain(r, &self.ledger, &self.witnesses, &self.pure, &self.circuits)
    }
    fn old(&self, r: &StatefulCircuit) -> Option<TypedPlan> {
        typed_plan::lower(r, &self.ledger, &self.witnesses, &self.pure, &self.circuits)
    }
    // Historical staged-priority oracle; production emission is checked separately.
    fn legacy_selected(&self, r: &StatefulCircuit) -> Result<ProfileSelection, RenderError> {
        legacy_select(
            r,
            &self.ledger,
            &self.witnesses,
            &self.pure,
            &self.circuits,
            None,
        )
    }
}
fn attempts(row: &serde_json::Value) -> Vec<&str> {
    row["attempts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["route"].as_str().unwrap())
        .collect()
}
fn legacy_selected(c: &Contract) -> serde_json::Value {
    let r = &c.stateful_circuits[0];
    let maps = Context::new(c);
    let (_, rows) = trace::capture_json(|| {
        let scope = trace::Scope::begin(r, false);
        let result = maps.legacy_selected(r).unwrap();
        scope.finish(&Ok::<_, RenderError>(
            crate::recorded::RecordingOutcome::Supported(()),
        ));
        result
    });
    rows[0].clone()
}
fn emitted(c: &Contract, name: &str) -> serde_json::Value {
    let (output, rows) = trace::capture_json(|| crate::render_with_capabilities(c));
    let output = output.unwrap();
    syn::parse_file(&output.source).unwrap();
    let cap = output
        .capabilities
        .circuits
        .iter()
        .find(|v| v.name == name)
        .unwrap();
    assert!(cap.recorded && cap.observed_call, "{cap:?}");
    rows.as_array()
        .unwrap()
        .iter()
        .find(|r| r["circuit"] == name && r["phase"] == "public_emission" && r["helper"] == false)
        .unwrap()
        .clone()
}
#[test]
fn declared_interfaces_admit_composition_without_old_effect_counts() {
    let enumeration = Type::Enum {
        name: "Choice".into(),
        variants: vec!["one".into(), "two".into()],
    };
    for (parameters, result) in [
        (vec![], Type::Unit),
        (vec![enumeration], Type::Unit),
        (vec![Type::OpaqueString], Type::Unit),
        (vec![structure("Left"), structure("Right")], Type::Unit),
        (vec![Type::Boolean, uint64(), Type::Boolean], Type::Unit),
        (vec![], Type::Boolean),
        (vec![uint64(), Type::Boolean], Type::Boolean),
        (vec![Type::Field], Type::Field),
        (vec![], uint64()),
        (vec![Type::Boolean, Type::Boolean], uint64()),
    ] {
        let c = subject(parameters, result);
        let maps = Context::new(&c);
        let r = &c.stateful_circuits[0];
        assert!(maps.old(r).is_none(), "new composition: {r:?}");
        assert!(maps.domain(r).unwrap().is_some(), "{r:?}");
        let row = emitted(&c, "composed");
        assert_eq!(row["body_owner"], "algebra.ordinary");
        let a = attempts(&row);
        assert_eq!(a.iter().filter(|&&x| x == "algebra.ordinary").count(), 1);
        assert_eq!(a, ["algebra.ordinary"]);
        assert!(!a.contains(&"selector.membership-optional"));
    }
}
#[test]
fn genuine_string_result_and_unit_return_forms_use_membership() {
    let c: Contract =
        serde_json::from_str(include_str!("../../../tests/bboard-schema20-ir.json")).unwrap();
    let r = c
        .stateful_circuits
        .iter()
        .find(|r| r.name == "take_down")
        .unwrap();
    assert!(Context::new(&c).old(r).is_some());
    assert!(Context::new(&c).domain(r).unwrap().is_some());
    assert_eq!(emitted(&c, "take_down")["body_owner"], "algebra.ordinary");
    let mut c = subject(vec![], Type::Unit);
    c.stateful_circuits[0].return_value = StateReturn::Unit;
    assert_eq!(emitted(&c, "composed")["body_owner"], "algebra.ordinary");
}
#[test]
fn nearby_native_interfaces_decline_without_widening_to_all_public_values() {
    for (parameters, result) in [
        (vec![Type::Field], Type::Unit),
        (vec![Type::OpaqueBytes], Type::Unit),
        (vec![Type::OpaqueString, Type::Boolean], Type::Unit),
        (vec![structure("Single")], Type::Unit),
        (vec![Type::Unsigned { max: "255".into() }], Type::Boolean),
        (vec![Type::Field], Type::Boolean),
        (vec![Type::Boolean], Type::Field),
        (vec![Type::Field, Type::Field], Type::Field),
        (vec![uint64()], uint64()),
        (vec![Type::Boolean], Type::Unsigned { max: "255".into() }),
    ] {
        let c = subject(parameters, result);
        assert!(
            Context::new(&c)
                .domain(&c.stateful_circuits[0])
                .unwrap()
                .is_none()
        );
        // Full native emission remains valid; this test does not demand another recorded owner.
        crate::render_with_capabilities(&c).unwrap();
    }
    let mut c: Contract =
        serde_json::from_str(include_str!("../../../tests/bboard-schema20-ir.json")).unwrap();
    let r = c
        .stateful_circuits
        .iter_mut()
        .find(|r| r.name == "take_down")
        .unwrap();
    r.parameters.push(Parameter {
        name: "unused".into(),
        ty: Type::Boolean,
    });
    let r = c
        .stateful_circuits
        .iter()
        .find(|r| r.name == "take_down")
        .unwrap();
    assert!(Context::new(&c).domain(r).unwrap().is_none());
    crate::render_with_capabilities(&c).unwrap();
}
#[test]
fn internal_helpers_keep_the_same_interface_and_checked_owner() {
    let mut c = subject(vec![Type::OpaqueString], Type::Unit);
    c.stateful_circuits[0].internal = true;
    let r = &c.stateful_circuits[0];
    let maps = Context::new(&c);
    assert!(maps.domain(r).unwrap().is_some());
    let (item, rows) = trace::capture_json(|| {
        let _phase = trace::HelperPlanning::begin();
        crate::recorded::render_recorded_helper(
            r,
            &maps.ledger,
            &maps.witnesses,
            &maps.pure,
            &maps.circuits,
            &std::collections::HashSet::new(),
        )
    });
    assert!(item.unwrap().is_supported());
    assert_eq!(rows[0]["body_owner"], "algebra.ordinary");
    assert_eq!(rows[0]["helper"], true);
}
#[test]
fn retired_organizer_and_effectful_admission_keep_checked_priority() {
    for (json, name, owner) in [
        (
            include_str!("../../../tests/welcome-organizer-schema13-ir.json"),
            "add_participant",
            "algebra.ordinary",
        ),
        (
            include_str!("../../../tests/effectful-return-schema20-ir.json"),
            "choose",
            "effectful.checked-migration",
        ),
    ] {
        let mut c: Contract = serde_json::from_str(json).unwrap();
        c.schema_version = crate::ir::SCHEMA_VERSION;
        let row = emitted(&c, name);
        assert_eq!(row["body_owner"], owner);
        // #590 retires the organizer recognizer. Checked ordinary admission owns the
        // named participant root; the earlier effectful route still bypasses it.
        let a = attempts(&row);
        assert_eq!(
            a.iter()
                .filter(|&&route| route == "algebra.ordinary")
                .count(),
            usize::from(owner == "algebra.ordinary")
        );
        if owner == "algebra.ordinary" {
            // Public emission also probes retained guards after selection.
            // Only the checked selector must stop at its first admitted owner.
            let selected: Vec<_> = a
                .iter()
                .copied()
                .filter(|route| route.starts_with("algebra."))
                .collect();
            assert_eq!(selected, ["algebra.ordinary"]);
        }
        assert!(!a.iter().any(|route| route.starts_with("organizer.")));
    }
}
#[test]
fn interface_decline_reaches_later_independent_unit_owner() {
    let c = subject(vec![Type::Field], Type::Unit);
    assert!(
        Context::new(&c)
            .domain(&c.stateful_circuits[0])
            .unwrap()
            .is_none()
    );
    let current = emitted(&c, "composed");
    assert_eq!(current["body_owner"], "algebra.ordinary");
    assert_eq!(attempts(&current), ["algebra.ordinary"]);
    let row = legacy_selected(&c);
    assert_eq!(row["body_owner"], "algebra.unit-actions");
    let a = attempts(&row);
    assert!(
        a.iter().position(|&x| x == "algebra.membership").unwrap()
            < a.iter().position(|&x| x == "algebra.unit-actions").unwrap()
    );
}
#[test]
fn unsupported_body_declines_and_preserves_later_attempts() {
    let mut c = subject(vec![], Type::Unit);
    c.stateful_circuits[0].actions = vec![StateAction::Expression {
        value: Expr::TransientHash {
            value: Box::new(Expr::Default {
                ty: Type::OpaqueString,
            }),
        },
    }];
    assert!(
        Context::new(&c)
            .domain(&c.stateful_circuits[0])
            .unwrap()
            .is_some()
    );
    crate::recorded::test_support::register_unsupported_fold(&mut c);
    c.stateful_circuits[0].actions[0] = StateAction::Expression {
        value: Expr::PersistentCommit {
            value: Box::new(crate::recorded::test_support::unsupported_fixed_input()),
            opening: Box::new(Expr::BytesLiteral { bytes: vec![0; 32] }),
        },
    };
    assert!(
        Context::new(&c)
            .domain(&c.stateful_circuits[0])
            .unwrap()
            .is_none()
    );
    let row = legacy_selected(&c);
    assert_ne!(row["body_owner"], "algebra.membership");
    let a = attempts(&row);
    let at = a.iter().position(|&x| x == "algebra.membership").unwrap();
    assert_eq!(a[at + 1], "algebra.jubjub-values");
    assert!(a.contains(&"algebra.unit-actions"));
}
#[test]
fn duplicate_and_noncanonical_source_are_not_accepted_plans() {
    let mut c = subject(vec![Type::Boolean, Type::Boolean], Type::Unit);
    c.stateful_circuits[0].parameters[1].name = "arg0".into();
    let maps = Context::new(&c);
    let r = &c.stateful_circuits[0];
    assert!(
        matches!(effect_plan::check_with_calls(r,&maps.ledger,&maps.witnesses,&maps.pure,&maps.circuits),Err(effect_plan::PlanError::Unsupported(gap)) if gap.path=="parameters[1]"&&gap.detail=="duplicate parameter")
    );
    assert!(maps.domain(r).unwrap().is_none());
    assert_ne!(legacy_selected(&c)["body_owner"], "algebra.membership");
    let c = subject(
        vec![Type::Unsigned {
            max: format!("0{}", u64::MAX),
        }],
        Type::Unit,
    );
    assert!(
        Context::new(&c)
            .domain(&c.stateful_circuits[0])
            .unwrap()
            .is_none()
    );
}
#[test]
fn invalid_checked_plan_propagates_its_exact_diagnostic() {
    let gap = RecordingGap {
        resource: None,
        code: crate::recorded::RecordingGapCode::UnsupportedExpression,
        ir_node: "RecordedPlan".into(),
        path: "membership.forged.operand".into(),
        detail: "wrong output operand".into(),
    };
    assert!(
        resolve_checked_plan(Err(effect_plan::PlanError::Unsupported(gap.clone())))
            .unwrap()
            .is_none()
    );
    assert!(
        matches!(resolve_checked_plan(Err(effect_plan::PlanError::Invalid(gap))),Err(RenderError::RecordingPlanInvariant{path,detail}) if path=="membership.forged.operand"&&detail=="wrong output operand")
    );
}
