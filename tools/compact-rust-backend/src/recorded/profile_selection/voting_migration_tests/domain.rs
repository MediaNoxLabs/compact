// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;

#[test]
fn interface_boundary_is_exact_including_nominal_coin_and_return_form() {
    for mode in 0..9 {
        let mut c = source();
        let r = root_mut(&mut c);
        match mode {
            0 => r.parameters.clear(),
            1 => r.parameters.push(Parameter {
                name: "extra".into(),
                ty: Type::Boolean,
            }),
            2 => r.parameters.swap(0, 1),
            3 => r.parameters[0].ty = Type::Field,
            4 => {
                let Type::Struct { name, .. } = &mut r.parameters[1].ty else {
                    unreachable!()
                };
                *name = "LookalikeCoin".into();
            }
            5 => {
                let Type::Struct { fields, .. } = &mut r.parameters[1].ty else {
                    unreachable!()
                };
                fields[0].ty = Type::Bytes { length: 31 };
            }
            6 => r.result = Type::Field,
            7 => r.return_value = StateReturn::Expression { value: Expr::Unit },
            8 => {
                r.return_value = StateReturn::Effectful {
                    body: ReturnPlan::Value { value: Expr::Unit },
                }
            }
            _ => unreachable!(),
        }
        assert!(
            Context::new(&c).domain(root(&c)).unwrap().is_none(),
            "interface case {mode}"
        );
    }
}

#[test]
fn duplicate_source_formals_are_refused_before_checked_plan_construction() {
    let mut c = source();
    let r = root_mut(&mut c);
    r.parameters[1].name = r.parameters[0].name.clone();
    let maps = Context::new(&c);
    // Source lowering refusals are Unsupported. Invalid is reserved for a plan
    // that violates independent validation after source lowering succeeds.
    assert!(matches!(
        effect_plan::check_with_calls(root(&c), &maps.ledger, &maps.witnesses,
            &maps.pure, &maps.circuits),
        Err(effect_plan::PlanError::Unsupported(gap))
            if gap.path == "parameters[1]" && gap.detail == "duplicate parameter"
    ));
    assert!(maps.domain(root(&c)).unwrap().is_none());
    let row = selected_trace(&c, root(&c));
    assert_ne!(row["body_owner"], "algebra.voting-commit");
    assert!(attempts(&row).contains(&"algebra.guarded-deposit"));
    assert!(!attempts(&row).contains(&"selector.funded-mint"));
}

#[test]
fn compositional_expansion_does_not_require_old_effect_counts() {
    for actions in [vec![], vec![StateAction::Expression { value: Expr::Unit }]] {
        let mut c = source();
        root_mut(&mut c).actions = actions;
        let maps = Context::new(&c);
        assert!(maps.old(root(&c)).is_none(), "explicit new domain coverage");
        assert!(maps.domain(root(&c)).unwrap().is_some());
        public_owner(&c, "vote_commit");
    }
}
