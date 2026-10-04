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

use compact_rust_merkle_path_witness_fixture::ledger_contract::{
    LedgerView, PublicStateView, TryWitnesses, append, append_h, check_witness_history,
    check_witness_merkle, get_historic_path, get_path, initial_state,
};
use compact_rust_merkle_path_witness_fixture::ledger_slots::{h, t};
use compact_rust_merkle_path_witness_fixture::types::{MerkleTreeDigest, MerkleTreePath};
use midnight_compact_runtime as runtime;
use midnight_onchain_state::state::{
    ContractMaintenanceAuthority, ContractOperation, ContractState, EntryPointBuf,
};
use midnight_serialize::tagged_serialize;
use midnight_storage::storage::HashMap;
use runtime::CompactError;
use runtime::context::{ConstructorContext, RunningCost, WitnessContext};
use runtime::ledger::{ContractAddress, DefaultDB, StateValue};
use runtime::slots::MerkleSlot;

struct PathWitness;

fn state_hex(state: StateValue<DefaultDB>) -> String {
    let mut operations: HashMap<EntryPointBuf, ContractOperation, DefaultDB> = HashMap::new();
    for name in ["append", "append_h"] {
        operations = operations.insert(
            EntryPointBuf(name.as_bytes().to_vec()),
            ContractOperation::new(None),
        );
    }
    let state = ContractState::new(state, operations, ContractMaintenanceAuthority::default());
    let mut bytes = Vec::new();
    tagged_serialize(&state, &mut bytes).unwrap();
    hex::encode(bytes)
}

fn assert_gas(actual: RunningCost, expected: &serde_json::Value) {
    let actual = serde_json::to_value(actual).unwrap();
    for dimension in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
        assert_eq!(
            actual[dimension].as_u64().unwrap(),
            expected[dimension]
                .as_str()
                .unwrap()
                .parse::<u64>()
                .unwrap(),
            "{dimension} gas",
        );
    }
}

fn assert_fab(
    actual: &runtime::context::CircuitResult<(), impl Sized>,
    expected: &serde_json::Value,
) {
    assert_eq!(actual.private_transcript_outputs.len(), 1);
    let output = &actual.private_transcript_outputs[0];
    let atoms = output
        .value
        .0
        .iter()
        .map(|atom| &atom.0)
        .collect::<Vec<_>>();
    assert_eq!(
        serde_json::to_value(atoms).unwrap(),
        expected[0]["valueAtoms"]
    );
    assert_eq!(
        serde_json::to_value(&output.alignment).unwrap(),
        expected[0]["alignment"]
    );
}

impl TryWitnesses<()> for PathWitness {
    fn leaf_path(
        &self,
        context: WitnessContext<'_, (), LedgerView<'_>>,
    ) -> Result<((), MerkleTreePath), CompactError> {
        let tree = context.ledger.t()?;
        let path = tree.path_for_leaf(0, runtime::BoundedUint::<255>::new(7).unwrap())?;
        assert_eq!(Some(path.root()), tree.root());
        let compact = MerkleTreePath::from_ledger_path(path)?;
        assert_eq!(Some(compact.clone().into_ledger_path().root()), tree.root());
        Ok(((), compact))
    }

    fn historic_path(
        &self,
        context: WitnessContext<'_, (), LedgerView<'_>>,
    ) -> Result<((), MerkleTreePath), CompactError> {
        let tree = context.ledger.h()?;
        assert_eq!(tree.first_free()?.value(), 1);
        let root = tree.root().unwrap();
        assert!(tree.history()?.contains(&root));
        let path = tree.path_for_leaf(0, runtime::BoundedUint::<255>::new(7).unwrap())?;
        Ok(((), MerkleTreePath::from_ledger_path(path)?))
    }

    fn merkle_checks(
        &self,
        context: WitnessContext<'_, (), LedgerView<'_>>,
    ) -> Result<((), bool), CompactError> {
        let tree = context.ledger.t()?;
        let root = MerkleTreeDigest {
            field: tree.root().unwrap().0,
        };
        let full = tree.is_full()?;
        let known = tree.check_root(root)?;
        Ok(((), !full && known))
    }

    fn historic_checks(
        &self,
        context: WitnessContext<'_, (), LedgerView<'_>>,
    ) -> Result<((), bool), CompactError> {
        let tree = context.ledger.h()?;
        let root = MerkleTreeDigest {
            field: tree.root().unwrap().0,
        };
        let prior = tree
            .history()?
            .into_iter()
            .find(|entry| entry.0 != root.field)
            .expect("historic prior root");
        let full = tree.is_full()?;
        let known = tree.check_root(root)?;
        let known_prior = tree.check_root(MerkleTreeDigest { field: prior.0 })?;
        Ok(((), !full && known && known_prior))
    }
}

#[test]
fn generated_local_merkle_views_preserve_kind_leaf_and_depth() {
    type Leaf = runtime::BoundedUint<255>;
    let leaf = Leaf::new(7).unwrap();
    let initial = initial_state(ConstructorContext::new(())).unwrap();
    let initial_view = PublicStateView::from(&initial);
    assert_eq!(initial_view.t().unwrap().first_free().unwrap().value(), 0);
    assert_eq!(initial_view.h().unwrap().first_free().unwrap().value(), 0);
    assert_eq!(
        runtime::slots::PlainMerkleStateView::<Leaf, 3, DefaultDB>::DECLARED_DEPTH,
        3
    );
    assert!(matches!(
        MerkleSlot::<Leaf, 4, false>::new(&[0]).inspect(initial.ledger_state.get_ref()),
        Err(CompactError::InvalidLedgerCell(_))
    ));
    assert!(matches!(
        MerkleSlot::<Leaf, 4, true>::new(&[1]).inspect(initial.ledger_state.get_ref()),
        Err(CompactError::InvalidLedgerCell(_))
    ));

    let plain = append(
        initial.into_circuit_context(ContractAddress::default()),
        leaf,
    )
    .unwrap();
    {
        let view = PublicStateView::from(&plain);
        let generated = view.t().unwrap();
        let raw =
            runtime::ledger::merkle_tree_view_at_path(plain.context.query.state.get_ref(), &[0])
                .unwrap();
        assert_eq!(generated.root(), raw.root());
        assert_eq!(generated.first_free().unwrap(), raw.first_free().unwrap());
        assert_eq!(
            generated.path_for_leaf(0, leaf).unwrap().root(),
            raw.path_for_leaf(0, leaf).unwrap().root()
        );
        assert!(generated.path_for_leaf(8, leaf).is_err());
        assert_ne!(
            generated
                .path_for_leaf(0, Leaf::new(8).unwrap())
                .unwrap()
                .root(),
            generated.root().unwrap()
        );
    }

    let historic = append_h(plain.context, leaf).unwrap();
    let view = PublicStateView::from(&historic);
    let generated = view.h().unwrap();
    let raw = runtime::ledger::historic_merkle_tree_view_at_path(
        historic.context.query.state.get_ref(),
        &[1],
    )
    .unwrap();
    assert_eq!(generated.root(), raw.root());
    assert_eq!(generated.first_free().unwrap(), raw.first_free().unwrap());
    assert_eq!(generated.history().unwrap(), raw.history().unwrap());
    assert!(generated.contains_root(generated.root().unwrap()));
    assert_eq!(
        generated.path_for_leaf(0, leaf).unwrap().root(),
        raw.path_for_leaf(0, leaf).unwrap().root()
    );

    let broken_plain = StateValue::Array(
        vec![
            StateValue::Null,
            runtime::ledger::constructor_historic_merkle_tree::<DefaultDB>(3),
        ]
        .into(),
    );
    assert!(t.inspect(&broken_plain).is_err());
    let historic = runtime::ledger::constructor_historic_merkle_tree::<DefaultDB>(3);
    let StateValue::Array(fields) = &historic else {
        unreachable!()
    };
    let bad_history = StateValue::Array(
        vec![
            fields.get(0).unwrap().clone(),
            fields.get(1).unwrap().clone(),
            StateValue::Null,
        ]
        .into(),
    );
    let broken_history = StateValue::Array(
        vec![
            runtime::ledger::constructor_merkle_tree::<DefaultDB>(3),
            bad_history,
        ]
        .into(),
    );
    assert!(h.inspect(&broken_history).is_err());
    let plain = runtime::ledger::constructor_merkle_tree::<DefaultDB>(3);
    let StateValue::Array(fields) = &plain else {
        unreachable!()
    };
    let bad_first_free =
        StateValue::Array(vec![fields.get(0).unwrap().clone(), StateValue::Null].into());
    let broken_first_free = StateValue::Array(
        vec![
            bad_first_free,
            runtime::ledger::constructor_historic_merkle_tree::<DefaultDB>(3),
        ]
        .into(),
    );
    assert!(t.inspect(&broken_first_free).unwrap().first_free().is_err());
}

#[test]
fn generated_path_conversions_match_typescript_witness_output() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../../runtime-rs/tests/fixtures/merkle-path-witness.json"
    ))
    .unwrap();
    let initial = initial_state(ConstructorContext::new(())).unwrap();
    let context = initial.into_circuit_context(ContractAddress::default());
    let after_insert = append(context, runtime::BoundedUint::<255>::new(7).unwrap()).unwrap();
    assert_eq!(
        state_hex(after_insert.context.query.state.get_ref().clone()),
        oracle["afterPlainInsert"]
    );
    let output = get_path(after_insert.context, &PathWitness).unwrap();
    assert_eq!(output.gas_cost, RunningCost::ZERO);
    let rust_gas = serde_json::to_value(output.gas_cost).unwrap();
    for dimension in ["readTime", "computeTime", "bytesWritten", "bytesDeleted"] {
        assert_eq!(
            rust_gas[dimension].as_u64().unwrap(),
            oracle["reportedGas"][dimension]
                .as_str()
                .unwrap()
                .parse::<u64>()
                .unwrap(),
            "{dimension} witness gas",
        );
    }
    assert!(oracle["queries"].as_array().unwrap().is_empty());
    assert!(oracle["publicTranscript"].as_array().unwrap().is_empty());
    assert_eq!(output.result.leaf.value().to_string(), oracle["leaf"]);
    assert_eq!(
        output.result.path.0.len(),
        oracle["path"].as_array().unwrap().len()
    );
    for (actual, expected) in output
        .result
        .path
        .0
        .iter()
        .zip(oracle["path"].as_array().unwrap())
    {
        let sibling = num_bigint::BigUint::from_bytes_le(&actual.sibling.field.as_le_bytes());
        assert_eq!(sibling.to_string(), expected["sibling"]);
        assert_eq!(actual.goes_left, expected["goesLeft"]);
    }
    let transcript = &output.private_transcript_outputs[0];
    let atoms = transcript
        .value
        .0
        .iter()
        .map(|atom| &atom.0)
        .collect::<Vec<_>>();
    assert_eq!(
        serde_json::to_value(atoms).unwrap(),
        oracle["privateTranscriptOutputs"][0]["valueAtoms"]
    );
    assert_eq!(
        serde_json::to_value(&transcript.alignment).unwrap(),
        oracle["privateTranscriptOutputs"][0]["alignment"]
    );
    assert_eq!(output.private_transcript_outputs.len(), 1);

    let invalid = runtime::ledger::MerklePath {
        leaf: runtime::BoundedUint::<255>::new(7).unwrap(),
        path: Vec::new(),
    };
    assert!(MerkleTreePath::from_ledger_path(invalid).is_err());

    let context = append_h(output.context, runtime::BoundedUint::<255>::new(7).unwrap())
        .unwrap()
        .context;
    assert_eq!(
        state_hex(context.query.state.get_ref().clone()),
        oracle["afterHistoricInsert"]
    );
    let historic = get_historic_path(context, &PathWitness).unwrap();
    assert_eq!(historic.gas_cost, RunningCost::ZERO);
    assert_eq!(
        historic.result.leaf.value().to_string(),
        oracle["historic"]["leaf"]
    );
    assert_eq!(
        historic.result.path.0.len(),
        oracle["historic"]["path"].as_array().unwrap().len()
    );
    for (actual, expected) in historic
        .result
        .path
        .0
        .iter()
        .zip(oracle["historic"]["path"].as_array().unwrap())
    {
        let sibling = num_bigint::BigUint::from_bytes_le(&actual.sibling.field.as_le_bytes());
        assert_eq!(sibling.to_string(), expected["sibling"]);
        assert_eq!(actual.goes_left, expected["goesLeft"]);
    }
    assert!(oracle["historic"]["queries"].as_array().unwrap().is_empty());
    assert_fab(&historic, &oracle["historic"]["privateTranscriptOutputs"]);

    let context = historic.context;
    let plain_root = runtime::ledger::merkle_tree_view_at_path(context.query.state.get_ref(), &[0])
        .unwrap()
        .root()
        .unwrap();
    let (plain_full_query, _) =
        runtime::ledger::merkle_is_full(&context.query, &[0], 3, None, &context.cost_model)
            .unwrap();
    let (plain_root_query, _) = runtime::ledger::merkle_check_root(
        &context.query,
        &[0],
        MerkleTreeDigest {
            field: plain_root.0,
        },
        None,
        &context.cost_model,
    )
    .unwrap();
    let plain_oracle = &oracle["plainReads"];
    assert_eq!(
        plain_oracle["queries"][0]["opTags"],
        serde_json::json!(["dup", "idx", "idx", "push", "lt", "neg", "popeq"])
    );
    assert_eq!(
        plain_oracle["queries"][1]["opTags"],
        serde_json::json!(["dup", "idx", "idx", "root", "push", "eq", "popeq"])
    );
    assert_gas(
        plain_full_query.gas_cost,
        &plain_oracle["queries"][0]["gasCost"],
    );
    assert_gas(
        plain_root_query.gas_cost,
        &plain_oracle["queries"][1]["gasCost"],
    );
    let plain = check_witness_merkle(context, &PathWitness).unwrap();
    assert_eq!(plain.result, plain_oracle["result"]);
    assert_eq!(
        plain.gas_cost,
        plain_full_query.gas_cost + plain_root_query.gas_cost
    );
    assert_fab(&plain, &plain_oracle["privateTranscriptOutputs"]);
    assert!(
        plain_oracle["publicTranscript"]
            .as_array()
            .unwrap()
            .is_empty()
    );

    let context = plain.context;
    let historic_view =
        runtime::ledger::historic_merkle_tree_view_at_path(context.query.state.get_ref(), &[1])
            .unwrap();
    let historic_root = historic_view.root().unwrap();
    let prior_root = historic_view
        .history()
        .unwrap()
        .into_iter()
        .find(|entry| *entry != historic_root)
        .unwrap();
    let (historic_full_query, _) =
        runtime::ledger::historic_is_full(&context.query, &[1], 3, None, &context.cost_model)
            .unwrap();
    let (historic_root_query, _) = runtime::ledger::historic_check_root(
        &context.query,
        &[1],
        MerkleTreeDigest {
            field: historic_root.0,
        },
        None,
        &context.cost_model,
    )
    .unwrap();
    let (historic_prior_query, _) = runtime::ledger::historic_check_root(
        &context.query,
        &[1],
        MerkleTreeDigest {
            field: prior_root.0,
        },
        None,
        &context.cost_model,
    )
    .unwrap();
    let historic_oracle = &oracle["historicReads"];
    assert_eq!(
        historic_oracle["queries"][0]["opTags"],
        serde_json::json!(["dup", "idx", "idx", "push", "lt", "neg", "popeq"])
    );
    assert_eq!(
        historic_oracle["queries"][1]["opTags"],
        serde_json::json!(["dup", "idx", "idx", "push", "member", "popeq"])
    );
    assert_eq!(
        historic_oracle["queries"][2]["opTags"],
        historic_oracle["queries"][1]["opTags"]
    );
    assert_gas(
        historic_full_query.gas_cost,
        &historic_oracle["queries"][0]["gasCost"],
    );
    assert_gas(
        historic_root_query.gas_cost,
        &historic_oracle["queries"][1]["gasCost"],
    );
    assert_gas(
        historic_prior_query.gas_cost,
        &historic_oracle["queries"][2]["gasCost"],
    );
    let historic_reads = check_witness_history(context, &PathWitness).unwrap();
    assert_eq!(historic_reads.result, historic_oracle["result"]);
    assert_eq!(
        historic_reads.gas_cost,
        historic_full_query.gas_cost + historic_root_query.gas_cost + historic_prior_query.gas_cost
    );
    assert_fab(
        &historic_reads,
        &historic_oracle["privateTranscriptOutputs"],
    );
    assert!(
        historic_oracle["publicTranscript"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

#[test]
fn merkle_witness_vm_rejection_is_a_typed_error() {
    let mut context = initial_state(ConstructorContext::new(()))
        .unwrap()
        .into_circuit_context(ContractAddress::default());
    context.gas_limit = Some(RunningCost::ZERO);
    assert!(matches!(
        check_witness_merkle(context, &PathWitness),
        Err(CompactError::LedgerQueryRejected(_))
    ));

    let mut context = append_h(
        initial_state(ConstructorContext::new(()))
            .unwrap()
            .into_circuit_context(ContractAddress::default()),
        runtime::BoundedUint::<255>::new(7).unwrap(),
    )
    .unwrap()
    .context;
    context.gas_limit = Some(RunningCost::ZERO);
    assert!(matches!(
        check_witness_history(context, &PathWitness),
        Err(CompactError::LedgerQueryRejected(_))
    ));
}
