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

//! Witness-admitted membership, reveal, historic spend, and signature profiles.

use super::lifecycle_recording::generated_body;
use compact_rust_backend::ir::{
    Contract, CounterAmount, Expr, LedgerField, LedgerFieldKind, SCHEMA_VERSION, StateAction, Type,
};
use compact_rust_backend::{RenderError, render, render_with_capabilities};

#[test]
fn witness_merkle_composition_keeps_types_and_source_order_without_profile_shape_gates() {
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
    assert!(available(&contract, "add_voter"));
    let renamed = source
        .replace("add_voter", "admit_member")
        .replace("eligible_voters", "registry")
        .replace("private$secret_key", "load_credential")
        .replace("public_key", "derive_key")
        .replace("authority", "controller")
        .replace("PublicState", "Phase")
        .replace("setup", "draft")
        .replace("Maybe", "OptionalValue")
        .replace("is_some", "present");
    let mut renamed: Contract = serde_json::from_str(&renamed).unwrap();
    renamed.schema_version = SCHEMA_VERSION;
    assert!(available(&renamed, "admit_member"));
    let mut literal_argument = contract.clone();
    let circuit = literal_argument
        .stateful_circuits
        .iter_mut()
        .find(|circuit| circuit.name == "add_voter")
        .unwrap();
    let StateAction::Sequence { actions } = &mut circuit.actions[0] else {
        unreachable!()
    };
    let StateAction::Assert {
        condition: Expr::If { condition, .. },
        ..
    } = &mut actions[0]
    else {
        unreachable!()
    };
    let Expr::StructField { value, .. } = condition.as_mut() else {
        unreachable!()
    };
    let Expr::WitnessCall { arguments, .. } = value.as_mut() else {
        unreachable!()
    };
    arguments[0] = Expr::Coerce {
        value: Box::new(Expr::BytesLiteral { bytes: vec![0; 32] }),
        ty: Type::Bytes { length: 32 },
    };
    // A declared Bytes32 literal is a valid witness argument. The old organizer
    // required the exact circuit parameter, a provenance shape the source need not use.
    assert!(available(&literal_argument, "add_voter"));
    let mut malformed_argument = literal_argument.clone();
    let circuit = malformed_argument
        .stateful_circuits
        .iter_mut()
        .find(|c| c.name == "add_voter")
        .unwrap();
    let StateAction::Sequence { actions } = &mut circuit.actions[0] else {
        unreachable!()
    };
    let StateAction::Assert {
        condition: Expr::If { condition, .. },
        ..
    } = &mut actions[0]
    else {
        unreachable!()
    };
    let Expr::StructField { value, .. } = condition.as_mut() else {
        unreachable!()
    };
    let Expr::WitnessCall { arguments, .. } = value.as_mut() else {
        unreachable!()
    };
    arguments[0] = Expr::Boolean { value: false };
    assert!(render_with_capabilities(&malformed_argument).is_err());
    let mut reordered = contract.clone();
    let circuit = reordered
        .stateful_circuits
        .iter_mut()
        .find(|circuit| circuit.name == "add_voter")
        .unwrap();
    let StateAction::Sequence { actions } = &mut circuit.actions[0] else {
        unreachable!()
    };
    actions.swap(0, 1);
    // Reordering these well-typed source actions changes when checks run. The
    // compiler records that source order instead of imposing the organizer recipe.
    assert!(available(&reordered, "add_voter"));
    let mut wrong_path = contract.clone();
    wrong_path
        .ledger_fields
        .iter_mut()
        .find(|f| f.id == "eligible_voters")
        .unwrap()
        .path = vec![6, 99];
    assert!(render_with_capabilities(&wrong_path).is_err());
    let mut changed_depth = contract;
    let field = changed_depth
        .ledger_fields
        .iter_mut()
        .find(|field| field.id == "eligible_voters")
        .unwrap();
    let LedgerFieldKind::MerkleTree { depth, .. } = &mut field.declaration else {
        unreachable!()
    };
    *depth = 9;
    // add_voter observes only the witness presence bit, not its path. A valid
    // tree depth is independent here; other path-consuming exports may reject
    // the whole source first. This is no proof of membership for a changed path.
    if let Ok(rendered) = render_with_capabilities(&changed_depth) {
        assert!(
            rendered
                .capabilities
                .circuits
                .iter()
                .find(|circuit| circuit.name == "add_voter")
                .unwrap()
                .recorded
        );
    }
    let LedgerFieldKind::MerkleTree { depth, .. } = &mut changed_depth
        .ledger_fields
        .iter_mut()
        .find(|f| f.id == "eligible_voters")
        .unwrap()
        .declaration
    else {
        unreachable!()
    };
    *depth = 1;
    assert!(render_with_capabilities(&changed_depth).is_err());
}

#[test]
fn composite_witness_and_pure_struct_hash_recording_remains_typed_and_closed() {
    let source = include_str!("../zerocash-schema13-ir.json");
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
    assert!(available(&contract, "zerocash_mint"));
    assert!(available(&contract, "spend"));
    let renamed = source
        .replace("zerocash_mint", "create_item")
        .replace("commitment_from_coin_info", "derive_item")
        .replace("context$new_coin_info", "load_item")
        .replace("private$add_coin", "save_item")
        .replace("coin_info", "ItemData");
    let mut renamed: Contract = serde_json::from_str(&renamed).unwrap();
    renamed.schema_version = SCHEMA_VERSION;
    assert!(available(&renamed, "create_item"));
    // A certified pure branch now composes with the typed Merkle insertion.
    let mut branch = contract.clone();
    let helper = branch
        .circuits
        .iter_mut()
        .find(|circuit| circuit.name == "commitment_from_coin_info")
        .unwrap();
    helper.body = Expr::If {
        condition: Box::new(Expr::Boolean { value: true }),
        then: Box::new(helper.body.clone()),
        otherwise: Box::new(helper.body.clone()),
    };
    assert!(available(&branch, "zerocash_mint"));
    let mut wrong_witness = contract.clone();
    wrong_witness
        .witnesses
        .iter_mut()
        .find(|witness| witness.name == "context$new_coin_info")
        .unwrap()
        .result = Type::Field;
    assert!(render_with_capabilities(&wrong_witness).is_err());
    let mut wrong_slot = contract;
    wrong_slot
        .ledger_fields
        .iter_mut()
        .find(|field| field.id == "commitments")
        .unwrap()
        .index = 7;
    assert!(render_with_capabilities(&wrong_slot).is_err());
}

#[test]
fn typed_membership_composes_certified_pure_guards_within_its_interface_domain() {
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
    assert!(available(&contract, "vote$commit"));
    assert!(available(&contract, "vote$reveal"));
    let renamed = source
        .replace("vote$commit", "submit_ballot")
        .replace("vote$reveal", "open_ballot")
        .replace("tally_yes", "accepted")
        .replace("tally_no", "rejected")
        .replace("private$vote$record", "store_ballot")
        .replace("commitment_nullifier", "unique_vote")
        .replace("public_key", "voter_key")
        .replace("ballot_repr", "encode_vote")
        .replace("commit_with_sk", "seal_vote")
        .replace("eligible_voters", "membership")
        .replace("committed_votes", "ballots");
    let mut renamed: Contract = serde_json::from_str(&renamed).unwrap();
    renamed.schema_version = SCHEMA_VERSION;
    assert!(available(&renamed, "submit_ballot"));
    assert!(available(&renamed, "open_ballot"));
    let mut effectful = contract.clone();
    let helper = effectful
        .circuits
        .iter_mut()
        .find(|circuit| circuit.name == "ballot_repr")
        .unwrap();
    helper.body = Expr::Sequence {
        steps: vec![Expr::Assert {
            condition: Box::new(Expr::Boolean { value: true }),
            message: "extra guard".into(),
        }],
        value: Box::new(helper.body.clone()),
    };
    // The explicit membership interface admits this enum root; a typed pure
    // assertion composes without matching the former commitment recipe.
    assert!(available(&effectful, "vote$commit"));
    let rendered = render_with_capabilities(&effectful).unwrap();
    let helper = generated_body(&rendered.source, "pure_circuits", "ballot_repr");
    assert!(helper.contains("AssertionFailed") && helper.contains("extra guard"));
    let commit = generated_body(&rendered.source, "recorded", "vote_commit");
    assert!(
        commit.find("pure_circuits :: ballot_repr").unwrap()
            < commit.find("record_insert").unwrap()
    );
    // The zero-argument reveal root can compose a certified pure assertion.
    // Whole-callee typing and cycle checks still apply to the entire helper body.
    assert!(available(&effectful, "vote$reveal"));
    let mut ill_typed_guard = effectful.clone();
    let helper = ill_typed_guard
        .circuits
        .iter_mut()
        .find(|c| c.name == "ballot_repr")
        .unwrap();
    let Expr::Sequence { steps, .. } = &mut helper.body else {
        unreachable!()
    };
    let Expr::Assert { condition, .. } = &mut steps[0] else {
        unreachable!()
    };
    **condition = Expr::FieldLiteral { value: "1".into() };
    assert!(render_with_capabilities(&ill_typed_guard).is_err());
    let mut recursive = contract.clone();
    let helper = recursive
        .circuits
        .iter_mut()
        .find(|circuit| circuit.name == "merkleTreePathEntryRoot")
        .unwrap();
    helper.body = Expr::Call {
        name: helper.name.clone(),
        arguments: helper
            .parameters
            .iter()
            .map(|parameter| Expr::Parameter {
                name: parameter.name.clone(),
            })
            .collect(),
    };
    // Recursive pure helpers are refused instead of publishing capabilities.
    let expected = RenderError::Located {
        location: helper.source.clone().unwrap(),
        error: Box::new(RenderError::RecursivePureCall(helper.name.clone())),
    };
    assert_eq!(
        render_with_capabilities(&recursive).err().unwrap(),
        expected
    );
    assert_eq!(render(&recursive).unwrap_err(), expected);
    let mut wrong_slot = contract;
    wrong_slot
        .ledger_fields
        .iter_mut()
        .find(|field| field.id == "eligible_voters")
        .unwrap()
        .index = 8;
    assert!(render_with_capabilities(&wrong_slot).is_err());
}

#[test]
fn typed_reveal_composes_counter_decrements_and_rejects_invalid_operands_and_slots() {
    fn available(value: &serde_json::Value) -> bool {
        let mut contract: Contract = serde_json::from_value(value.clone()).unwrap();
        contract.schema_version = SCHEMA_VERSION;
        render_with_capabilities(&contract)
            .unwrap()
            .capabilities
            .circuits
            .iter()
            .find(|c| c.name == "vote$reveal")
            .unwrap()
            .recorded
    }
    fn replace_kind(value: &mut serde_json::Value, from: &str, to: &str) {
        match value {
            serde_json::Value::Object(map) => {
                if map.get("kind").and_then(|v| v.as_str()) == Some(from) {
                    map.insert("kind".into(), serde_json::Value::String(to.into()));
                }
                for value in map.values_mut() {
                    replace_kind(value, from, to);
                }
            }
            serde_json::Value::Array(values) => {
                for value in values {
                    replace_kind(value, from, to);
                }
            }
            _ => {}
        }
    }
    let source: serde_json::Value =
        serde_json::from_str(include_str!("../election-schema13-ir.json")).unwrap();
    assert!(available(&source));
    let mut decrement = source.clone();
    let reveal = decrement["stateful_circuits"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|c| c["name"] == "vote$reveal")
        .unwrap();
    replace_kind(
        &mut reveal["actions"],
        "counter_increment",
        "counter_decrement",
    );
    // Counter decrement is an existing checked effect. Its source semantics
    // (including possible runtime underflow) do not depend on a tally profile.
    assert!(available(&decrement));
    let mut missing_operand: Contract = serde_json::from_value(decrement).unwrap();
    missing_operand.schema_version = SCHEMA_VERSION;
    missing_operand
        .stateful_circuits
        .iter_mut()
        .find(|c| c.name == "vote$reveal")
        .unwrap()
        .actions
        .insert(
            0,
            StateAction::CounterDecrement {
                field: "tally_yes".into(),
                index: 3,
                amount: CounterAmount::Parameter {
                    name: "missing_amount".into(),
                },
            },
        );
    assert!(render_with_capabilities(&missing_operand).is_err());
    let mut wrong_slot: Contract = serde_json::from_value(source).unwrap();
    wrong_slot.schema_version = SCHEMA_VERSION;
    wrong_slot
        .ledger_fields
        .iter_mut()
        .find(|f| f.id == "tally_yes")
        .unwrap()
        .index = 4;
    assert!(render_with_capabilities(&wrong_slot).is_err());
}

#[test]
fn typed_historic_spend_audits_path_helpers_and_slot_provenance() {
    let source = include_str!("../zerocash-schema13-ir.json");
    let load = |source: &str| {
        let mut c: Contract = serde_json::from_str(source).unwrap();
        c.schema_version = SCHEMA_VERSION;
        c
    };
    let available = |c: &Contract, name: &str| {
        render_with_capabilities(c)
            .unwrap()
            .capabilities
            .circuits
            .iter()
            .find(|c| c.name == name)
            .unwrap()
            .recorded
    };
    let contract = load(source);
    assert!(available(&contract, "spend"));
    let renamed = load(
        &source
            .replace("spend", "transfer")
            .replace("ciphertexts", "envelopes")
            .replace("commitments", "notes")
            .replace("nullifiers", "used_notes")
            .replace("context$encrypt", "seal"),
    );
    assert!(available(&renamed, "transfer"));
    let mut effectful = contract.clone();
    let helper = effectful
        .circuits
        .iter_mut()
        .find(|c| c.name == "merkleTreePathEntryRoot")
        .unwrap();
    helper.body = Expr::Sequence {
        steps: vec![Expr::Assert {
            condition: Box::new(Expr::Boolean { value: true }),
            message: "extra guard".into(),
        }],
        value: Box::new(helper.body.clone()),
    };
    assert!(available(&effectful, "spend"));
    let rendered = render_with_capabilities(&effectful).unwrap();
    let helper = generated_body(&rendered.source, "pure_circuits", "merkleTreePathEntryRoot");
    assert!(helper.contains("AssertionFailed") && helper.contains("extra guard"));
    let spend = generated_body(&rendered.source, "recorded", "spend");
    assert!(
        spend.find("pure_circuits :: merkleTreePathRoot").unwrap()
            < spend.find("record_check_root").unwrap()
    );
    let mut wrong_slot = contract;
    wrong_slot
        .ledger_fields
        .iter_mut()
        .find(|f| f.id == "commitments")
        .unwrap()
        .index = 2;
    assert!(render(&wrong_slot).is_err());
    let mut json: serde_json::Value = serde_json::from_str(source).unwrap();
    json["schema_version"] = serde_json::json!(SCHEMA_VERSION);
    fn replace(value: &mut serde_json::Value) {
        match value {
            serde_json::Value::Object(map) => {
                if map.get("kind").and_then(|v| v.as_str()) == Some("set_insert") {
                    map.insert("kind".into(), serde_json::json!("set_remove"));
                }
                for value in map.values_mut() {
                    replace(value);
                }
            }
            serde_json::Value::Array(values) => {
                for value in values {
                    replace(value);
                }
            }
            _ => {}
        }
    }
    replace(&mut json);
    // Removal is a typed Set effect. The compiler must preserve this changed
    // program, without claiming it still implements the original spend policy.
    let removal: Contract = serde_json::from_value(json).unwrap();
    assert!(available(&removal, "spend"));
    let rendered = render_with_capabilities(&removal).unwrap();
    let spend = generated_body(&rendered.source, "recorded", "spend");
    assert!(spend.contains("ledger_slots :: nullifiers . record_remove"));
    assert!(!spend.contains("ledger_slots :: nullifiers . record_insert"));
    // The independent Merkle insertion remains part of this changed program.
    assert!(spend.contains("ledger_slots :: commitments . record_insert"));
}

#[test]
fn audited_schnorr_local_helper_requires_transitive_public_purity_and_typed_sources() {
    let mut contract: Contract =
        serde_json::from_str(include_str!("../schnorr-attestation-schema13-ir.json")).unwrap();
    contract.schema_version = SCHEMA_VERSION;
    let reported = render_with_capabilities(&contract).unwrap();
    for name in ["verifyAttestation", "acceptAttestation"] {
        assert!(
            reported
                .capabilities
                .circuits
                .iter()
                .find(|c| c.name == name)
                .unwrap()
                .recorded
        );
    }
    assert!(reported.source.contains(".call_local(|context|"));

    let mut effectful = contract.clone();
    let helper = effectful
        .stateful_circuits
        .iter_mut()
        .find(|c| c.name == "schnorrVerify")
        .unwrap();
    helper.actions.push(StateAction::CounterIncrement {
        field: "acceptedCount".into(),
        index: 1,
        amount: CounterAmount::Literal { value: 1 },
    });
    assert!(
        !render_with_capabilities(&effectful)
            .unwrap()
            .capabilities
            .circuits
            .iter()
            .find(|c| c.name == "verifyAttestation")
            .unwrap()
            .recorded
    );

    let mut read_opening = contract.clone();
    read_opening.ledger_fields.push(LedgerField {
        source: None,
        id: "hidden".into(),
        index: 3,
        path: vec![],
        declaration: LedgerFieldKind::Cell { ty: Type::Field },
    });
    let helper = read_opening
        .stateful_circuits
        .iter_mut()
        .find(|c| c.name == "schnorrVerify")
        .unwrap();
    helper.actions.push(StateAction::Expression {
        value: Expr::TransientCommit {
            value: Box::new(Expr::FieldLiteral { value: "1".into() }),
            opening: Box::new(Expr::CellRead {
                field: "hidden".into(),
                index: 3,
            }),
        },
    });
    assert!(
        !render_with_capabilities(&read_opening)
            .unwrap()
            .capabilities
            .circuits
            .iter()
            .find(|c| c.name == "verifyAttestation")
            .unwrap()
            .recorded
    );

    let mut wrong_key = contract.clone();
    let key = wrong_key
        .ledger_fields
        .iter_mut()
        .find(|f| f.id == "attestorKey")
        .unwrap();
    key.declaration = LedgerFieldKind::Cell { ty: Type::Field };
    assert!(render_with_capabilities(&wrong_key).is_err());

    let mut missing_guard = contract.clone();
    missing_guard
        .stateful_circuits
        .iter_mut()
        .find(|c| c.name == "verifyAttestation")
        .unwrap()
        .actions
        .remove(0);
    // The legacy audited-local profile still refuses this shape (its module test).
    // Unit composition admits the actual key read and audited local verifier,
    // without inventing the removed source guard.
    assert!(
        render_with_capabilities(&missing_guard)
            .unwrap()
            .capabilities
            .circuits
            .iter()
            .find(|c| c.name == "verifyAttestation")
            .unwrap()
            .recorded
    );

    let mut unknown = contract.clone();
    let helper = unknown
        .stateful_circuits
        .iter_mut()
        .find(|c| c.name == "schnorrVerifyDigest")
        .unwrap();
    let StateAction::CircuitCall { name, .. } = &mut helper.actions[0] else {
        unreachable!()
    };
    *name = "unknownHelper".into();
    assert!(render_with_capabilities(&unknown).is_err());

    let mut recursive = contract.clone();
    recursive
        .stateful_circuits
        .iter_mut()
        .find(|c| c.name == "schnorrVerify")
        .unwrap()
        .actions
        .push(StateAction::CircuitCall {
            name: "schnorrVerifyDigest".into(),
            arguments: vec![
                Expr::Parameter { name: "msg".into() },
                Expr::Parameter {
                    name: "signature".into(),
                },
                Expr::Parameter { name: "pk".into() },
            ],
        });
    assert!(render_with_capabilities(&recursive).is_err());
}
