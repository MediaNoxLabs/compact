// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use compact_rust_merkle_digest_witness_views_fixture::{
    ledger_contract as c, ledger_slots as slots, runtime as r, types,
};
use r::context::{CircuitContext, ConstructorContext, WitnessContext};
use r::recording::RecordingFrame;
use std::{cell::RefCell, rc::Rc};
type Root = types::MerkleTreeDigest;
type Events = Rc<RefCell<Vec<(u64, r::Field, r::Field)>>>;
struct Witness {
    plain: Root,
    historic: Root,
    events: Events,
}
impl c::TryWitnesses<u64> for Witness {
    fn flag(
        &self,
        ctx: WitnessContext<'_, u64, c::LedgerView<'_>>,
    ) -> Result<(u64, bool), r::CompactError> {
        let plain = ctx.ledger.plain()?;
        let historic = ctx.ledger.historic()?;
        let plain_official = plain.root().unwrap();
        let historic_official = historic.root().unwrap();
        // Exercise the generated canonical derive, not a manual field adapter.
        let plain_digest: Root = plain_official.into();
        let historic_digest: Root = historic_official.into();
        let plain_roundtrip: r::ledger::MerkleTreeDigest = plain_digest.clone().into();
        let historic_roundtrip: r::ledger::MerkleTreeDigest = historic_digest.clone().into();
        assert_eq!(plain_roundtrip, plain_official);
        assert_eq!(historic_roundtrip, historic_official);
        assert_eq!(
            r::fab::AlignedValue::from(plain_digest.clone()),
            r::fab::AlignedValue::from(plain_official)
        );
        assert_eq!(
            r::fab::AlignedValue::from(historic_digest.clone()),
            r::fab::AlignedValue::from(historic_official)
        );
        self.events.borrow_mut().push((
            *ctx.private_state,
            plain_digest.field,
            historic_digest.field,
        ));
        let plain_matches = plain.check_root(self.plain.clone())?;
        let historic_matches = historic.check_root(self.historic.clone())?;
        Ok((*ctx.private_state + 1, plain_matches && historic_matches))
    }
}
fn context(populated: bool) -> CircuitContext<u64> {
    let mut context = c::initial_state(ConstructorContext::new(5))
        .unwrap()
        .into_circuit_context(Default::default());
    if populated {
        context = slots::plain
            .insert(context, r::BoundedUint::new(37).unwrap())
            .unwrap()
            .context;
        context = slots::historic
            .insert(context, r::BoundedUint::new(99).unwrap())
            .unwrap()
            .context;
    }
    context
}
#[test]
fn implicit_digest_is_usable_from_both_generated_witness_views() {
    for populated in [false, true] {
        for query_current in [false, true] {
            let input = context(populated);
            let plain = slots::plain
                .inspect(input.query.state.get_ref())
                .unwrap()
                .root()
                .unwrap();
            let historic = slots::historic
                .inspect(input.query.state.get_ref())
                .unwrap()
                .root()
                .unwrap();
            let query = |actual: r::ledger::MerkleTreeDigest| -> Root {
                if query_current {
                    actual.into()
                } else {
                    r::ledger::MerkleTreeDigest(r::Field::from(97_u64)).into()
                }
            };
            let a = Witness {
                plain: query(plain),
                historic: query(historic),
                events: Rc::default(),
            };
            let b = Witness {
                plain: query(plain),
                historic: query(historic),
                events: Rc::default(),
            };
            let native = c::read_flag(context(populated), &a).unwrap();
            let recorded = c::recorded::read_flag(context(populated), &b).unwrap();
            let (frame, value) = RecordingFrame::new(context(populated))
                .try_witness_metered(|ctx, meter| {
                    let p = meter.read_merkle_check_root(slots::plain.path(), query(plain))?;
                    let h = meter
                        .read_historic_merkle_check_root(slots::historic.path(), query(historic))?;
                    Ok((ctx.private_state + 1, p && h))
                })
                .unwrap();
            let manual = frame.finish(value);
            assert_eq!(native.result, query_current);
            assert_eq!(*a.events.borrow(), vec![(5, plain.0, historic.0)]);
            assert_eq!(*b.events.borrow(), *a.events.borrow());
            for actual in [&native, &recorded.execution, &manual.execution] {
                assert_eq!(actual.result, query_current);
                assert_eq!(actual.context.private_state, 6);
                assert_eq!(actual.context.query.state, input.query.state);
                assert_eq!(actual.context.query.effects, input.query.effects);
                assert_eq!(actual.gas_cost, native.gas_cost);
                assert_eq!(
                    actual.private_transcript_outputs,
                    vec![r::fab::AlignedValue::from(query_current)]
                );
            }
            // Witness-local reads are metered; they do not become public proof queries.
            assert!(recorded.public.verify_ops().is_empty());
            assert!(manual.public.verify_ops().is_empty());
        }
    }
}
