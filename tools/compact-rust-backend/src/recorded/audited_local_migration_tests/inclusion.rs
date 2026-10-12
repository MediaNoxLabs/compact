// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;

#[test]
fn whole_original_attestation_wrappers_preserve_reads_witness_and_counter_order() {
    let c = fixture();
    for name in [VERIFY, ACCEPT] {
        let methods = calls(&included(&c, name));
        let reads: Vec<_> = methods
            .iter()
            .enumerate()
            .filter(|(_, n)| n.as_str() == "record_read")
            .map(|(i, _)| i)
            .collect();
        assert_eq!(reads.len(), 2);
        let witness = methods
            .iter()
            .position(|n| n == "try_witness_metered")
            .unwrap();
        assert!(reads[1] < witness);
        let increment = methods.iter().position(|n| n == "record_increment");
        if name == ACCEPT {
            assert!(increment.unwrap() > witness);
        } else {
            assert!(increment.is_none());
        }
    }
}

#[test]
fn coherent_slot_names_and_native_declaration_authority_are_independent() {
    let renamed = change(fixture(), |v| {
        rename(
            v,
            &[
                ("open", "r#type"),
                ("attestorKey", "r#match"),
                ("acceptedCount", "r#loop"),
                ("schnorrVerifyDigest", "renamed$helper"),
            ],
        )
    });
    included(&renamed, ACCEPT);
    let layout = change(fixture(), |v| {
        edit(v, &mut |m| {
            if let Some(index) = m.get("index").and_then(Value::as_u64)
                && (m.contains_key("declaration")
                    || m.get("field").is_some_and(|f| {
                        matches!(f.as_str(), Some("open" | "attestorKey" | "acceptedCount"))
                    }))
            {
                let next = (index + 1) % 3;
                m.insert("index".into(), json!(next));
                if m.contains_key("path") {
                    m.insert("path".into(), json!([next]));
                }
            }
        })
    });
    included(&layout, ACCEPT);
    for case in ["guard_index", "point_kind", "argument_type", "callee_arity"] {
        let mut c = fixture();
        match case {
            "guard_index" => {
                if let StateAction::Assert {
                    condition: Expr::CellRead { index, .. },
                    ..
                } = &mut target_mut(&mut c, VERIFY).actions[0]
                {
                    *index = 0;
                }
            }
            "point_kind" => {
                c.ledger_fields[0].declaration = LedgerFieldKind::Cell { ty: Type::Boolean }
            }
            "argument_type" => target_mut(&mut c, VERIFY).parameters[0].ty = Type::Boolean,
            "callee_arity" => {
                target_mut(&mut c, "schnorrVerifyDigest").parameters.pop();
            }
            _ => unreachable!(),
        }
        assert!(old(&c, VERIFY).unwrap().is_none(), "{case}");
        assert!(
            crate::render_with_capabilities(&c).is_err(),
            "native must reject {case}"
        );
        assert!(checked(&c, VERIFY).is_err(), "checked must reject {case}");
    }
}

#[test]
fn complete_transitive_helpers_validate_all_branches_and_private_witness_arguments() {
    let mut c = fixture();
    let original = target(&c, LOCAL).actions.clone();
    let mut leaf = target(&c, LOCAL).clone();
    leaf.name = "local_leaf".into();
    leaf.actions = original;
    let arguments = leaf.parameters.iter().map(|p| param(&p.name)).collect();
    c.stateful_circuits.push(leaf);
    local(
        &mut c,
        vec![StateAction::If {
            condition: Expr::Boolean { value: true },
            then: Box::new(StateAction::CircuitCall {
                name: "local_leaf".into(),
                arguments,
            }),
            otherwise: Box::new(StateAction::Sequence { actions: vec![] }),
        }],
    );
    included(&c, ACCEPT);
    // Both branches remain checked even when the condition is statically true.
    if let StateAction::If { otherwise, .. } = &mut target_mut(&mut c, LOCAL).actions[0] {
        **otherwise = StateAction::CircuitCall {
            name: "missing_helper".into(),
            arguments: vec![],
        };
    }
    assert!(matches!(
        old(&c, ACCEPT).unwrap(),
        Some(RecordingOutcome::Unsupported(_))
    ));
    assert!(crate::render_with_capabilities(&c).is_err());
    assert!(checked(&c, ACCEPT).is_err());
    let mut wrong_witness = fixture();
    wrong_witness
        .witnesses
        .iter_mut()
        .find(|w| w.name == "getSchnorrReduction")
        .unwrap()
        .parameters
        .clear();
    assert!(matches!(
        old(&wrong_witness, ACCEPT).unwrap(),
        Some(RecordingOutcome::Unsupported(_))
    ));
    assert!(crate::render_with_capabilities(&wrong_witness).is_err());
}
