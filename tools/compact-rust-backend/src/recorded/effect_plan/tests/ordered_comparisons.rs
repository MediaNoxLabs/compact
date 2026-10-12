// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Whole-source comparison premises: old admission, checked gaps and actual owners.
use super::*;
use crate::recorded::admission_probe::trace;
use crate::recorded::profile_attempt::ProfileAttempt;
use serde_json::json;

#[path = "ordered_comparisons/probe.rs"]
mod probe;
#[path = "ordered_comparisons/source.rs"]
mod source;
#[path = "ordered_comparisons/typed.rs"]
mod typed;

fn inspect(source: &str, entry: &str, label: &str, require_old: bool) {
    let contract: Contract = serde_json::from_str(source).unwrap();
    let ledger = contract
        .ledger_fields
        .iter()
        .map(|v| (v.id.as_str(), v))
        .collect();
    let witnesses = contract
        .witnesses
        .iter()
        .map(|v| (v.name.as_str(), v))
        .collect();
    let pure = contract
        .circuits
        .iter()
        .map(|v| (v.name.as_str(), v))
        .collect();
    let stateful: HashMap<_, _> = contract
        .stateful_circuits
        .iter()
        .map(|v| (v.name.as_str(), v))
        .collect();
    let root = stateful[entry];
    // The complete historical premise is evaluated BEFORE the new candidate.
    let old = crate::recorded::typed_plan::lower_unit_composition_checked(
        root, &ledger, &witnesses, &pure, &stateful,
    );
    let old_status = match old {
        ProfileAttempt::Admitted(_) => "admitted",
        ProfileAttempt::NotApplicable => "not_applicable",
        ProfileAttempt::Rejected(_) => "rejected",
    };
    if require_old {
        assert_eq!(old_status, "admitted", "{label}: complete old admission");
    }
    let checked = match check_with_calls(root, &ledger, &witnesses, &pure, &stateful) {
        Ok(plan) => {
            if require_old {
                assert!(
                    super::retained_schnorr::remaining_map_member_path(label).is_none(),
                    "{label}: update the measured MapMember gap deliberately"
                );
            }
            render(&plan).unwrap();
            json!({"status":"supported"})
        }
        Err(PlanError::Unsupported(gap)) => {
            assert!(
                require_old,
                "{label}: genuine ordering source must compose: {gap:?}"
            );
            super::retained_schnorr::assert_map_member_gap(label, &gap);
            json!({"status":"unsupported", "ir_node":gap.ir_node,
                "path":gap.path, "detail":gap.detail})
        }
        Err(PlanError::Invalid(gap)) => panic!("{label}: invariant failure {gap:?}"),
    };
    probe::export(
        source,
        &contract,
        root,
        label,
        require_old,
        old_status,
        checked,
    );
}
