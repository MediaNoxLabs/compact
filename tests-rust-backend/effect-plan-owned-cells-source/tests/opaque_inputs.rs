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
        // These call official runtime adapters directly. Source setters above
        // intentionally remain non-recordable under the existing interface policy.
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
}
