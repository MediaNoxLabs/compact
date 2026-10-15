// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Independent source-ordered Map writes shared by the two proof scenarios.
use super::*;
use compact_rust_asset_registry_oracle_fixture::ledger_slots as slots;
use compact_rust_asset_registry_oracle_fixture::types::CustodyGrant;
use runtime::context::{CircuitContext, CircuitResult};
use runtime::recording::{RecordedCircuitResult, RecordingFrame};
use std::{cell::RefCell, rc::Rc};

#[derive(Clone, Default)]
pub(crate) struct TimestampWitness(pub Rc<RefCell<Vec<u64>>>);
impl asset::Witnesses<u64> for TimestampWitness {
    fn localOperatorKey(
        &self,
        _: WitnessContext<'_, u64, asset::LedgerView<'_>>,
    ) -> (u64, runtime::JubjubPoint) {
        panic!("Map write must not request operator key")
    }
    fn localAuditorKey(
        &self,
        _: WitnessContext<'_, u64, asset::LedgerView<'_>>,
    ) -> (u64, runtime::JubjubPoint) {
        panic!("Map write must not request auditor key")
    }
    fn currentTimestamp(
        &self,
        context: WitnessContext<'_, u64, asset::LedgerView<'_>>,
    ) -> (u64, BoundedUint<{ u64::MAX as u128 }>) {
        self.0.borrow_mut().push(*context.private_state);
        asset::Witnesses::currentTimestamp(&super::super::asset_writable::AssetWitness, context)
    }
}

pub(crate) enum StoredValue {
    Record(AssetRecord),
    Grant(CustodyGrant),
}

pub(crate) fn write(
    context: CircuitContext<u64>,
    key: runtime::OpaqueString,
    value: StoredValue,
    update: bool,
    witness: &TimestampWitness,
) -> Result<RecordedCircuitResult<u64, ()>, Box<dyn Error>> {
    let (frame, open) = slots::open.record_read(RecordingFrame::new(context))?;
    if !open {
        return Err("manual registry closed".into());
    }
    let (mut frame, frozen) = slots::frozen.record_read(frame)?;
    if frozen {
        return Err("manual registry frozen".into());
    }
    if let StoredValue::Record(record) = &value
        && record.kind == AssetClass::Unspecified
    {
        return Err("manual asset class unspecified".into());
    }
    if update {
        frame = match &value {
            StoredValue::Record(_) => {
                let (frame, exists) = slots::records.record_member(frame, key.clone())?;
                if !exists {
                    return Err("manual record missing".into());
                }
                slots::records.record_remove(frame, key.clone())?
            }
            StoredValue::Grant(_) => {
                let (frame, exists) = slots::custodyGrants.record_member(frame, key.clone())?;
                if !exists {
                    return Err("manual grant missing".into());
                }
                slots::custodyGrants.record_remove(frame, key.clone())?
            }
        };
    } else {
        // Compact's || is lazy: the second Map is queried only when the first
        // membership is false. A successful Insert requires both absent.
        let (next, exists) = slots::records.record_member(frame, key.clone())?;
        frame = next;
        if exists {
            return Err("manual duplicate record".into());
        }
        let (next, exists) = slots::custodyGrants.record_member(frame, key.clone())?;
        frame = next;
        if exists {
            return Err("manual duplicate grant".into());
        }
        if matches!(&value, StoredValue::Record(_)) {
            frame = slots::recordCount.record_increment(frame, 1)?;
        }
    }
    frame = match value {
        StoredValue::Record(value) => slots::records.record_insert(frame, key, value)?,
        StoredValue::Grant(value) => slots::custodyGrants.record_insert(frame, key, value)?,
    };
    frame = slots::writeCount.record_increment(frame, 1)?;
    frame = slots::revision.record_increment(frame, 1)?;
    let (frame, timestamp) = frame.try_witness_metered(|context, meter| {
        witness.0.borrow_mut().push(context.private_state);
        slots::recordCount.witness_read(meter)?;
        slots::records.witness_view(meter)?;
        slots::watchList.witness_view(meter)?;
        Ok((
            context.private_state + 1,
            BoundedUint::<{ u64::MAX as u128 }>::new(1_700_000_000)?,
        ))
    })?;
    Ok(slots::updatedAt.record_write(frame, timestamp)?.finish(()))
}

pub(crate) fn compare(
    native: &CircuitResult<u64, ()>,
    recorded: &RecordedCircuitResult<u64, ()>,
    observed: &RecordedCircuitResult<u64, ()>,
    manual: &RecordedCircuitResult<u64, ()>,
    private: u64,
    witnesses: &[&TimestampWitness],
) -> Result<(), Box<dyn Error>> {
    let expected_private = vec![AlignedValue::from(
        BoundedUint::<{ u64::MAX as u128 }>::new(1_700_000_000)?,
    )];
    if native.context.private_state != private + 1
        || native.private_transcript_outputs != expected_private
    {
        return Err("Map write native private timestamp differs".into());
    }
    for witness in witnesses {
        if *witness.0.borrow() != [private] {
            return Err("Map write timestamp callback order differs".into());
        }
    }
    let manual_replay = manual.public.initial().query(
        manual.public.verify_ops(),
        None,
        &manual.execution.context.cost_model,
    )?;
    for candidate in [recorded, observed, manual] {
        if candidate.execution.context.query.state != native.context.query.state
            || candidate.execution.context.query.effects != native.context.query.effects
            || candidate.execution.gas_cost != native.gas_cost
            || candidate.execution.context.private_state != private + 1
            || candidate.execution.private_transcript_outputs != expected_private
            || candidate.public.verify_ops() != manual.public.verify_ops()
        {
            return Err("Map write native/recorded/observed/manual execution differs".into());
        }
        let replay = candidate.public.initial().query(
            candidate.public.verify_ops(),
            None,
            &candidate.execution.context.cost_model,
        )?;
        if replay.context.state != native.context.query.state
            || replay.context.effects != native.context.query.effects
            || replay.gas_cost != manual_replay.gas_cost
        {
            return Err("Map write replay differs from manual replay".into());
        }
    }
    Ok(())
}
