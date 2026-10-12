// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Actual source-native boundaries plus separately labeled recorded runtime adapters.
include!(concat!(env!("OUT_DIR"), "/inputs_contract.rs"));

#[cfg(test)]
mod tests {
    use super::{ledger_contract as c, ledger_slots, runtime as r, types};
    use r::context::{CircuitContext, ConstructorContext};
    use r::ledger::CellValue;
    use r::recording::RecordingFrame;
    use r::slots::CellSlot;
    use r::{CompactError, Field, OpaqueBytes, OpaqueString};

    fn error() -> CompactError {
        CompactError::InvalidLedgerCell("opaque ledger input has trailing zero bytes".into())
    }
    fn envelope(text: &str, bytes: &[u8]) -> types::Envelope {
        types::Envelope {
            payload: types::Payload {
                text: text.into(),
                bytes: bytes.to_vec().into(),
            },
            marker: Field::from(23_u64),
        }
    }
    fn context() -> CircuitContext<u64> {
        c::initial_state(
            ConstructorContext::new(7),
            "initial".into(),
            vec![1].into(),
            envelope("nested", &[2]),
        )
        .unwrap()
        .into_circuit_context(Default::default())
    }
    #[test]
    fn generated_constructor_preserves_canonical_payloads_and_refuses_invalid_children() {
        for (text, bytes) in [
            ("", vec![]),
            ("a\0b", vec![0, 1, 0, 255]),
            ("stored ✓", vec![0, 1, 255]),
        ] {
            let expected = envelope(text, &bytes);
            let ctx = c::initial_state(
                ConstructorContext::new(7),
                text.into(),
                bytes.clone().into(),
                expected.clone(),
            )
            .unwrap()
            .into_circuit_context(Default::default());
            assert_eq!(
                ledger_slots::text
                    .inspect(ctx.query.state.get_ref())
                    .unwrap(),
                OpaqueString::from(text)
            );
            assert_eq!(
                ledger_slots::blob
                    .inspect(ctx.query.state.get_ref())
                    .unwrap(),
                OpaqueBytes::from(bytes)
            );
            assert_eq!(
                ledger_slots::envelope
                    .inspect(ctx.query.state.get_ref())
                    .unwrap(),
                expected
            );
        }
        let normal = envelope("nested", &[1]);
        for (text, bytes, aggregate) in [
            (
                OpaqueString::from("a\0"),
                OpaqueBytes::from(vec![1]),
                normal.clone(),
            ),
            (
                OpaqueString::from("a"),
                OpaqueBytes::from(vec![0]),
                normal.clone(),
            ),
            (
                OpaqueString::from("a"),
                OpaqueBytes::from(vec![1]),
                envelope("nested\0", &[1]),
            ),
            (
                OpaqueString::from("a"),
                OpaqueBytes::from(vec![1]),
                envelope("nested", &[0, 1, 255, 0]),
            ),
        ] {
            assert_eq!(
                c::initial_state(ConstructorContext::new(7), text, bytes, aggregate)
                    .err()
                    .unwrap(),
                error()
            );
        }
    }
    #[test]
    fn generated_native_setters_reject_invalid_payload_before_encoding() {
        for bytes in [vec![0], vec![0, 1, 255, 0]] {
            assert_eq!(c::setBytes(context(), bytes.into()).err().unwrap(), error());
        }
        for text in ["\0", "a\0"] {
            assert_eq!(c::setText(context(), text.into()).err().unwrap(), error());
        }
        for aggregate in [envelope("bad\0", &[1]), envelope("good", &[1, 0])] {
            assert_eq!(c::setEnvelope(context(), aggregate).err().unwrap(), error());
        }
        let ctx = c::setBytes(context(), vec![0, 1, 0, 255].into())
            .unwrap()
            .context;
        let ctx = c::setText(ctx, "a\0b".into()).unwrap().context;
        let expected = envelope("nested\0ok", &[0, 1, 0, 255]);
        let ctx = c::setEnvelope(ctx, expected.clone()).unwrap().context;
        assert_eq!(
            ledger_slots::blob
                .inspect(ctx.query.state.get_ref())
                .unwrap(),
            OpaqueBytes::from(vec![0, 1, 0, 255])
        );
        assert_eq!(
            ledger_slots::text
                .inspect(ctx.query.state.get_ref())
                .unwrap(),
            OpaqueString::from("a\0b")
        );
        assert_eq!(
            ledger_slots::envelope
                .inspect(ctx.query.state.get_ref())
                .unwrap(),
            expected
        );
    }
    fn reject_at_slot<T: CellValue + Clone>(slot: CellSlot<T>, value: T) {
        assert_eq!(
            slot.clone().write(context(), value.clone()).err().unwrap(),
            error()
        );
        assert_eq!(
            slot.record_write(RecordingFrame::new(context()), value)
                .err()
                .unwrap(),
            error()
        );
    }
    #[test]
    fn recorded_runtime_adapters_reject_raw_direct_and_nested_values() {
        // These direct official-runtime controls also cover setBytes/setEnvelope,
        // whose production source routes remain non-recordable. setText's
        // generated recorded/observed boundary is exercised below.
        reject_at_slot(ledger_slots::text, OpaqueString::from("bad\0"));
        reject_at_slot(ledger_slots::blob, OpaqueBytes::from(vec![0, 1, 255, 0]));
        reject_at_slot(ledger_slots::envelope, envelope("bad\0", &[1]));
        reject_at_slot(ledger_slots::envelope, envelope("valid", &[1, 0]));
        // Valid path syntax with absent ledger slot: canonical input reaches the
        // VM and fails, while invalid input must be refused before querying.
        let missing = CellSlot::<OpaqueBytes>::new(&[7, 0]);
        let native = missing
            .clone()
            .write(context(), vec![1].into())
            .err()
            .unwrap();
        let recorded = missing
            .clone()
            .record_write(RecordingFrame::new(context()), vec![1].into())
            .err()
            .unwrap();
        assert_eq!(native, recorded);
        assert_ne!(native, error());
        reject_at_slot(missing, OpaqueBytes::from(vec![1, 0]));
    }

    #[cfg(feature = "ledger-transaction")]
    #[test]
    fn generated_text_recording_preserves_values_and_observed_input_refusals() {
        use r::transaction::{Observation, ObservedContractState};
        // This fixture tests observed execution, not preparation or proof keys.
        let state = ObservedContractState::new(
            Default::default(),
            r::ledger::ContractState::new(
                context().query.state.get_ref().clone(),
                r::ledger::LedgerHashMap::new(),
                Default::default(),
            ),
            Observation {
                transaction_hash: [0; 32],
                block_hash: [0; 32],
                block_height: 0,
            },
        );
        let original = state.contract().data.get_ref().clone();
        for text in [
            "",
            "a\0b",
            "stored ✓",
            "a canonical message longer than thirty-one bytes",
        ] {
            let value = OpaqueString::from(text);
            let native = c::setText(state.circuit_context(7), value.clone()).unwrap();
            let recorded = c::recorded::setText(state.circuit_context(7), value.clone()).unwrap();
            let manual = ledger_slots::text
                .record_write(RecordingFrame::new(state.circuit_context(7)), value.clone())
                .unwrap()
                .finish(());
            let call = c::recorded::Contract
                .setText_call(&state, 7, value.clone())
                .unwrap();
            for actual in [&recorded, &manual, call.recorded()] {
                assert_eq!(
                    actual.execution.context.query.state,
                    native.context.query.state
                );
                assert_eq!(
                    actual.execution.context.query.effects,
                    native.context.query.effects
                );
                assert_eq!(
                    actual.execution.context.private_state,
                    native.context.private_state
                );
                assert_eq!(
                    actual.execution.private_transcript_outputs,
                    native.private_transcript_outputs
                );
                assert_eq!(actual.execution.gas_cost, native.gas_cost);
                assert_eq!(actual.public.verify_ops(), manual.public.verify_ops());
                assert_eq!(
                    ledger_slots::text
                        .inspect(actual.execution.context.query.state.get_ref())
                        .unwrap(),
                    value
                );
                let replay = actual
                    .public
                    .initial()
                    .query(actual.public.verify_ops(), None, &native.context.cost_model)
                    .unwrap();
                assert_eq!(replay.context.state, native.context.query.state);
                assert_eq!(replay.context.effects, native.context.query.effects);
            }
            assert_eq!(state.contract().data.get_ref(), &original);
        }
        for text in ["\0", "a\0", "é文\0\0"] {
            let value = OpaqueString::from(text);
            assert_eq!(
                c::setText(state.circuit_context(7), value.clone())
                    .err()
                    .unwrap(),
                error()
            );
            assert_eq!(
                c::recorded::setText(state.circuit_context(7), value.clone())
                    .err()
                    .unwrap(),
                error()
            );
            assert_eq!(
                c::recorded::Contract
                    .setText_call(&state, 7, value)
                    .err()
                    .unwrap(),
                error()
            );
            assert_eq!(state.contract().data.get_ref(), &original);
        }
    }
}
