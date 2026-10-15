// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;
fn first_assert(action: &mut StateAction) -> Option<&mut Expr> {
    match action {
        StateAction::Assert { condition, .. } => Some(condition),
        StateAction::Sequence { actions } => actions.iter_mut().find_map(first_assert),
        StateAction::Let { action, .. } => first_assert(action),
        _ => None,
    }
}
#[test]
fn whole_old_guarded_conditions_may_skip_observations_on_a_selected_path() {
    for selected in [false, true] {
        let mut c = source(true);
        let r = circuit_mut(&mut c, "set_topic");
        let condition = r.actions.iter_mut().find_map(first_assert).unwrap();
        *condition = Expr::If {
            condition: Box::new(Expr::Boolean { value: selected }),
            then: Box::new(condition.clone()),
            otherwise: Box::new(Expr::Boolean { value: true }),
        };
        // Syntactic reads remain unchanged; false skips the original price guard.
        // This is an old-admission preservation control, not an application policy recommendation.
        included(&c, "set_topic", &format!("guard-read-lazy-{selected}"));
    }
}
#[test]
fn merge_helper_branch_evidence_is_syntactic_while_outer_flow_stays_fixed() {
    for selected in [false, true] {
        let mut c = source(true);
        let ty = circuit_mut(&mut c, "mergeCoin").result.clone();
        let body = returned_mut(&mut c, "mergeCoin");
        *body = Expr::If {
            condition: Box::new(Expr::Boolean { value: selected }),
            then: Box::new(body.clone()),
            otherwise: Box::new(Expr::Default { ty }),
        };
        included(&c, "set_topic", &format!("merge-body-lazy-{selected}"));
    }
}
