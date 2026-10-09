// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//  	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Witness authorization, enum advancement, and optional-cell lifecycle profiles.

use compact_rust_backend::ir::{Contract, Expr, SCHEMA_VERSION, StateAction, StateReturn};
use compact_rust_backend::{render, render_with_capabilities};

// Limit semantic order checks to the chosen generated API, excluding its native twin.
fn generated_body(source: &str, module_name: &str, function_name: &str) -> String {
    use quote::ToTokens;
    fn find(items: &[syn::Item], module_name: &str, function_name: &str) -> Option<String> {
        for item in items {
            if let syn::Item::Mod(module) = item
                && let Some((_, items)) = &module.content
            {
                if module.ident == module_name {
                    return items.iter().find_map(|item| match item {
                        syn::Item::Fn(function) if function.sig.ident == function_name => {
                            Some(function.block.to_token_stream().to_string())
                        }
                        _ => None,
                    });
                }
                if let Some(body) = find(items, module_name, function_name) {
                    return Some(body);
                }
            }
        }
        None
    }
    find(
        &syn::parse_file(source).unwrap().items,
        module_name,
        function_name,
    )
    .expect("requested generated function")
}

#[test]
fn welcome_organizer_recording_requires_the_closed_witness_hash_guard() {
    let mut contract: Contract =
        serde_json::from_str(include_str!("../welcome-organizer-schema13-ir.json")).unwrap();
    contract.schema_version = SCHEMA_VERSION;
    let statuses = |contract: &Contract| {
        let rendered = render_with_capabilities(contract).unwrap();
        rendered
            .capabilities
            .circuits
            .into_iter()
            .map(|capability| {
                (
                    capability.name,
                    capability.recorded,
                    capability.recording_unavailable.map(|gap| gap.path),
                )
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(
        statuses(&contract),
        vec![
            ("add_participant".to_owned(), true, None),
            ("add_organizer".to_owned(), true, None),
            ("check_in".to_owned(), true, None),
        ]
    );

    let hash = contract
        .circuits
        .iter_mut()
        .find(|circuit| circuit.name == "public_key")
        .unwrap();
    hash.body = Expr::Parameter { name: "sk".into() };
    assert_eq!(
        statuses(&contract),
        vec![
            (
                "add_participant".to_owned(),
                false,
                Some("actions[0]".to_owned())
            ),
            (
                "add_organizer".to_owned(),
                false,
                Some("actions[0]".to_owned())
            ),
            ("check_in".to_owned(), true, None),
        ]
    );

    let hash = contract
        .circuits
        .iter_mut()
        .find(|circuit| circuit.name == "public_key")
        .unwrap();
    let original: Contract =
        serde_json::from_str(include_str!("../welcome-organizer-schema13-ir.json")).unwrap();
    hash.body = original
        .circuits
        .iter()
        .find(|circuit| circuit.name == "public_key")
        .unwrap()
        .body
        .clone();
    let helper = contract
        .stateful_circuits
        .iter_mut()
        .find(|circuit| circuit.name == "local_sk_or_error")
        .unwrap();
    let StateReturn::Expression {
        value: Expr::Let { body, .. },
    } = &mut helper.return_value
    else {
        unreachable!()
    };
    let Expr::Sequence { steps, .. } = body.as_mut() else {
        unreachable!()
    };
    steps.push(Expr::Assert {
        condition: Box::new(Expr::Boolean { value: true }),
        message: "extra assertion".into(),
    });
    assert_eq!(
        statuses(&contract),
        vec![
            (
                "add_participant".to_owned(),
                false,
                Some("actions[0]".to_owned())
            ),
            (
                "add_organizer".to_owned(),
                false,
                Some("actions[0]".to_owned())
            ),
            ("check_in".to_owned(), true, None),
        ]
    );
}

#[test]
fn authorized_optional_cell_write_is_structural_and_fails_closed() {
    let source = include_str!("../election-schema13-ir.json");
    let mut contract: Contract = serde_json::from_str(source).unwrap();
    contract.schema_version = SCHEMA_VERSION;
    let available = |contract: &Contract| {
        render_with_capabilities(contract)
            .unwrap()
            .capabilities
            .circuits
            .into_iter()
            .find(|circuit| circuit.name == "set_topic")
            .unwrap()
            .recorded
    };
    assert!(available(&contract));
    // No circuit, witness, formal, struct, field or enum spelling is an allowlist.
    let renamed = source
        .replace("set_topic", "configure")
        .replace("private$secret_key", "load_credential")
        .replace("public_key", "derive_key")
        .replace("authority", "controller")
        .replace("PublicState", "Phase")
        .replace("setup", "draft")
        .replace("Maybe", "OptionalText")
        .replace("is_some", "present")
        .replace("\"sk\"", "\"credential\"")
        .replace("\"apk\"", "\"digest\"");
    let mut renamed: Contract = serde_json::from_str(&renamed).unwrap();
    renamed.schema_version = SCHEMA_VERSION;
    assert!(
        render_with_capabilities(&renamed)
            .unwrap()
            .capabilities
            .circuits
            .iter()
            .find(|circuit| circuit.name == "configure")
            .unwrap()
            .recorded
    );
    let mut wrong_hash = contract.clone();
    let hash = wrong_hash
        .circuits
        .iter_mut()
        .find(|circuit| circuit.name == "public_key")
        .unwrap();
    let Expr::PersistentHash { value } = &mut hash.body else {
        unreachable!()
    };
    let Expr::Tuple { elements } = value.as_mut() else {
        unreachable!()
    };
    elements[1] = Expr::BytesLiteral { bytes: vec![0; 32] };
    assert!(!available(&wrong_hash));
    let actions = |contract: &mut Contract| {
        let circuit = contract
            .stateful_circuits
            .iter_mut()
            .find(|circuit| circuit.name == "set_topic")
            .unwrap();
        let StateAction::Let { action, .. } = &mut circuit.actions[0] else {
            unreachable!()
        };
        let StateAction::Let { action, .. } = action.as_mut() else {
            unreachable!()
        };
        let StateAction::Sequence { actions } = action.as_mut() else {
            unreachable!()
        };
        actions.swap(0, 1);
    };
    let mut reordered = contract.clone();
    actions(&mut reordered);
    assert!(!available(&reordered));
    let mut wrong_index = contract;
    let field = wrong_index
        .ledger_fields
        .iter_mut()
        .find(|field| field.id == "authority")
        .unwrap();
    field.index = 8;
    assert!(render_with_capabilities(&wrong_index).is_err());
}

#[test]
fn enum_advance_composes_certified_successors_and_preserves_source_read_order() {
    let source = include_str!("../election-schema13-ir.json");
    let mut contract: Contract = serde_json::from_str(source).unwrap();
    contract.schema_version = SCHEMA_VERSION;
    let available = |contract: &Contract, name: &str| {
        render_with_capabilities(contract)
            .unwrap()
            .capabilities
            .circuits
            .iter()
            .find(|circuit| circuit.name == name)
            .unwrap()
            .recorded
    };
    assert!(available(&contract, "advance"));
    let renamed = source
        .replace("advance", "progress")
        .replace("successor", "next_phase")
        .replace("private$secret_key", "load_credential")
        .replace("public_key", "derive_key")
        .replace("authority", "controller")
        .replace("topic", "subject")
        .replace("PublicState", "Phase")
        .replace("setup", "draft")
        .replace("Maybe", "OptionalText")
        .replace("is_some", "present")
        .replace("\"sk\"", "\"credential\"")
        .replace("\"apk\"", "\"digest\"");
    let mut renamed: Contract = serde_json::from_str(&renamed).unwrap();
    renamed.schema_version = SCHEMA_VERSION;
    assert!(available(&renamed, "progress"));
    let mut guarded_successor = contract.clone();
    let helper = guarded_successor
        .circuits
        .iter_mut()
        .find(|circuit| circuit.name == "successor")
        .unwrap();
    helper.body = Expr::Sequence {
        steps: vec![Expr::Assert {
            condition: Box::new(Expr::Boolean { value: true }),
            message: "extra".into(),
        }],
        value: Box::new(helper.body.clone()),
    };
    // A pure assertion is a valid certified helper body; it need not match the
    // former closed successor recognizer once the owned topic Cell is supported.
    assert!(available(&guarded_successor, "advance"));
    let rendered = render_with_capabilities(&guarded_successor).unwrap();
    let helper_body = generated_body(&rendered.source, "pure_circuits", "successor");
    assert!(helper_body.contains("AssertionFailed") && helper_body.contains("\"extra\""));
    let recorded = generated_body(&rendered.source, "recorded", "advance");
    assert!(recorded.contains("pure_circuits :: successor"));
    assert!(
        recorded.find("authority . record_read").unwrap()
            < recorded.find("topic . record_read").unwrap()
    );
    assert!(
        recorded.find("topic . record_read").unwrap()
            < recorded.find("state . record_write").unwrap()
    );

    // Actual state effects remain invalid inside a declared pure helper.
    let mut impure_successor = guarded_successor.clone();
    let helper = impure_successor
        .circuits
        .iter_mut()
        .find(|c| c.name == "successor")
        .unwrap();
    let Expr::Sequence { steps, .. } = &mut helper.body else {
        unreachable!()
    };
    steps.push(Expr::CellRead {
        field: "topic".into(),
        index: 2,
    });
    assert!(render_with_capabilities(&impure_successor).is_err());
    let mut reordered = contract.clone();
    let circuit = reordered
        .stateful_circuits
        .iter_mut()
        .find(|circuit| circuit.name == "advance")
        .unwrap();
    let StateAction::Let { action, .. } = &mut circuit.actions[0] else {
        unreachable!()
    };
    let StateAction::Let { action, .. } = action.as_mut() else {
        unreachable!()
    };
    let StateAction::Sequence { actions } = action.as_mut() else {
        unreachable!()
    };
    actions.swap(0, 1);
    // Reordering two valid assertions changes contract behavior, not recording
    // eligibility. The recorded function must retain that changed read order.
    assert!(available(&reordered, "advance"));
    let rendered = render_with_capabilities(&reordered).unwrap();
    let recorded = generated_body(&rendered.source, "recorded", "advance");
    assert!(
        recorded.find("topic . record_read").unwrap()
            < recorded.find("authority . record_read").unwrap()
    );
    assert!(
        recorded.find("authority . record_read").unwrap()
            < recorded.find("state . record_write").unwrap()
    );
    let mut wrong_index = contract;
    let field = wrong_index
        .ledger_fields
        .iter_mut()
        .find(|field| field.id == "topic")
        .unwrap();
    field.index = 8;
    assert!(render_with_capabilities(&wrong_index).is_err());
}

#[test]
fn typed_cell_lifecycle_retains_only_final_root_scope_and_audits_helpers() {
    let source = include_str!("../bboard-schema13-ir.json");
    let load = |source: &str| {
        let mut contract: Contract = serde_json::from_str(source).unwrap();
        contract.schema_version = SCHEMA_VERSION;
        contract
    };
    let available = |contract: &Contract| {
        render_with_capabilities(contract)
            .unwrap()
            .capabilities
            .circuits
            .iter()
            .map(|c| c.recorded)
            .collect::<Vec<_>>()
    };
    let contract = load(source);
    assert_eq!(available(&contract), [true, true]);
    let renamed = load(
        &source
            .replace("post", "publish")
            .replace("take_down", "remove_message")
            .replace("former_msg", "saved_content")
            .replace("public_key", "derive_owner")
            .replace("instance", "generation"),
    );
    assert_eq!(available(&renamed), [true, true]);
    let mut effectful = contract.clone();
    let helper = effectful
        .circuits
        .iter_mut()
        .find(|c| c.name == "public_key")
        .unwrap();
    helper.body = Expr::Sequence {
        steps: vec![Expr::Assert {
            condition: Box::new(Expr::Boolean { value: true }),
            message: "extra effect".into(),
        }],
        value: Box::new(helper.body.clone()),
    };
    assert_eq!(available(&effectful), [false, false]);
    let mut nested = contract.clone();
    let action = nested.stateful_circuits[1].actions.pop().unwrap();
    nested.stateful_circuits[1]
        .actions
        .push(StateAction::Sequence {
            actions: vec![action],
        });
    // The shared continuation adapter preserves the same terminal lexical
    // scope as native lowering, without changing this profile's effects.
    assert_eq!(available(&nested), [true, true]);
    let mut earlier = contract.clone();
    earlier.stateful_circuits[1]
        .actions
        .push(StateAction::Assert {
            condition: Expr::Boolean { value: true },
            message: "after scoped binding".into(),
        });
    let error = render(&earlier).unwrap_err();
    assert!(format!("{error:?}").contains("former_msg"), "{error:?}");
    let mut wrong_slot = contract;
    wrong_slot
        .ledger_fields
        .iter_mut()
        .find(|f| f.id == "instance")
        .unwrap()
        .index = 3;
    assert!(render(&wrong_slot).is_err());
}
