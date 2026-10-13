// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;

#[test]
fn whole_final_gates_native_errors_and_resource_exclusions_are_not_prefix_admission() {
    let empty = unit(vec![]);
    assert!(matches!(
        Context::new(&empty).old(&empty, "root"),
        Ok(RecordingOutcome::Unsupported(_))
    ));
    assert!(Context::new(&empty).checked("root").is_ok());
    assert!(
        crate::render_with_capabilities(&empty)
            .unwrap()
            .capabilities
            .circuits[0]
            .recorded
    );
    let mut opaque = cell_contract();
    opaque.stateful_circuits[0].parameters = vec![parameter("unused", Type::OpaqueString)];
    assert!(matches!(
        Context::new(&opaque).old(&opaque, "root"),
        Ok(RecordingOutcome::Unsupported(_))
    ));
    assert!(Context::new(&opaque).checked("root").is_ok());
    assert!(
        crate::render_with_capabilities(&opaque)
            .unwrap()
            .capabilities
            .circuits[0]
            .recorded
    );
    let mut internal = cell_contract();
    internal.stateful_circuits[0].internal = true;
    assert!(matches!(
        Context::new(&internal).old(&internal, "root"),
        Ok(RecordingOutcome::Unsupported(_))
    ));
    let cx = Context::new(&internal);
    assert!(
        generic_reference::render_recorded_helper(
            &internal.stateful_circuits[0],
            &cx.ledger,
            &cx.witnesses,
            &cx.pure,
            &cx.circuits,
            &HashSet::new()
        )
        .unwrap()
        .is_supported()
    );
    let mut malformed = cell_contract();
    malformed.stateful_circuits[0].name = "not-a-rust-name".into();
    assert!(crate::render_with_capabilities(&malformed).is_err());
    let mut wrong = cell_contract();
    wrong.ledger_fields[0].declaration = LedgerFieldKind::Counter;
    assert!(crate::render_with_capabilities(&wrong).is_err());
    let mut duplicate = cell_contract();
    duplicate.stateful_circuits[0].parameters =
        vec![parameter("x", Type::Field), parameter("x", Type::Field)];
    assert!(crate::render_with_capabilities(&duplicate).is_err());
    // Existing source-qualified unsupported Fold remains an old refusal, not a
    // prerequisite manufactured from an arbitrary syntax node.
    let mut unsupported = cell_contract();
    test_support::register_unsupported_fold(&mut unsupported);
    let folded = test_support::unsupported_fixed_input();
    unsupported.stateful_circuits[0].actions = vec![StateAction::CellWrite {
        field: "cell".into(),
        index: 0,
        value: folded,
    }];
    assert!(matches!(
        Context::new(&unsupported).old(&unsupported, "root"),
        Ok(RecordingOutcome::Unsupported(_))
    ));
    assert!(matches!(
        Context::new(&unsupported).checked("root"),
        Err(effect_plan::PlanError::Unsupported(_))
    ));
    let rendered = crate::render_with_capabilities(&unsupported).unwrap();
    assert!(!rendered.capabilities.circuits[0].recorded);
    // Policy exclusion: an unused huge canonical public parameter can pass the
    // old syntax emitter. It is explicitly outside the checked layout budget.
    // Native rendering is not a claim that this fixture was frontend generated.
    let mut large = cell_contract();
    large.stateful_circuits[0].parameters = vec![parameter(
        "large",
        Type::Vector {
            length: 32769,
            element: Box::new(Type::Field),
        },
    )];
    assert!(
        Context::new(&large)
            .old(&large, "root")
            .unwrap()
            .is_supported()
    );
    let Err(effect_plan::PlanError::Unsupported(gap)) = Context::new(&large).checked("root") else {
        panic!("fixed layout must refuse")
    };
    assert!(gap.resource.is_some());
    crate::render_with_capabilities(&large).unwrap();
}

#[test]
fn complete_pipeline_preserves_read_before_write_and_independent_reference_authority() {
    let mut c = cell_contract();
    c.stateful_circuits[0].parameters = vec![parameter("amount", Type::Field)];
    c.stateful_circuits[0].actions = vec![StateAction::Let {
        bindings: vec![binding(
            "amount",
            Type::Field,
            Expr::CellRead {
                field: "cell".into(),
                index: 0,
            },
        )],
        action: Box::new(StateAction::Sequence {
            actions: vec![
                StateAction::CellWrite {
                    field: "cell".into(),
                    index: 0,
                    value: var("amount"),
                },
                assertion(),
            ],
        }),
    }];
    let plan = included(&c, "root", "lexical shadow and ordered body");
    let calls = methods(&plan);
    assert!(
        calls.iter().position(|m| m == "record_read").unwrap()
            < calls.iter().position(|m| m == "record_write").unwrap()
    );
    // The independent oracle cannot run the checked lowerer; source authentication
    // additionally reconstructs its entire signed function and copied helper planner.
    let reference = include_str!("../generic_reference.rs");
    assert!(!reference.contains("profile_selection::select("));
    assert!(!reference.contains("effect_plan::lower_with_calls"));
    assert!(reference.contains("fn plan_recorded_helpers("));
    // The ordinary recursive and cumulative-work workers are separate maintained
    // gates: this law does not invent another recursion limit or stack size.
}
