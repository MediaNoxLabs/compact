// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

use super::*;
use compact_rust_guarded_point_nested_source_fixture::{
    ledger_contract as nested, ledger_slots as slots,
};

pub(crate) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    let actual: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("contract/compact-rust-ir.json"))?)?;
    let expected: serde_json::Value =
        serde_json::from_str(compact_rust_guarded_point_nested_source_fixture::RETAINED_IR)?;
    if actual != expected {
        return Err("nested point proof source IR mismatch".into());
    }
    for (index, (outer, inner)) in [(false, false), (false, true), (true, false), (true, true)]
        .into_iter()
        .enumerate()
    {
        let mut rng = StdRng::seed_from_u64(0x0518_0000 + index as u64);
        let initial = nested::initial_state(ConstructorContext::new(7_u64))?;
        let deploy = make_deploy(
            root,
            "nested",
            initial.ledger_state.get_ref().clone(),
            &mut rng,
        )?;
        let observed = ObservedContractState::new(
            deploy.address(),
            deploy.initial_state.clone(),
            Observation {
                transaction_hash: [0; 32],
                block_hash: [0; 32],
                block_height: 0,
            },
        );
        let point = r::ec_mul_generator(Field::from(7_u64))?;
        let expected = if outer {
            if inner {
                r::ec_add(point, r::ec_mul_generator(Field::from(1_u64))?)
            } else {
                r::ec_mul(point, Field::from(2_u64))?
            }
        } else {
            point
        };
        let native = nested::nested(observed.circuit_context(7_u64), outer, inner, point)?;
        let recorded =
            nested::recorded::nested(observed.circuit_context(7_u64), outer, inner, point)?;
        if native.result != expected
            || recorded.execution.result != native.result
            || recorded.execution.context.query.state != native.context.query.state
            || recorded.execution.context.query.effects != native.context.query.effects
            || recorded.execution.gas_cost != native.gas_cost
            || recorded.execution.context.private_state != 7
            || !recorded.execution.private_transcript_outputs.is_empty()
        {
            return Err("nested guard source native/recorded mismatch".into());
        }
        let expected_state = native.context.query.state.get_ref().clone();
        if slots::first.inspect(&expected_state)? != point
            || slots::second.inspect(&expected_state)? != expected
        {
            return Err("nested guard state semantics changed".into());
        }
        let input = AlignedValue::concat(&[
            AlignedValue::from(outer),
            AlignedValue::from(inner),
            AlignedValue::from(point),
        ]);
        let manual = check_generated_trace(root, "nested", recorded, input)?;
        let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
            root.join("keys/nested.verifier"),
        )?))?;
        let prepared = nested::Contract::default()
            .recording
            .nested_call(&observed, 7_u64, outer, inner, point)?
            .prepare(verifier, Fr::from(0_u64))?;
        if format!("{manual:?}") != format!("{prepared:?}") {
            return Err("nested observed/direct call mismatch".into());
        }
        check_source_preimage(root, "nested", &prepared, Some(outer))?;
        check_transaction(root, "nested", deploy, prepared, &mut rng, |state| {
            if state.data.get_ref() != &expected_state {
                return Err("nested applied state mismatch".into());
            }
            Ok(())
        })?;
        println!(
            "nested outer={outer} inner={inner}: checked, proved, verified and ledger-applied"
        );
    }
    Ok(())
}
