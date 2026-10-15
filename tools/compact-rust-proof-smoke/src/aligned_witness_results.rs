// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
//! Source-bound proof of one canonical nested aligned witness result.
use super::*;
use compact_rust_effect_plan_aligned_witness_results_source_fixture::{
    RETAINED_IR, ledger_contract as c, ledger_slots as slots, runtime as r, types,
};
use std::{cell::RefCell, rc::Rc};
type Event = (&'static str, u64, u128);
type Tuple11 = (
    r::FixedBytes<32>,
    r::FixedBytes<32>,
    r::FixedBytes<32>,
    r::FixedBytes<32>,
    r::FixedBytes<32>,
    r::FixedBytes<32>,
    r::FixedBytes<32>,
    r::FixedBytes<32>,
    r::FixedBytes<32>,
    r::FixedBytes<32>,
    r::FixedBytes<32>,
);
#[derive(Clone, Default)]
struct Witness(Rc<RefCell<Vec<Event>>>);
impl Witness {
    fn take(&self, name: &'static str, private: u64, count: u128) -> u64 {
        self.0.borrow_mut().push((name, private, count));
        private + 1
    }
}
fn unused() -> r::CompactError {
    r::CompactError::InvalidLedgerCell("unexpected proof witness".into())
}
impl c::TryWitnesses<u64> for Witness {
    fn text_value(
        &self,
        _: WitnessContext<'_, u64, c::LedgerView<'_>>,
    ) -> Result<(u64, r::OpaqueString), r::CompactError> {
        Err(unused())
    }
    fn entry_value(
        &self,
        _: WitnessContext<'_, u64, c::LedgerView<'_>>,
    ) -> Result<(u64, types::Entry), r::CompactError> {
        Err(unused())
    }
    fn tuple_value(
        &self,
        _: WitnessContext<'_, u64, c::LedgerView<'_>>,
    ) -> Result<(u64, Tuple11), r::CompactError> {
        Err(unused())
    }
    fn nested_value(
        &self,
        ctx: WitnessContext<'_, u64, c::LedgerView<'_>>,
    ) -> Result<(u64, types::Nested), r::CompactError> {
        Ok((
            self.take(
                "nested_value",
                *ctx.private_state,
                ctx.ledger.calls()?.value(),
            ),
            nested("λ\0key"),
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
fn nested(text: &str) -> types::Nested {
    // Checked official point decoding from frozen TS coordinates; no hash under test.
    let identity = r::fab::Value::from((Field::from(0_u64), Field::from(1_u64)));
    let point = r::fab::Value::from((
        Field::from_le_bytes(&[
            157, 52, 201, 185, 204, 155, 101, 1, 10, 105, 33, 117, 104, 143, 190, 125, 12, 131,
            178, 249, 240, 240, 108, 254, 120, 107, 228, 167, 96, 185, 219, 18,
        ])
        .unwrap(),
        Field::from_le_bytes(&[
            17, 169, 199, 80, 148, 108, 92, 119, 108, 249, 200, 118, 27, 54, 239, 222, 17, 105,
            237, 204, 41, 209, 198, 215, 185, 85, 130, 220, 150, 114, 176, 75,
        ])
        .unwrap(),
    ));
    types::Nested {
        items: r::FixedVector([
            (
                r::OpaqueString(String::new()),
                r::JubjubPoint::try_from(&*identity).unwrap(),
            ),
            (
                r::OpaqueString(text.into()),
                r::JubjubPoint::try_from(&*point).unwrap(),
            ),
        ]),
    }
}
fn known_output() -> types::Hashes {
    // Frozen TS nested-canonical answer, independent of Rust hashing.
    types::Hashes {
        x: Field::from_le_bytes(&[
            184, 216, 253, 7, 222, 241, 1, 226, 36, 234, 54, 216, 143, 166, 91, 168, 105, 236, 4,
            103, 90, 11, 247, 56, 57, 80, 140, 57, 142, 63, 7, 30,
        ])
        .unwrap(),
        y: Field::from_le_bytes(&[
            249, 194, 116, 128, 162, 77, 214, 135, 182, 235, 56, 5, 49, 13, 1, 73, 91, 128, 21,
            204, 107, 77, 33, 126, 179, 62, 69, 34, 23, 16, 33, 27,
        ])
        .unwrap(),
        transient: Field::from_le_bytes(&[
            34, 94, 176, 252, 219, 218, 137, 68, 226, 3, 248, 127, 140, 130, 234, 110, 88, 206,
            175, 168, 6, 104, 218, 100, 186, 85, 128, 202, 87, 1, 179, 81,
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
    let name = "nested";
    let mut rng = StdRng::seed_from_u64(0x0584_0000);
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
    let fallback = nested("");
    let native = c::nested(observed.circuit_context(5), &a, fallback.clone(), true)?;
    let recorded = c::recorded::nested(observed.circuit_context(5), &b, fallback.clone(), true)?;
    let contract = c::Contract::from(w.clone());
    let call = contract
        .recording()
        .nested_call(&observed, 5, fallback.clone(), true)?;
    let expected = known_output();
    let private = vec![
        r::exact_private_output(nested("λ\0key")),
        AlignedValue::from(true),
    ];
    for witness in [&a, &b, &w] {
        if *witness.0.borrow() != vec![("nested_value", 5, 1), ("permit", 6, 1)] {
            return Err("aligned witness callback order/read/private state mismatch".into());
        }
    }
    if native.result != expected
        || native.context.private_state != 7
        || native.private_transcript_outputs != private
    {
        return Err("aligned witness native output/private FAB differs from frozen TS".into());
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
            return Err("aligned witness native/recorded/observed binding differs".into());
        }
    }
    let expected_state = native.context.query.state.get_ref().clone();
    if slots::calls.inspect(&expected_state)? != 1
        || slots::calls.inspect(observed.circuit_context(5).query.state.get_ref())? != 0
    {
        return Err("aligned witness Counter or immutable observation differs".into());
    }
    let verifier: VerifierKey = tagged_deserialize(&mut BufReader::new(File::open(
        root.join("keys/nested.verifier"),
    )?))?;
    let prepared = call.prepare(verifier.clone(), Fr::from(0_u64))?;
    let input = (fallback, true);
    if prepared.input != AlignedValue::from(input.clone())
        || prepared.output != AlignedValue::from(expected)
    {
        return Err("aligned witness prepared original input/product output differs".into());
    }
    // Ordinary shared preparation replays and checks the generated trace;
    // exhaustive manual/replay cases live in the source fixture.
    let direct = check_generated_trace(root, name, recorded, input)?;
    if format!("{direct:?}") != format!("{prepared:?}") {
        return Err("aligned witness direct/observed preparation differs".into());
    }
    kernel_shielded_effects::prove_and_verify_call(root, name, &prepared, &verifier)?;
    opaque_public::apply_funded(root, name, prepared, deploy, &expected_state, rng, 584)?;
    println!(
        "nested(selected=true): frozen TS three-Field result, exact nested private FAB, ordered witnesses, Counter, default-v2 verified proof+changed-binding refusal and default-strict funded ledger8 application; canonical witness only, constructor state supplied, deployment not proved"
    );
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    fn aligned_json(value: &AlignedValue) -> serde_json::Value {
        serde_json::json!({"value": value.value.0.iter().map(|a| &a.0).collect::<Vec<_>>(), "alignment": value.alignment})
    }
    #[test]
    fn mismatched_whole_source_refuses_before_keys() {
        counter_scenario::validate_ir(RETAINED_IR.as_bytes(), RETAINED_IR).unwrap();
        let mut value: serde_json::Value = serde_json::from_str(RETAINED_IR).unwrap();
        value["independent_aligned_witness_guard"] = true.into();
        assert!(
            counter_scenario::validate_ir(&serde_json::to_vec(&value).unwrap(), RETAINED_IR)
                .is_err()
        );
    }
    #[test]
    fn selected_answer_and_nested_private_outputs_match_original_ts_case() {
        let oracle:serde_json::Value=serde_json::from_str(include_str!("../../../tests-rust-backend/effect-plan-aligned-witness-results-source/oracle/typescript.json")).unwrap();
        let rows = oracle["rows"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| r["id"] == "nested-canonical")
            .collect::<Vec<_>>();
        assert_eq!(rows.len(), 1);
        let row = rows[0];
        assert_eq!(row["circuit"], "nested");
        assert_eq!(row["selected"], true);
        assert_eq!(row["permit"], true);
        assert_eq!(row["fail"], serde_json::Value::Null);
        assert_eq!(row["args"], serde_json::json!([row["fallback"], true]));
        assert_eq!(row["payload"]["items"][1][0], "λ\0key");
        assert_eq!(row["serializedPreimage"]["status"], "success");
        assert_eq!(
            row["events"],
            serde_json::json!([["nested_value", 5, "1"], ["permit", 6, "1"]])
        );
        assert_eq!(row["privateState"], 7);
        assert_eq!(
            aligned_json(&AlignedValue::from((nested(""), true))),
            row["proofData"]["input"]
        );
        assert_eq!(
            aligned_json(&AlignedValue::from(known_output())),
            row["proofData"]["output"]
        );
        let private = [
            r::exact_private_output(nested("λ\0key")),
            AlignedValue::from(true),
        ];
        assert_eq!(
            serde_json::Value::Array(private.iter().map(aligned_json).collect()),
            row["proofData"]["privateTranscriptOutputs"]
        );
    }
}
