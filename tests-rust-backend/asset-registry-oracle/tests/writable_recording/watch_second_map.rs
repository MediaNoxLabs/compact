// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Independent second-Map authorization branch for watch Add and Drop.
use super::*;

#[test]
fn typed_watch_accepts_second_map_only_and_matches_manual_add_drop() {
    // This is an independent Rust/manual branch control, not a new TS oracle row.
    // The existing TS watch rows authorize via records; here only custodyGrants
    // contains the key, so recordExists must query its second Map.
    let key = runtime::OpaqueString::from("grant-only-α");
    for mutation in [ListMutation::Add, ListMutation::Drop] {
        let seed_witness = TrackingWitness::default();
        let context = setCustodyGrant(
            initial("success", &seed_witness),
            &seed_witness,
            key.clone(),
            custody_grant(false),
            RecordMutation::Insert,
        )
        .unwrap()
        .context;
        let context = if mutation == ListMutation::Drop {
            setWatch(context, &seed_witness, key.clone(), ListMutation::Add)
                .unwrap()
                .context
        } else {
            context
        };
        let private = context.private_state;
        let snapshot = ContractState::new(
            context.query.state.get_ref().clone(),
            Default::default(),
            Default::default(),
        );
        assert!(
            !ledger_slots::records
                .inspect(snapshot.data.get_ref())
                .unwrap()
                .member(key.clone())
        );
        assert!(
            ledger_slots::custodyGrants
                .inspect(snapshot.data.get_ref())
                .unwrap()
                .member(key.clone())
        );
        let fresh =
            || CircuitContext::from_contract_state(private, ContractAddress::default(), &snapshot);
        let native_witness = TrackingWitness::default();
        let recorded_witness = TrackingWitness::default();
        let manual_witness = TrackingWitness::default();
        let native = setWatch(fresh(), &native_witness, key.clone(), mutation).unwrap();
        let recorded =
            recorded::setWatch(fresh(), &recorded_witness, key.clone(), mutation).unwrap();

        let (frame, open) = ledger_slots::open
            .record_read(RecordingFrame::new(fresh()))
            .unwrap();
        assert!(open);
        let (frame, frozen) = ledger_slots::frozen.record_read(frame).unwrap();
        assert!(!frozen);
        let (frame, first_exists) = ledger_slots::records
            .record_member(frame, key.clone())
            .unwrap();
        assert!(!first_exists);
        let (frame, second_exists) = ledger_slots::custodyGrants
            .record_member(frame, key.clone())
            .unwrap();
        assert!(second_exists);
        let (frame, watched) = ledger_slots::watchList
            .record_member(frame, key.clone())
            .unwrap();
        assert_eq!(watched, mutation == ListMutation::Drop);
        let frame = if mutation == ListMutation::Add {
            ledger_slots::watchList
                .record_insert(frame, key.clone())
                .unwrap()
        } else {
            ledger_slots::watchList
                .record_remove(frame, key.clone())
                .unwrap()
        };
        let frame = ledger_slots::writeCount.record_increment(frame, 1).unwrap();
        let frame = ledger_slots::revision.record_increment(frame, 1).unwrap();
        let (frame, timestamp) = frame
            .try_witness_metered(|context, meter| {
                // Use public slot views for the independent manual callback;
                // generated LedgerView fields intentionally remain private.
                manual_witness.0.borrow_mut().push("currentTimestamp");
                ledger_slots::recordCount.witness_read(meter)?;
                ledger_slots::records.witness_view(meter)?;
                ledger_slots::watchList.witness_view(meter)?;
                Ok((
                    context.private_state + 1,
                    BoundedUint::<{ u64::MAX as u128 }>::new(1_700_000_000)?,
                ))
            })
            .unwrap();
        let manual = ledger_slots::updatedAt
            .record_write(frame, timestamp)
            .unwrap()
            .finish(());
        let outputs = vec![runtime::fab::AlignedValue::from(
            BoundedUint::<{ u64::MAX as u128 }>::new(1_700_000_000).unwrap(),
        )];
        let manual_replay = manual
            .public
            .initial()
            .query(
                manual.public.verify_ops(),
                None,
                &manual.execution.context.cost_model,
            )
            .unwrap();
        for candidate in [&recorded, &manual] {
            assert_eq!(
                candidate.execution.context.query.state,
                native.context.query.state
            );
            assert_eq!(
                candidate.execution.context.query.effects,
                native.context.query.effects
            );
            assert_eq!(candidate.execution.gas_cost, native.gas_cost);
            assert_eq!(candidate.execution.context.private_state, private + 1);
            assert_eq!(candidate.execution.private_transcript_outputs, outputs);
            assert_eq!(candidate.public.verify_ops(), manual.public.verify_ops());
            let replay = candidate
                .public
                .initial()
                .query(
                    candidate.public.verify_ops(),
                    None,
                    &candidate.execution.context.cost_model,
                )
                .unwrap();
            assert_eq!(replay.context.state, native.context.query.state);
            assert_eq!(replay.context.effects, native.context.query.effects);
            assert_eq!(replay.gas_cost, manual_replay.gas_cost);
        }
        assert_eq!(native.context.private_state, private + 1);
        assert_eq!(native.private_transcript_outputs, outputs);
        for witness in [&native_witness, &recorded_witness, &manual_witness] {
            assert_eq!(witness.calls(), ["currentTimestamp"]);
        }
        let state = native.context.query.state.get_ref();
        assert_eq!(
            ledger_slots::watchList
                .inspect(state)
                .unwrap()
                .member(key.clone()),
            mutation == ListMutation::Add
        );
        assert!(
            !ledger_slots::records
                .inspect(state)
                .unwrap()
                .member(key.clone())
        );
        assert_eq!(
            ledger_slots::custodyGrants
                .inspect(state)
                .unwrap()
                .lookup(key.clone())
                .unwrap(),
            custody_grant(false)
        );
    }
}
