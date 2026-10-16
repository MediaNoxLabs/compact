// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Maintained source projections: one witness, one coordinate, no public query.
use compact_rust_witness_jubjub_fixture::{ledger_contract as c, runtime as r};
use midnight_transient_crypto::curve::{EmbeddedFr, EmbeddedGroupAffine};
use std::{cell::RefCell, rc::Rc};

#[derive(Clone)]
struct PointWitness {
    point: EmbeddedGroupAffine,
    fail: bool,
    events: Rc<RefCell<Vec<(r::Field, u64)>>>,
}
impl PointWitness {
    fn new(scalar: u64, fail: bool) -> Self {
        Self {
            point: EmbeddedGroupAffine::generator() * EmbeddedFr::from(scalar),
            fail,
            events: Rc::default(),
        }
    }
}
fn failure() -> r::CompactError {
    r::CompactError::InvalidLedgerCell("point witness refused".to_owned())
}
impl c::TryWitnesses<u64> for PointWitness {
    fn secret_point(
        &self,
        context: r::context::WitnessContext<'_, u64, c::LedgerView<'_>>,
        seed: r::Field,
    ) -> Result<(u64, r::JubjubPoint), r::CompactError> {
        self.events
            .borrow_mut()
            .push((seed, *context.private_state));
        if self.fail {
            return Err(failure());
        }
        Ok((*context.private_state + 1, self.point.into()))
    }
    fn secret_scalar(
        &self,
        _: r::context::WitnessContext<'_, u64, c::LedgerView<'_>>,
        _: r::Field,
    ) -> Result<(u64, r::Field), r::CompactError> {
        panic!("coordinate projections must not request a scalar witness")
    }
}
fn context() -> r::context::CircuitContext<u64> {
    c::initial_state(r::context::ConstructorContext::new(7))
        .unwrap()
        .into_circuit_context(Default::default())
}
#[derive(Clone, Copy)]
enum Coordinate {
    X,
    Y,
}
impl Coordinate {
    fn expected(self, point: EmbeddedGroupAffine) -> r::Field {
        // An independent official curve accessor, not a runtime helper under test.
        match self {
            Self::X => point.x().unwrap(),
            Self::Y => point.y().unwrap(),
        }
    }
    fn native(
        self,
        ctx: r::context::CircuitContext<u64>,
        witness: &PointWitness,
        seed: r::Field,
    ) -> Result<r::context::CircuitResult<u64, r::Field>, r::CompactError> {
        match self {
            Self::X => c::point_x(ctx, witness, seed),
            Self::Y => c::point_y(ctx, witness, seed),
        }
    }
    fn recorded(
        self,
        ctx: r::context::CircuitContext<u64>,
        witness: &PointWitness,
        seed: r::Field,
    ) -> Result<r::recording::RecordedCircuitResult<u64, r::Field>, r::CompactError> {
        match self {
            Self::X => c::recorded::point_x(ctx, witness, seed),
            Self::Y => c::recorded::point_y(ctx, witness, seed),
        }
    }
    #[cfg(feature = "ledger-transaction")]
    fn observed<'a>(
        self,
        state: &'a r::transaction::ObservedContractState,
        witness: &PointWitness,
        seed: r::Field,
    ) -> Result<r::transaction::RecordedCall<'a, u64, r::Field>, r::CompactError> {
        let contract = c::Contract::from(witness.clone());
        match self {
            Self::X => contract.recording().point_x_call(state, 7, seed),
            Self::Y => contract.recording().point_y_call(state, 7, seed),
        }
    }
}
fn compare(
    native: &r::context::CircuitResult<u64, r::Field>,
    recorded: &r::recording::RecordedCircuitResult<u64, r::Field>,
    expected: r::Field,
    point: EmbeddedGroupAffine,
) {
    assert_eq!(native.result, expected);
    assert_eq!(recorded.execution.result, expected);
    assert_eq!(native.context.private_state, 8);
    assert_eq!(recorded.execution.context.private_state, 8);
    assert_eq!(
        native.context.query.state,
        recorded.execution.context.query.state
    );
    assert_eq!(
        native.context.query.effects,
        recorded.execution.context.query.effects
    );
    assert_eq!(native.gas_cost, recorded.execution.gas_cost);
    let private = vec![r::fab::AlignedValue::from(r::JubjubPoint::from(point))];
    assert_eq!(native.private_transcript_outputs, private);
    assert_eq!(recorded.execution.private_transcript_outputs, private);
    assert!(recorded.public.verify_ops().is_empty());
    let replay = recorded
        .public
        .initial()
        .query(
            recorded.public.verify_ops(),
            None,
            &native.context.cost_model,
        )
        .unwrap();
    assert_eq!(replay.context.state, native.context.query.state);
    assert_eq!(replay.context.effects, native.context.query.effects);
    assert_eq!(
        r::fab::AlignedValue::from(recorded.execution.result),
        r::fab::AlignedValue::from(expected)
    );
}
#[test]
fn recorded_coordinates_preserve_official_values_and_single_point_witness() {
    let seed = r::Field::from(2_u64);
    for coordinate in [Coordinate::X, Coordinate::Y] {
        for scalar in [0, 1, 2, 17] {
            let native_witness = PointWitness::new(scalar, false);
            let recorded_witness = PointWitness::new(scalar, false);
            let native = coordinate.native(context(), &native_witness, seed).unwrap();
            let recorded = coordinate
                .recorded(context(), &recorded_witness, seed)
                .unwrap();
            let expected = coordinate.expected(native_witness.point);
            if scalar == 0 {
                assert!(native_witness.point.is_identity());
                assert_eq!(
                    expected,
                    r::Field::from(match coordinate {
                        Coordinate::X => 0_u64,
                        Coordinate::Y => 1_u64,
                    })
                );
            }
            compare(&native, &recorded, expected, native_witness.point);
            for witness in [&native_witness, &recorded_witness] {
                assert_eq!(*witness.events.borrow(), vec![(seed, 7)]);
            }
        }
    }
}
#[test]
fn failed_point_witness_is_called_once_and_prevents_a_recorded_result() {
    let seed = r::Field::from(2_u64);
    for coordinate in [Coordinate::X, Coordinate::Y] {
        let native = PointWitness::new(1, true);
        let recorded = PointWitness::new(1, true);
        assert_eq!(
            coordinate.native(context(), &native, seed).err().unwrap(),
            failure()
        );
        assert_eq!(
            coordinate
                .recorded(context(), &recorded, seed)
                .err()
                .unwrap(),
            failure()
        );
        for witness in [&native, &recorded] {
            assert_eq!(*witness.events.borrow(), vec![(seed, 7)]);
        }
    }
}

#[cfg(feature = "ledger-transaction")]
#[test]
fn observed_coordinates_preserve_witness_fab_but_refuse_empty_public_programs() {
    use midnight_onchain_state::state::{ContractOperation, EntryPointBuf};
    use midnight_serialize::{Deserializable, Serializable};
    use r::transaction::{
        Observation, ObservedCallError, ObservedContractState, PrepareCallError, VerifierKey,
    };
    // The installed carrier matches the observation; the empty program is refused
    // before proof preparation. This carrier is not a proof key.
    let mut bytes = Vec::new();
    Vec::<u8>::new().serialize(&mut bytes).unwrap();
    let key = VerifierKey::deserialize(&mut bytes.as_slice(), 0).unwrap();
    let mut operations = r::ledger::LedgerHashMap::new();
    for name in ["point_x", "point_y"] {
        operations = operations.insert(
            EntryPointBuf(name.as_bytes().to_vec()),
            ContractOperation::new(Some(key.clone())),
        );
    }
    let state = ObservedContractState::new(
        Default::default(),
        r::ledger::ContractState::new(
            context().query.state.get_ref().clone(),
            operations,
            Default::default(),
        ),
        Observation {
            transaction_hash: [0; 32],
            block_hash: [0; 32],
            block_height: 0,
        },
    );
    let seed = r::Field::from(2_u64);
    for coordinate in [Coordinate::X, Coordinate::Y] {
        for scalar in [0, 1, 17] {
            let native_witness = PointWitness::new(scalar, false);
            let witness = PointWitness::new(scalar, false);
            let native = coordinate
                .native(state.circuit_context(7), &native_witness, seed)
                .unwrap();
            let call = coordinate.observed(&state, &witness, seed).unwrap();
            compare(
                &native,
                call.recorded(),
                coordinate.expected(witness.point),
                witness.point,
            );
            assert!(matches!(
                call.prepare(key.clone(), r::Field::from(0_u64)),
                Err(ObservedCallError::Prepare(
                    PrepareCallError::EmptyTranscript
                ))
            ));
            assert_eq!(*witness.events.borrow(), vec![(seed, 7)]);
        }
        let failed = PointWitness::new(1, true);
        assert_eq!(
            coordinate.observed(&state, &failed, seed).err().unwrap(),
            failure()
        );
        assert_eq!(*failed.events.borrow(), vec![(seed, 7)]);
    }
}
