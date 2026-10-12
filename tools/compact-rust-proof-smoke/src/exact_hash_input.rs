// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Source-bound proof of exact opaque hash input through one canonical witness.
use super::*;
use compact_rust_effect_plan_exact_hash_input_source_fixture::{
    RETAINED_IR, ledger_contract as c, ledger_slots as slots, runtime as r, types,
};
use std::{cell::RefCell, rc::Rc};
type Event = (&'static str, u64, u128);
#[derive(Clone, Default)]
struct Witness(Rc<RefCell<Vec<Event>>>);
impl Witness {
    fn take(&self, name: &'static str, private: u64, count: u128) -> u64 {
        self.0.borrow_mut().push((name, private, count));
        private + 1
    }
}
impl c::TryWitnesses<u64> for Witness {
    fn bytes(
        &self,
        ctx: WitnessContext<'_, u64, c::LedgerView<'_>>,
    ) -> Result<(u64, r::OpaqueBytes), r::CompactError> {
        Ok((
            self.take("bytes", *ctx.private_state, ctx.ledger.calls()?.value()),
            r::OpaqueBytes(vec![1, 0, 2]),
        ))
    }
    fn permit(
        &self,
        ctx: WitnessContext<'_, u64, c::LedgerView<'_>>,
    ) -> Result<(u64, bool), r::CompactError> {
        Ok((
            self.take("permit", *ctx.private_state, ctx.ledger.calls()?.value()),
            true,
        ))
    }
}
fn known_output() -> types::Hashes {
    // Frozen original TS witness-interior answer, independent of Rust hashing.
    types::Hashes {
        x: Field::from_le_bytes(&[
            26, 79, 76, 166, 65, 105, 249, 222, 191, 181, 5, 231, 56, 168, 193, 188, 229, 89, 8,
            183, 151, 243, 74, 239, 47, 17, 119, 127, 174, 74, 89, 102,
        ])
        .unwrap(),
        y: Field::from_le_bytes(&[
            102, 75, 200, 190, 51, 158, 3, 198, 22, 55, 115, 191, 7, 7, 122, 83, 133, 135, 73, 147,
            106, 122, 249, 239, 203, 104, 195, 109, 63, 205, 105, 92,
        ])
        .unwrap(),
        transient: Field::from_le_bytes(&[
            195, 29, 79, 106, 211, 53, 19, 75, 132, 150, 177, 162, 13, 159, 112, 179, 231, 252, 64,
            77, 170, 51, 140, 103, 183, 200, 35, 125, 92, 114, 25, 114,
        ])
        .unwrap(),
        committed: Field::from_le_bytes(&[
            19, 53, 19, 129, 29, 78, 85, 144, 128, 30, 173, 48, 18, 96, 172, 51, 181, 4, 94, 211,
            49, 253, 209, 2, 195, 63, 208, 66, 229, 126, 238, 54,
        ])
        .unwrap(),
    }
}
pub(super) fn run(root: &Path) -> Result<(), Box<dyn Error>> {
    // Whole-source identity is checked before make_deploy can load proof keys.
    counter_scenario::validate_ir(
        &std::fs::read(root.join("contract/compact-rust-ir.json"))?,
        RETAINED_IR,
    )?;
    let name = "witnessed";
    let mut rng = StdRng::seed_from_u64(0x0583_0000);
    let initial = c::initial_state(ConstructorContext::new(5_u64))?;
    let deploy = make_deploy(root, name, initial.ledger_state.get_ref().clone(), &mut rng)?;
    let observed = ObservedContractState::new(
        deploy.address(),
        deploy.initial_state.clone(),
        Observation {
            transaction_hash: [0; 32],
            block_hash: [0; 32],
            block_height: 0,
        },
    );
    let a = Witness::default();
    let b = Witness::default();
    let w = Witness::default();
    let fallback = r::OpaqueBytes(vec![]);
    let native = c::witnessed(observed.circuit_context(5), &a, fallback.clone(), true)?;
    let recorded = c::recorded::witnessed(observed.circuit_context(5), &b, fallback.clone(), true)?;
    let contract = c::Contract::from(w.clone());
    let call = contract
        .recording()
        .witnessed_call(&observed, 5, fallback.clone(), true)?;
    let expected = known_output();
    let private = vec![
        r::exact_private_output(r::OpaqueBytes(vec![1, 0, 2])),
        AlignedValue::from(true),
    ];
    for witness in [&a, &b, &w] {
        if *witness.0.borrow() != vec![("bytes", 5, 1), ("permit", 6, 1)] {
            return Err("exact hash callback order/read/private state mismatch".into());
        }
    }
    if native.result != expected
        || native.context.private_state != 7
        || native.private_transcript_outputs != private
    {
        return Err("exact hash native output/private FAB differs from frozen TS".into());
    }
    for candidate in [&recorded, call.recorded()] {
        if candidate.execution.result != expected
            || candidate.execution.context.private_state != 7
            || candidate.execution.private_transcript_outputs != private
            || candidate.execution.context.query.state != native.context.query.state
            || candidate.execution.context.query.effects != native.context.query.effects
            || candidate.execution.gas_cost != native.gas_cost
            || candidate.public.verify_ops() != recorded.public.verify_ops()
            || candidate.public.verify_ops().is_empty()
        {
            return Err("exact hash native/recorded/observed binding differs".into());
        }
    }
    let expected_state = native.context.query.state.get_ref().clone();
    if slots::calls.inspect(&expected_state)? != 1
        || slots::calls.inspect(observed.circuit_context(5).query.state.get_ref())? != 0
    {
        return Err("exact hash Counter or immutable observation differs".into());
    }
    let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
        root.join("keys/witnessed.verifier"),
    )?))?;
    let prepared = call.prepare(verifier.clone(), Fr::from(0_u64))?;
    let input = (fallback, true);
    if prepared.input != AlignedValue::from(input.clone())
        || prepared.output != AlignedValue::from(expected)
    {
        return Err("exact hash prepared original input/product output differs".into());
    }
    // Ordinary shared preparation replays and checks the generated trace;
    // exhaustive manual/replay cases live in the source fixture.
    let direct = check_generated_trace(root, name, recorded, input)?;
    if format!("{direct:?}") != format!("{prepared:?}") {
        return Err("exact hash direct/observed preparation differs".into());
    }
    kernel_shielded_effects::prove_and_verify_call(root, name, &prepared, &verifier)?;
    opaque_public::apply_funded(root, name, prepared, deploy, &expected_state, rng, 583)?;
    println!(
        "witnessed(selected=true, bytes=[1,0,2], fallback=[]): frozen TS four-Field result, exact private FAB, ordered witnesses, Counter, default-v2 verified proof+changed-binding refusal and default-strict funded ledger8 application; constructor state supplied, deployment not proved"
    );
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mismatched_whole_source_refuses_before_keys() {
        counter_scenario::validate_ir(RETAINED_IR.as_bytes(), RETAINED_IR).unwrap();
        let mut value: serde_json::Value = serde_json::from_str(RETAINED_IR).unwrap();
        value["independent_exact_hash_guard"] = true.into();
        assert!(
            counter_scenario::validate_ir(&serde_json::to_vec(&value).unwrap(), RETAINED_IR)
                .is_err()
        );
    }
    #[test]
    fn selected_answer_and_private_outputs_match_original_ts_case() {
        let oracle: serde_json::Value = serde_json::from_str(include_str!(
            "../../../tests-rust-backend/effect-plan-exact-hash-input-source/oracle/typescript.json"
        ))
        .unwrap();
        let rows = oracle["rows"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| r["id"] == "witness-interior")
            .collect::<Vec<_>>();
        assert_eq!(rows.len(), 1);
        let row = rows[0];
        assert_eq!(row["circuit"], "witnessed");
        assert_eq!(row["args"], serde_json::json!([[], true]));
        assert_eq!(row["payload"], serde_json::json!([1, 0, 2]));
        assert_eq!(row["permit"], true);
        assert_eq!(row["fail"], serde_json::Value::Null);
        assert_eq!(row["serializedPreimage"]["status"], "success");
        let expected = known_output();
        for (name, actual) in [
            ("x", expected.x),
            ("y", expected.y),
            ("transient", expected.transient),
            ("committed", expected.committed),
        ] {
            let decimal = row["result"][name]
                .as_str()
                .unwrap()
                .bytes()
                .fold(Field::from(0_u64), |n, d| {
                    n * Field::from(10_u64) + Field::from((d - b'0') as u64)
                });
            assert_eq!(actual, decimal, "{name}");
        }
        let private = [
            r::exact_private_output(r::OpaqueBytes(vec![1, 0, 2])),
            AlignedValue::from(true),
        ];
        let captured = row["proofData"]["privateTranscriptOutputs"]
            .as_array()
            .unwrap();
        assert_eq!(captured.len(), private.len());
        for (actual, captured) in private.iter().zip(captured) {
            assert_eq!(
                serde_json::json!(actual.value.0.iter().map(|a| &a.0).collect::<Vec<_>>()),
                captured["value"]
            );
            assert_eq!(
                serde_json::to_value(&actual.alignment).unwrap(),
                captured["alignment"]
            );
        }
    }
}
