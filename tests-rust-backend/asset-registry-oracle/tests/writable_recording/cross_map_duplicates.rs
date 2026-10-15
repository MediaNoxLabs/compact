// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Cross-Map duplicate controls; independent Rust checks, not new TS rows.
use super::*;
use compact_rust_asset_registry_oracle_fixture::ledger_contract::Contract;
use runtime::transaction::{Observation, ObservedContractState};
use std::rc::Rc;

#[derive(Clone, Default)]
struct SharedWitness(Rc<TrackingWitness>);
impl Witnesses<u64> for SharedWitness {
    fn localOperatorKey(
        &self,
        context: WitnessContext<'_, u64, LedgerView<'_>>,
    ) -> (u64, JubjubPoint) {
        self.0.localOperatorKey(context)
    }
    fn localAuditorKey(
        &self,
        context: WitnessContext<'_, u64, LedgerView<'_>>,
    ) -> (u64, JubjubPoint) {
        self.0.localAuditorKey(context)
    }
    fn currentTimestamp(
        &self,
        context: WitnessContext<'_, u64, LedgerView<'_>>,
    ) -> (u64, BoundedUint<{ u64::MAX as u128 }>) {
        self.0.currentTimestamp(context)
    }
}

#[test]
fn typed_map_insert_rejects_cross_map_duplicates_before_timestamp() {
    for insert_record in [true, false] {
        let key = runtime::OpaqueString::from("shared-λ\0key");
        let seed = TrackingWitness::default();
        let context = initial("success", &seed);
        let context = if insert_record {
            setCustodyGrant(
                context,
                &seed,
                key.clone(),
                custody_grant(false),
                RecordMutation::Insert,
            )
            .unwrap()
            .context
        } else {
            setRecord(
                context,
                &seed,
                key.clone(),
                asset_record(false, false),
                RecordMutation::Insert,
            )
            .unwrap()
            .context
        };
        let private = context.private_state;
        let observed = ObservedContractState::new(
            ContractAddress::default(),
            ContractState::new(
                context.query.state.get_ref().clone(),
                Default::default(),
                Default::default(),
            ),
            Observation {
                transaction_hash: [0; 32],
                block_hash: [0; 32],
                block_height: 0,
            },
        );
        assert_eq!(
            ledger_slots::records
                .inspect(observed.contract().data.get_ref())
                .unwrap()
                .member(key.clone()),
            !insert_record
        );
        assert_eq!(
            ledger_slots::custodyGrants
                .inspect(observed.contract().data.get_ref())
                .unwrap()
                .member(key.clone()),
            insert_record
        );
        let native_witness = TrackingWitness::default();
        let recorded_witness = TrackingWitness::default();
        let observed_witness = SharedWitness::default();
        let generated = Contract::from(observed_witness.clone());
        let errors = if insert_record {
            [
                setRecord(
                    observed.circuit_context(private),
                    &native_witness,
                    key.clone(),
                    asset_record(false, false),
                    RecordMutation::Insert,
                )
                .err()
                .unwrap(),
                recorded::setRecord(
                    observed.circuit_context(private),
                    &recorded_witness,
                    key.clone(),
                    asset_record(false, false),
                    RecordMutation::Insert,
                )
                .err()
                .unwrap(),
                generated
                    .recording()
                    .setRecord_call(
                        &observed,
                        private,
                        key.clone(),
                        asset_record(false, false),
                        RecordMutation::Insert,
                    )
                    .err()
                    .unwrap(),
            ]
        } else {
            [
                setCustodyGrant(
                    observed.circuit_context(private),
                    &native_witness,
                    key.clone(),
                    custody_grant(false),
                    RecordMutation::Insert,
                )
                .err()
                .unwrap(),
                recorded::setCustodyGrant(
                    observed.circuit_context(private),
                    &recorded_witness,
                    key.clone(),
                    custody_grant(false),
                    RecordMutation::Insert,
                )
                .err()
                .unwrap(),
                generated
                    .recording()
                    .setCustodyGrant_call(
                        &observed,
                        private,
                        key.clone(),
                        custody_grant(false),
                        RecordMutation::Insert,
                    )
                    .err()
                    .unwrap(),
            ]
        };
        for error in errors {
            assert_eq!(
                error,
                runtime::CompactError::AssertionFailed(
                    if insert_record {
                        "record already exists"
                    } else {
                        "grant already exists"
                    }
                    .into()
                )
            );
        }
        for witness in [
            &native_witness,
            &recorded_witness,
            observed_witness.0.as_ref(),
        ] {
            assert!(
                witness.calls().is_empty(),
                "duplicate guard must precede timestamp"
            );
        }
        // Errors consume their contexts. These assertions establish refusal and
        // callback prefix, not transactional rollback of a consumed execution.
    }
}
