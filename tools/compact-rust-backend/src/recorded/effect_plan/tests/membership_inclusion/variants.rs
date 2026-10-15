// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use super::*;

#[test]
fn old_membership_is_invariant_under_formal_and_ledger_renaming() {
    let c = renamed(
        &bboard(),
        &[
            ("new_message", "incoming_text"),
            ("post", "publish_message"),
            ("take_down", "clear_message"),
        ],
    );
    included(
        &c,
        "publish_message",
        "optional-renamed-input",
        "cell_lifecycle/input",
    );
    included(
        &c,
        "clear_message",
        "optional-renamed-output",
        "cell_lifecycle/output",
    );
    let c = renamed(
        &families::field_cell(),
        &[
            ("total", "running_total"),
            ("amount", "incoming"),
            ("before", "previous"),
        ],
    );
    included(&c, "update", "field-renamed-authority", "field_cell_root");
}

#[test]
fn old_membership_preserves_sequences_and_shadowed_owned_formals() {
    let mut c = bboard();
    let root = root_mut(&mut c, "post");
    root.actions = vec![StateAction::Sequence {
        actions: root.actions.clone(),
    }];
    included(&c, "post", "optional-sequence", "cell_lifecycle/input");
    let root = root_mut(&mut c, "post");
    let parameter = root.parameters[0].clone();
    root.actions = vec![StateAction::Let {
        bindings: vec![LocalBinding {
            name: parameter.name.clone(),
            ty: parameter.ty,
            value: Expr::Parameter {
                name: parameter.name,
            },
        }],
        action: Box::new(StateAction::Sequence {
            actions: root.actions.clone(),
        }),
    }];
    included(&c, "post", "optional-shadow-formal", "cell_lifecycle/input");
}

#[test]
fn old_counter_comparison_checks_selected_and_unselected_branch_composition() {
    for selected in [false, true] {
        let mut c = counter();
        let root = root_mut(&mut c, "checked");
        let actions = root.actions.clone();
        root.actions = vec![StateAction::If {
            condition: Expr::Boolean { value: selected },
            then: Box::new(StateAction::Sequence { actions }),
            otherwise: Box::new(StateAction::Sequence { actions: vec![] }),
        }];
        included(
            &c,
            "checked",
            &format!("comparison-selected-{selected}"),
            "counter_comparison",
        );
    }
    let mut c = counter();
    let root = root_mut(&mut c, "compare");
    root.parameters.extend([
        Parameter {
            name: "unused_gate".into(),
            ty: Type::Boolean,
        },
        Parameter {
            name: "unused_limit".into(),
            ty: validate::uint("18446744073709551615"),
        },
    ]);
    included(
        &c,
        "compare",
        "comparison-mixed-interface",
        "counter_comparison",
    );
}

#[test]
fn old_internal_roots_keep_the_same_parameter_and_result_domains() {
    for (mut c, name, label, family) in [
        (
            bboard(),
            "post",
            "internal-opaque-unit",
            "cell_lifecycle/input",
        ),
        (
            counter(),
            "compare",
            "internal-u64-boolean",
            "counter_comparison",
        ),
        (
            assertions(),
            "checked",
            "internal-boolean-u64",
            "assertion_entry/Uint64",
        ),
    ] {
        root_mut(&mut c, name).internal = true;
        included(&c, name, label, family);
    }
}

#[test]
fn old_read_only_assertions_keep_all_boolean_interfaces_and_uint8_callback_arguments() {
    let mut c = assertions();
    let root = root_mut(&mut c, "checked");
    root.parameters.push(Parameter {
        name: "unused_gate".into(),
        ty: Type::Boolean,
    });
    included(
        &c,
        "checked",
        "assertion-multiple-booleans",
        "assertion_entry/Uint64",
    );
    let c = renamed(
        &c,
        &[
            ("next_gate", "check_policy"),
            ("tag", "ordinal"),
            ("selected", "enabled"),
        ],
    );
    included(
        &c,
        "checked",
        "assertion-renamed-callback",
        "assertion_entry/Uint64",
    );
}

fn helper_case(result: Json, body: Json, extra: Vec<Json>) -> Contract {
    let mut c = serde_json::to_value(bboard()).unwrap();
    c["circuits"].as_array_mut().unwrap().extend(extra);
    c["circuits"]
        .as_array_mut()
        .unwrap()
        .push(json!({"name":"audit_helper","parameters":[],"result":result,"body":body}));
    let root = c["stateful_circuits"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|r| r["name"] == "post")
        .unwrap();
    root["actions"] = json!([{"kind":"let","bindings":[{"name":"audit_local","ty":result,"value":{"kind":"call","name":"audit_helper","arguments":[]}}],"action":{"kind":"sequence","actions":root["actions"]}}]);
    serde_json::from_value(c).unwrap()
}
#[test]
fn old_pure_default_and_vector_fold_helpers_are_certified_without_shape_inlining() {
    let string = json!({"kind":"opaque_string"});
    let c = helper_case(
        string.clone(),
        json!({"kind":"default","ty":string}),
        vec![],
    );
    included(
        &c,
        "post",
        "pure-default-string",
        "cell_lifecycle/pure_helper",
    );
    let field = json!({"kind":"field"});
    let literal = json!({"kind":"field_literal","value":"1"});
    let c = helper_case(
        field.clone(),
        json!({"kind":"vector_fold_call","name":"take_acc","initial":literal,"source":{"kind":"coerce","ty":{"kind":"vector","element":field,"length":2},"value":{"kind":"tuple","elements":[literal,literal]}},"accumulator":field,"element":field,"length":2}),
        vec![
            json!({"name":"take_acc","parameters":[{"name":"acc","ty":field},{"name":"item","ty":field}],"result":field,"body":{"kind":"parameter","name":"acc"}}),
        ],
    );
    included(&c, "post", "pure-vector-fold", "cell_lifecycle/pure_helper");
}
