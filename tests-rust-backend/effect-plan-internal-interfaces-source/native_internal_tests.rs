// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
use crate::{ledger_contract as c, runtime as r, support::*, types};
use r::recording::RecordingFrame;
#[test]
fn direct_native_internal_interfaces_preserve_all_twelve_members_and_owned_aliases() {
    for bytes in [vec![], vec![1, 0, 255], vec![9, 0]] {
        let original = tuple(&bytes);
        let witness = Witness::default();
        let touched = c::touch(context(), &witness, original.clone()).unwrap();
        assert_eq!(touched.result, original);
        assert_eq!(*witness.events.borrow(), vec![(5, 1)]);
        let manual_witness = Witness::default();
        let manual = manual_step(RecordingFrame::new(context()), &manual_witness)
            .unwrap()
            .finish(original.clone());
        assert_eq!(
            touched.context.query.state,
            manual.execution.context.query.state
        );
        assert_eq!(
            touched.context.query.effects,
            manual.execution.context.query.effects
        );
        assert_eq!(touched.gas_cost, manual.execution.gas_cost);
        assert_eq!(
            touched.private_transcript_outputs,
            manual.execution.private_transcript_outputs
        );
        let mut copied = touched.result.clone();
        copied.0.0.push(42);
        assert_eq!(touched.result.0.0, bytes);
        assert_ne!(copied.0, touched.result.0);
        let projected = c::project(touched.context, &witness, touched.result).unwrap();
        assert_eq!(
            projected.result,
            types::Piece {
                bytes: bytes.into(),
                value: r::Field::from(11_u64)
            }
        );
        assert_eq!(*witness.events.borrow(), vec![(5, 1), (6, 2)]);
        assert_eq!(projected.context.private_state, 7);
        assert_eq!(original.11, r::Field::from(11_u64));
    }
}
#[test]
fn direct_native_internal_callback_failure_retains_its_prefix() {
    for project in [false, true] {
        let witness = Witness::new(Some(1));
        let error = if project {
            c::project(context(), &witness, tuple(&[1])).err().unwrap()
        } else {
            c::touch(context(), &witness, tuple(&[1])).err().unwrap()
        };
        assert_eq!(error, failure());
        assert_eq!(*witness.events.borrow(), vec![(5, 1)]);
    }
}
